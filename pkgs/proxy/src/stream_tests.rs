//! End to end on loopback: a fake upstream (hyper), the proxy, and a client. No real API.

use crate::serve::{self, ChanBody, Config, ProxyBody, State};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use mandrake_home::{Dirs, Profile};
use mandrake_meter::{Event, Mode, Usage};
use serde_json::{Value, json};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

const SSE: [&str; 4] = [
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude-sonnet-4-5\",\"usage\":{\"input_tokens\":12,\"cache_creation_input_tokens\":3,\"cache_read_input_tokens\":40,\"output_tokens\":1}}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
    "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n",
    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
];

#[derive(Default)]
struct Seen {
    /// (path and query, body) of every request the fake upstream got.
    requests: Mutex<Vec<(String, Vec<u8>)>>,
    /// Set when a slow stream noticed its reader went away.
    aborted: AtomicBool,
}

impl Seen {
    fn bodies(&self, path: &str) -> Vec<Vec<u8>> {
        let r = self.requests.lock().unwrap();
        r.iter()
            .filter(|(p, _)| p == path)
            .map(|(_, b)| b.clone())
            .collect()
    }
}

async fn fake_upstream() -> (String, Arc<Seen>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(Seen::default());
    let s = seen.clone();
    tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            let s = s.clone();
            tokio::spawn(async move {
                let svc = service_fn(move |req| upstream_handle(s.clone(), req));
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), svc)
                    .await;
            });
        }
    });
    (format!("http://{addr}"), seen)
}

async fn upstream_handle(
    seen: Arc<Seen>,
    req: Request<Incoming>,
) -> Result<Response<ProxyBody>, Infallible> {
    let pq = req.uri().path_and_query().unwrap().as_str().to_string();
    let path = req.uri().path().to_string();
    let slow = req.uri().query().is_some_and(|q| q.contains("slow"));
    let body = req.into_body().collect().await.unwrap().to_bytes().to_vec();
    let len = body.len();
    seen.requests.lock().unwrap().push((pq, body));
    if path == "/v1/messages/count_tokens" {
        let text = json!({"input_tokens": len}).to_string();
        let mut r = Response::new(Full::new(Bytes::from(text)).map_err(|n| match n {}).boxed());
        r.headers_mut()
            .insert("content-type", "application/json".parse().unwrap());
        return Ok(r);
    }
    let (tx, rx) = mpsc::channel(1);
    tokio::spawn(async move {
        for part in SSE.iter().take(if slow { 1 } else { SSE.len() }) {
            tokio::time::sleep(Duration::from_millis(5)).await;
            if tx
                .send(Ok(Bytes::from_static(part.as_bytes())))
                .await
                .is_err()
            {
                return;
            }
        }
        if !slow {
            return;
        }
        loop {
            tokio::time::sleep(Duration::from_millis(20)).await;
            if tx
                .send(Ok(Bytes::from_static(b": ping\n\n")))
                .await
                .is_err()
            {
                seen.aborted.store(true, Ordering::SeqCst);
                return;
            }
        }
    });
    let mut r = Response::new(ChanBody(rx).boxed());
    r.headers_mut()
        .insert("content-type", "text/event-stream".parse().unwrap());
    r.headers_mut()
        .insert("request-id", "req_fake".parse().unwrap());
    Ok(r)
}

fn dirs(tmp: &Path) -> Dirs {
    let tmp = tmp.to_path_buf();
    Dirs::resolve(move |k| match k {
        "HOME" => Some(tmp.join("home")),
        "MANDRAKE_HOME" => Some(tmp.join("mh")),
        _ => None,
    })
    .unwrap()
}

async fn start_proxy(upstream: &str, mode: Mode, tmp: &Path) -> (String, Dirs) {
    let dirs = dirs(tmp);
    let mut profile = Profile::default();
    profile.proxy.upstream = upstream.to_string();
    profile.proxy.audit = true;
    let state = Arc::new(State::new(Config::new(&dirs, &profile, mode)).unwrap());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(serve::run(listener, state));
    (format!("http://{addr}"), dirs)
}

