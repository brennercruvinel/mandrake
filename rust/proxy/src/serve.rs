//! The HTTP side: accept on loopback, trim, forward, stream the answer back, record one event.

use crate::log::RotatingLog;
use crate::usage::Meter;
use crate::{audit, clock, count, trim};
use anyhow::{Context as _, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::{Body, Frame, Incoming};
use hyper::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::{TokioIo, TokioTimer};
use mandrake_home::{Dirs, Profile};
use mandrake_meter::{Event, Mode};
use std::convert::Infallible;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Request headers that describe this hop, not the request: never forwarded.
/// `GET` here answers `mandrake` without going upstream, so the launcher and doctor can tell this
/// proxy from another program listening on the same port.
pub const IDENTITY: &str = "/_mandrake";

const HOP: [&str; 10] = [
    "host",
    "content-length",
    "accept-encoding",
    "connection",
    "transfer-encoding",
    "keep-alive",
    "proxy-connection",
    "te",
    "trailer",
    "upgrade",
];
/// Response headers hyper sets again for the client side.
const BACK: [&str; 4] = [
    "content-length",
    "transfer-encoding",
    "connection",
    "keep-alive",
];

pub type ProxyBody = BoxBody<Bytes, io::Error>;

/// Everything a request needs, resolved once at start.
#[derive(Debug, Clone)]
pub struct Config {
    pub upstream: String,
    pub mode: Mode,
    pub strip: Vec<String>,
    pub rules: trim::Rules,
    pub audit: bool,
    pub count_tokens: bool,
    pub events: PathBuf,
    pub audit_dir: PathBuf,
    pub log: PathBuf,
}

impl Config {
    pub fn new(dirs: &Dirs, profile: &Profile, mode: Mode) -> Self {
        let p = &profile.proxy;
        Self {
            upstream: p.upstream.trim_end_matches('/').to_string(),
            mode,
            strip: p.strip_fields.clone(),
            rules: trim::rules_for(p.cut_identity, p.unwrap_reminders),
            audit: p.audit,
            count_tokens: p.count_tokens,
            events: dirs.events(),
            audit_dir: dirs.audit(),
            log: dirs.logs().join("proxy.log"),
        }
    }
}

pub struct State {
    pub cfg: Config,
    pub client: reqwest::Client,
    pub log: RotatingLog,
    events: Mutex<()>,
}

impl State {
    pub fn new(cfg: Config) -> Result<Self> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .build()
            .context("building the HTTP client")?;
        let log = RotatingLog::new(cfg.log.clone());
        Ok(Self {
            cfg,
            client,
            log,
            events: Mutex::new(()),
        })
    }

    /// One line in `events.jsonl` and one in `proxy.log`.
    fn record(&self, method: &str, e: &Event) {
        let written = {
            let _guard = self.events.lock().unwrap_or_else(|p| p.into_inner());
            mandrake_meter::append(&self.cfg.events, e)
        };
        self.log.line(&log_line(method, e));
        if let Err(err) = written {
            self.log
                .line(&format!("{} {} events error={err:#}", clock::now(), e.id));
        }
    }
}

/// Serves until killed, on its own runtime.
pub fn serve(state: State, port: u16) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("starting the runtime")?;
    rt.block_on(async move {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("binding 127.0.0.1:{port}"))?;
        let line = format!(
            "mandrake proxy on http://127.0.0.1:{port} -> {} ({})",
            state.cfg.upstream,
            mode_name(state.cfg.mode)
        );
        state.log.line(&format!("{} {line}", clock::now()));
        eprintln!("{line}");
        run(listener, Arc::new(state)).await
    })
}

/// Accepts connections forever.
pub async fn run(listener: TcpListener, state: Arc<State>) -> Result<()> {
    loop {
        let stream = match listener.accept().await {
            Ok((s, _)) => s,
            Err(e) => {
                // Out of descriptors and the like: wait a moment instead of spinning.
                state
                    .log
                    .line(&format!("{} accept error={e}", clock::now()));
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }
        };
        let state = state.clone();
        tokio::spawn(async move {
            let svc = service_fn(move |req| handle(state.clone(), req));
            let _ = http1::Builder::new()
                .timer(TokioTimer::new())
                .serve_connection(TokioIo::new(stream), svc)
                .await;
        });
    }
}

