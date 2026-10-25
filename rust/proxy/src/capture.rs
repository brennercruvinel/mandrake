//! A stand-in for the API that answers on loopback and sends nothing anywhere: `mandrake compare`
//! points the proxies at it, so Claude Code's real requests can be kept and compared without
//! spending one. `count_tokens` answers an estimate (bytes / 4); every other POST gets a short
//! answer, streamed when the request asks for a stream. Headers are never read or kept.

use anyhow::{Context as _, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::header::{CONTENT_TYPE, HeaderValue};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use serde_json::{Value, json};
use std::convert::Infallible;
use tokio::net::TcpListener;

/// The text every captured request gets back.
pub const REPLY: &str = "ok (mandrake compare: resposta local, nada foi enviado à API)";

/// Serves on `127.0.0.1:port` until the process ends. Blocks; builds its own runtime.
pub fn serve(port: u16) -> Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("starting the runtime")?;
    rt.block_on(async move {
        let listener = TcpListener::bind(("127.0.0.1", port))
            .await
            .with_context(|| format!("binding 127.0.0.1:{port}"))?;
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            tokio::spawn(async move {
                let _ = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service_fn(answer))
                    .await;
            });
        }
    })
}

async fn answer(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path = req.uri().path().to_string();
    let body = req
        .into_body()
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let (kind, out) = reply(&path, &body);
    let mut res = Response::new(Full::new(Bytes::from(out)));
    res.headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(kind));
    Ok(res)
}

/// The content type and body answering a request to `path` with `body`.
pub fn reply(path: &str, body: &[u8]) -> (&'static str, String) {
    let tokens = body.len() / 4;
    if path.ends_with("/count_tokens") {
        return (
            "application/json",
            json!({ "input_tokens": tokens }).to_string(),
        );
    }
    let req: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let model = req["model"].as_str().unwrap_or("claude").to_string();
    let usage = json!({ "input_tokens": tokens, "output_tokens": 1,
        "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0 });
    let message = json!({ "id": "msg_mandrake_compare", "type": "message", "role": "assistant",
        "model": model, "stop_reason": null, "stop_sequence": null, "usage": usage });
    if req["stream"].as_bool() != Some(true) {
        let mut m = message;
        m["content"] = json!([{ "type": "text", "text": REPLY }]);
        m["stop_reason"] = json!("end_turn");
        return ("application/json", m.to_string());
    }
    let mut start = message;
    start["content"] = json!([]);
    let events = [
        (
            "message_start",
            json!({ "type": "message_start", "message": start }),
        ),
        (
            "content_block_start",
            json!({ "type": "content_block_start", "index": 0,
            "content_block": { "type": "text", "text": "" } }),
        ),
        (
            "content_block_delta",
            json!({ "type": "content_block_delta", "index": 0,
            "delta": { "type": "text_delta", "text": REPLY } }),
        ),
        (
            "content_block_stop",
            json!({ "type": "content_block_stop", "index": 0 }),
        ),
        (
            "message_delta",
            json!({ "type": "message_delta",
            "delta": { "stop_reason": "end_turn", "stop_sequence": null },
            "usage": { "output_tokens": 1 } }),
        ),
        ("message_stop", json!({ "type": "message_stop" })),
    ];
    let sse = events
        .iter()
        .map(|(name, data)| format!("event: {name}\ndata: {data}\n\n"))
        .collect();
    ("text/event-stream", sse)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_tokens_is_an_estimate() {
        let (kind, out) = reply("/v1/messages/count_tokens", &[b'x'; 400]);
        assert_eq!(kind, "application/json");
        assert_eq!(out, r#"{"input_tokens":100}"#);
    }

    #[test]
    fn a_stream_ends_with_message_stop() {
        let (kind, out) = reply("/v1/messages", br#"{"model":"m","stream":true}"#);
        assert_eq!(kind, "text/event-stream");
        assert!(out.starts_with("event: message_start\n"));
        assert!(out.trim_end().ends_with(r#"data: {"type":"message_stop"}"#));
        assert!(out.contains(r#""model":"m""#));
    }

    #[test]
    fn no_stream_gets_one_message() {
        let (kind, out) = reply("/v1/messages", br#"{"model":"m"}"#);
        assert_eq!(kind, "application/json");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["content"][0]["text"], REPLY);
    }
}