async fn events(path: &Path, n: usize) -> Vec<Event> {
    for _ in 0..250 {
        let all = mandrake_meter::read(path).unwrap();
        if all.len() >= n {
            return all;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("expected {n} event(s) in {}", path.display());
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

fn claude_body() -> Value {
    json!({
        "model": "claude-sonnet-4-5",
        "max_tokens": 1024,
        "stream": true,
        "metadata": {"user_id": "user_0000_account__session_0000"},
        "system": [
            {"type": "text", "text": "You are Claude Code, Anthropic's official CLI for Claude.", "cache_control": {"type": "ephemeral"}},
            {"type": "text", "text": "You are an interactive CLI tool that helps users with software engineering tasks."},
        ],
        "messages": [{"role": "user", "content": [
            {"type": "text", "text": "<system-reminder>\nThe date is 2026-01-01.\n</system-reminder>\nList the files."},
        ]}],
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn trim_streams_sse_and_records_the_event() {
    let tmp = tempfile::tempdir().unwrap();
    let (up, seen) = fake_upstream().await;
    let (proxy, dirs) = start_proxy(&up, Mode::Trim, tmp.path()).await;
    let raw = serde_json::to_vec(&claude_body()).unwrap();
    let resp = client()
        .post(format!("{proxy}/v1/messages?beta=true"))
        .header("x-api-key", "sk-ant-test-0000")
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .body(raw.clone())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()["content-type"], "text/event-stream");
    assert_eq!(resp.headers()["request-id"], "req_fake");
    assert_eq!(resp.text().await.unwrap(), SSE.concat());

    let e = events(&dirs.events(), 1).await.remove(0);
    assert_eq!(
        (e.status, e.mode, e.path.as_str()),
        (200, Mode::Trim, "/v1/messages")
    );
    assert_eq!(e.model.as_deref(), Some("claude-sonnet-4-5"));
    assert_eq!(
        e.usage,
        Some(Usage {
            input: 12,
            output: 7,
            cache_read: 40,
            cache_create: 3
        })
    );
    assert_eq!((e.cuts.system_blocks, e.cuts.reminders), (1, 1));
    assert_eq!(e.cuts.fields, vec!["metadata".to_string()]);
    assert_eq!(e.bytes_before, raw.len() as u64);
    assert!(e.bytes_after < e.bytes_before);
    let (b, a) = (e.tokens_before.unwrap(), e.tokens_after.unwrap());
    assert!(a < b, "{a} < {b}");
    assert!(e.error.is_none(), "{:?}", e.error);
    assert!(e.audited);

    let sent = seen.bodies("/v1/messages?beta=true");
    assert_eq!(sent.len(), 1);
    let sent: Value = serde_json::from_slice(&sent[0]).unwrap();
    assert!(sent.get("metadata").is_none());
    assert_eq!(
        sent["messages"][0]["content"][0]["text"],
        "The date is 2026-01-01.\nList the files."
    );
    let counted = seen.bodies("/v1/messages/count_tokens");
    assert_eq!(counted.len(), 2);
    for c in counted {
        let c: Value = serde_json::from_slice(&c).unwrap();
        assert!(
            c.get("max_tokens").is_none()
                && c.get("stream").is_none()
                && c.get("metadata").is_none()
        );
    }

    let before =
        std::fs::read_to_string(dirs.audit().join(format!("{}.before.json", e.id))).unwrap();
    assert!(before.contains("List the files"));
    assert!(
        !before.contains("x-api-key") && !before.contains("sk-ant-test"),
        "audit bodies never carry headers"
    );
    let log = std::fs::read_to_string(dirs.logs().join("proxy.log")).unwrap();
    assert_eq!(log.lines().count(), 1);
    assert!(log.contains("POST /v1/messages mode=trim status=200"));
    assert!(!log.contains("sk-ant") && !log.contains("List the files"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pass_forwards_byte_for_byte_and_still_measures() {
    let tmp = tempfile::tempdir().unwrap();
    let (up, seen) = fake_upstream().await;
    let (proxy, dirs) = start_proxy(&up, Mode::Pass, tmp.path()).await;
    let raw = serde_json::to_vec(&claude_body()).unwrap();
    let text = client()
        .post(format!("{proxy}/v1/messages"))
        .body(raw.clone())
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(text, SSE.concat());
    assert_eq!(seen.bodies("/v1/messages"), vec![raw.clone()]);
    let e = events(&dirs.events(), 1).await.remove(0);
    assert_eq!(e.mode, Mode::Pass);
    assert_eq!(
        (e.bytes_before, e.bytes_after),
        (raw.len() as u64, raw.len() as u64)
    );
    assert_eq!(e.cuts, Default::default());
    assert!(e.usage.is_some());
    assert_eq!(e.tokens_before, e.tokens_after);
    // One count is enough when nothing changed.
    assert_eq!(seen.bodies("/v1/messages/count_tokens").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn unreachable_upstream_is_a_502_with_a_proxy_error() {
    let tmp = tempfile::tempdir().unwrap();
    let closed = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = closed.local_addr().unwrap().port();
    drop(closed);
    let (proxy, dirs) =
        start_proxy(&format!("http://127.0.0.1:{port}"), Mode::Trim, tmp.path()).await;
    let resp = client()
        .post(format!("{proxy}/v1/messages"))
        .body(serde_json::to_vec(&claude_body()).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 502);
    assert_eq!(resp.headers()["content-type"], "application/json");
    let v: Value = serde_json::from_slice(&resp.bytes().await.unwrap()).unwrap();
    assert_eq!(v["type"], "error");
    assert_eq!(v["error"]["type"], "proxy_error");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("could not reach the API")
    );
    let e = events(&dirs.events(), 1).await.remove(0);
    assert_eq!(e.status, 502);
    assert!(e.error.unwrap().starts_with("upstream:"));
    assert_eq!((e.tokens_before, e.tokens_after), (None, None));
}

#[tokio::test(flavor = "multi_thread")]
async fn client_going_away_aborts_the_upstream_stream() {
    let tmp = tempfile::tempdir().unwrap();
    let (up, seen) = fake_upstream().await;
    let (proxy, dirs) = start_proxy(&up, Mode::Trim, tmp.path()).await;
    let mut resp = client()
        .post(format!("{proxy}/v1/messages?slow=1"))
        .body("{}")
        .send()
        .await
        .unwrap();
    let first = resp.chunk().await.unwrap().unwrap();
    assert!(first.starts_with(b"event: message_start"));
    drop(resp);
    for _ in 0..250 {
        if seen.aborted.load(Ordering::SeqCst) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        seen.aborted.load(Ordering::SeqCst),
        "upstream kept streaming after the client left"
    );
    let e = events(&dirs.events(), 1).await.remove(0);
    assert_eq!(e.error.as_deref(), Some("client disconnected"));
    assert_eq!(e.usage.map(|u| u.input), Some(12));
}