async fn handle(
    state: Arc<State>,
    req: Request<Incoming>,
) -> Result<Response<ProxyBody>, Infallible> {
    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let path = parts.uri.path().to_string();
    if method == Method::GET && path == IDENTITY {
        let body = Full::new(Bytes::from_static(b"mandrake\n"))
            .map_err(|never| match never {})
            .boxed();
        return Ok(Response::new(body));
    }
    let mut rec = Record::new(state.clone(), &method, &path);
    let before = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            rec.event.status = 400;
            rec.event.error = Some(format!("reading request: {}", chain(&e)));
            tokio::spawn(rec.finish());
            return Ok(error_response(
                StatusCode::BAD_REQUEST,
                "could not read the request body",
            ));
        }
    };
    let cfg = &state.cfg;
    let (after, cuts) = trim::prepare(
        cfg.mode,
        method.as_str(),
        &path,
        &before,
        &cfg.rules,
        &cfg.strip,
    );
    let after = Bytes::from(after);
    rec.event.model = model_of(&before);
    rec.event.bytes_before = before.len() as u64;
    rec.event.bytes_after = after.len() as u64;
    rec.event.cuts = cuts;
    if cfg.audit && trim::is_messages(method.as_str(), &path) {
        match audit::write(&cfg.audit_dir, &rec.event.id, &before, &after) {
            Ok(()) => rec.event.audited = true,
            Err(e) => state.log.line(&format!(
                "{} {} audit error={e:#}",
                clock::now(),
                rec.event.id
            )),
        }
    }
    if cfg.count_tokens && method == Method::POST && path == "/v1/messages" {
        let (st, auth, b, a) = (
            state.clone(),
            count::auth_headers(&parts.headers),
            before.clone(),
            after.clone(),
        );
        rec.counts = Some(tokio::spawn(async move {
            count::pair(&st.client, &st.cfg.upstream, &auth, &b, &a).await
        }));
    }

    let target = parts
        .uri
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");
    let mut up = state
        .client
        .request(method.clone(), format!("{}{target}", cfg.upstream))
        .headers(forward_headers(&parts.headers));
    if !after.is_empty() || !matches!(method, Method::GET | Method::HEAD) {
        up = up.body(after);
    }
    // Dropping this future (client gone) drops the upstream request with it.
    let resp = match up.send().await {
        Ok(r) => r,
        Err(e) => {
            let why = chain(&e.without_url());
            rec.event.status = 502;
            rec.event.error = Some(format!("upstream: {why}"));
            tokio::spawn(rec.finish());
            return Ok(error_response(
                StatusCode::BAD_GATEWAY,
                &format!("mandrake proxy could not reach the API: {why}"),
            ));
        }
    };

    rec.event.status = resp.status().as_u16();
    let meter = Meter::for_content_type(
        resp.headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
    );
    let mut headers = HeaderMap::new();
    for (k, v) in resp.headers() {
        if !BACK.contains(&k.as_str()) {
            headers.append(k, v.clone());
        }
    }
    let status = resp.status();
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(pump(resp, tx, meter, rec));
    let mut out = Response::new(ChanBody(rx).boxed());
    *out.status_mut() = status;
    *out.headers_mut() = headers;
    Ok(out)
}

