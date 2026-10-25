//! `count_tokens` for the body before and after the trim, off the response path.

use reqwest::header::{HeaderMap, HeaderValue};
use serde_json::{Map, Value};
use std::time::Duration;

/// The fields `POST /v1/messages/count_tokens` accepts; anything else is a 400.
pub const FIELDS: [&str; 6] = [
    "model",
    "system",
    "messages",
    "tools",
    "tool_choice",
    "thinking",
];

/// Request headers that authenticate and version the call. Nothing else is copied.
pub const AUTH_HEADERS: [&str; 4] = [
    "x-api-key",
    "authorization",
    "anthropic-version",
    "anthropic-beta",
];

pub const TIMEOUT: Duration = Duration::from_secs(30);

/// The `count_tokens` body for a `/v1/messages` body: only the accepted fields, in their original order.
/// `None` when it is not a JSON object with a model and messages.
pub fn count_body(body: &[u8]) -> Option<Vec<u8>> {
    let Value::Object(root) = serde_json::from_slice::<Value>(body).ok()? else {
        return None;
    };
    if !root.contains_key("model") || !root.contains_key("messages") {
        return None;
    }
    let kept: Map<String, Value> = root
        .into_iter()
        .filter(|(k, _)| FIELDS.contains(&k.as_str()))
        .collect();
    serde_json::to_vec(&Value::Object(kept)).ok()
}

pub fn auth_headers(from: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for name in AUTH_HEADERS {
        for v in from.get_all(name) {
            out.append(name, v.clone());
        }
    }
    if !out.contains_key("anthropic-version") {
        out.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    }
    out
}

/// Input tokens for one body, or `None` on any failure.
pub async fn count(
    client: &reqwest::Client,
    upstream: &str,
    auth: &HeaderMap,
    body: &[u8],
) -> Option<u64> {
    let payload = count_body(body)?;
    let url = format!(
        "{}/v1/messages/count_tokens",
        upstream.trim_end_matches('/')
    );
    let resp = client
        .post(url)
        .headers(auth.clone())
        .header("content-type", "application/json")
        .timeout(TIMEOUT)
        .body(payload)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: Value = serde_json::from_slice(&resp.bytes().await.ok()?).ok()?;
    v.get("input_tokens")?.as_u64()
}

/// Counts before and after side by side; an untouched body is counted once.
pub async fn pair(
    client: &reqwest::Client,
    upstream: &str,
    auth: &HeaderMap,
    before: &[u8],
    after: &[u8],
) -> (Option<u64>, Option<u64>) {
    if before == after {
        let n = count(client, upstream, auth, before).await;
        return (n, n);
    }
    tokio::join!(
        count(client, upstream, auth, before),
        count(client, upstream, auth, after)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_accepted_fields_in_order() {
        let body = br#"{"model":"m","max_tokens":9,"stream":true,"system":[],"messages":[],"tools":[],"metadata":{},"thinking":{"type":"enabled","budget_tokens":1024}}"#;
        let out = count_body(body).unwrap();
        assert_eq!(
            out,
            br#"{"model":"m","system":[],"messages":[],"tools":[],"thinking":{"type":"enabled","budget_tokens":1024}}"#
        );
        assert!(count_body(br#"{"messages":[]}"#).is_none());
        assert!(count_body(b"nope").is_none());
    }

    #[test]
    fn copies_auth_headers_only() {
        let mut h = HeaderMap::new();
        h.insert("x-api-key", HeaderValue::from_static("k"));
        h.insert("anthropic-beta", HeaderValue::from_static("b1"));
        h.insert("user-agent", HeaderValue::from_static("claude-cli"));
        h.insert("cookie", HeaderValue::from_static("c"));
        let out = auth_headers(&h);
        assert_eq!(out.len(), 3);
        assert_eq!(out["anthropic-version"], "2023-06-01");
        assert!(out.get("cookie").is_none());
    }
}