/// Copies the upstream body to the client chunk by chunk, reading usage on the way.
async fn pump(
    mut resp: reqwest::Response,
    tx: mpsc::Sender<io::Result<Bytes>>,
    mut meter: Meter,
    mut rec: Record,
) {
    loop {
        tokio::select! {
            _ = tx.closed() => {
                rec.event.error = Some("client disconnected".into());
                break;
            }
            chunk = resp.chunk() => match chunk {
                Ok(Some(b)) => {
                    meter.feed(&b);
                    if tx.send(Ok(b)).await.is_err() {
                        rec.event.error = Some("client disconnected".into());
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    let why = chain(&e.without_url());
                    rec.event.error = Some(format!("upstream stream: {why}"));
                    // An error frame makes hyper cut the connection: no JSON inside a started stream.
                    let _ = tx.send(Err(io::Error::other(why))).await;
                    break;
                }
            }
        }
    }
    drop(resp);
    drop(tx);
    rec.event.usage = meter.finish();
    rec.finish().await;
}

/// The event of one request, written exactly once: by `finish`, or on drop if the request was abandoned.
struct Record {
    state: Arc<State>,
    method: String,
    event: Event,
    started: Instant,
    ended: bool,
    counts: Option<JoinHandle<(Option<u64>, Option<u64>)>>,
    written: bool,
}

impl Record {
    fn new(state: Arc<State>, method: &Method, path: &str) -> Self {
        let event = Event {
            id: clock::new_id(),
            ts: clock::now(),
            mode: state.cfg.mode,
            path: path.to_string(),
            model: None,
            status: 0,
            ms: 0,
            bytes_before: 0,
            bytes_after: 0,
            tokens_before: None,
            tokens_after: None,
            usage: None,
            cuts: Default::default(),
            audited: false,
            error: None,
        };
        let method = method.to_string();
        Self {
            state,
            method,
            event,
            started: Instant::now(),
            ended: false,
            counts: None,
            written: false,
        }
    }

    fn stop_clock(&mut self) {
        if !self.ended {
            self.event.ms = self.started.elapsed().as_millis() as u64;
            self.ended = true;
        }
    }

    /// Stops the clock at the end of the response, then waits up to 30 s for the counts.
    async fn finish(mut self) {
        self.stop_clock();
        if let Some(mut h) = self.counts.take() {
            match tokio::time::timeout(count::TIMEOUT, &mut h).await {
                Ok(Ok((b, a))) => (self.event.tokens_before, self.event.tokens_after) = (b, a),
                _ => h.abort(),
            }
        }
        self.write();
    }

    fn write(&mut self) {
        self.written = true;
        self.state.record(&self.method, &self.event);
    }
}

impl Drop for Record {
    fn drop(&mut self) {
        if self.written {
            return;
        }
        if let Some(h) = self.counts.take() {
            h.abort();
        }
        self.stop_clock();
        if self.event.error.is_none() {
            self.event.error = Some("client disconnected".into());
        }
        self.write();
    }
}

/// The client side of the response body: whatever `pump` sends.
pub struct ChanBody(pub mpsc::Receiver<io::Result<Bytes>>);

impl Body for ChanBody {
    type Data = Bytes;
    type Error = io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<io::Result<Frame<Bytes>>>> {
        self.0.poll_recv(cx).map(|o| o.map(|r| r.map(Frame::data)))
    }
}

fn forward_headers(from: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::with_capacity(from.len());
    for (k, v) in from {
        if !HOP.contains(&k.as_str()) {
            out.append(k, v.clone());
        }
    }
    out
}

pub fn error_response(status: StatusCode, message: &str) -> Response<ProxyBody> {
    let body =
        serde_json::json!({"type": "error", "error": {"type": "proxy_error", "message": message}});
    let body = Full::new(Bytes::from(body.to_string()))
        .map_err(|never| match never {})
        .boxed();
    let mut out = Response::new(body);
    *out.status_mut() = status;
    out.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    out
}

fn model_of(body: &[u8]) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct M {
        model: Option<String>,
    }
    serde_json::from_slice::<M>(body).ok()?.model
}

/// An error and its causes on one line: reqwest's own message alone says little.
fn chain(e: &(dyn std::error::Error + 'static)) -> String {
    let mut s = e.to_string();
    let mut src = e.source();
    while let Some(c) = src {
        s.push_str(": ");
        s.push_str(&c.to_string());
        src = c.source();
    }
    s
}

pub fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Trim => "trim",
        Mode::Pass => "pass",
    }
}

fn log_line(method: &str, e: &Event) -> String {
    let opt = |n: Option<u64>| n.map_or_else(|| "?".to_string(), |n| n.to_string());
    let tokens = match (e.tokens_before, e.tokens_after) {
        (None, None) => "-".to_string(),
        (b, a) => format!("{}->{}", opt(b), opt(a)),
    };
    let mut line = format!(
        "{} {} {method} {} mode={} status={} ms={} bytes={}->{} tokens={tokens} cuts=system:{},reminders:{},fields:{}",
        e.ts,
        e.id,
        e.path,
        mode_name(e.mode),
        e.status,
        e.ms,
        e.bytes_before,
        e.bytes_after,
        e.cuts.system_blocks,
        e.cuts.reminders,
        e.cuts.fields.len(),
    );
    if let Some(err) = &e.error {
        line.push_str(&format!(" error={err:?}"));
    }
    line
}
