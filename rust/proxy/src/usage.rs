//! Reads token usage out of a response while it streams past, without holding it.

use mandrake_meter::Usage;
use serde_json::Value;

/// A JSON response bigger than this is not kept for its `usage`.
const JSON_CAP: usize = 8 << 20;

#[derive(Debug)]
enum Kind {
    /// `text/event-stream`: `message_start.message.usage`, then `message_delta.usage`.
    Sse {
        line: Vec<u8>,
    },
    /// A plain JSON body: `usage` at the top.
    Json {
        body: Vec<u8>,
    },
    Other,
}

#[derive(Debug)]
pub struct Meter {
    kind: Kind,
    usage: Option<Usage>,
}

impl Meter {
    pub fn for_content_type(ct: Option<&str>) -> Self {
        let ct = ct.unwrap_or("").to_ascii_lowercase();
        let kind = if ct.starts_with("text/event-stream") {
            Kind::Sse { line: Vec::new() }
        } else if ct.starts_with("application/json") {
            Kind::Json { body: Vec::new() }
        } else {
            Kind::Other
        };
        Self { kind, usage: None }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        match &mut self.kind {
            Kind::Sse { line } => {
                for &b in chunk {
                    if b == b'\n' {
                        let l = std::mem::take(line);
                        sse_line(&l, &mut self.usage);
                    } else {
                        line.push(b);
                    }
                }
            }
            Kind::Json { body } => {
                if body.len() + chunk.len() <= JSON_CAP {
                    body.extend_from_slice(chunk);
                } else {
                    self.kind = Kind::Other;
                }
            }
            Kind::Other => {}
        }
    }

    /// What the response reported, once it ended.
    pub fn finish(mut self) -> Option<Usage> {
        match std::mem::replace(&mut self.kind, Kind::Other) {
            Kind::Sse { line } => sse_line(&line, &mut self.usage),
            Kind::Json { body } => {
                if let Some(u) = serde_json::from_slice::<Value>(&body)
                    .ok()
                    .as_ref()
                    .and_then(|v| v.get("usage"))
                {
                    merge(&mut self.usage, u);
                }
            }
            Kind::Other => {}
        }
        self.usage
    }
}

fn sse_line(line: &[u8], usage: &mut Option<Usage>) {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let Some(data) = line.strip_prefix(b"data:") else {
        return;
    };
    // Most events are text deltas; skip parsing them.
    if !data.windows(7).any(|w| w == b"\"usage\"") {
        return;
    }
    let Ok(v) = serde_json::from_slice::<Value>(data) else {
        return;
    };
    let u = match v.get("type").and_then(Value::as_str) {
        Some("message_start") => v.get("message").and_then(|m| m.get("usage")),
        Some("message_delta") => v.get("usage"),
        _ => None,
    };
    if let Some(u) = u {
        merge(usage, u);
    }
}

/// Later events carry cumulative counts, so a field present overwrites the earlier one.
fn merge(usage: &mut Option<Usage>, u: &Value) {
    let Some(obj) = u.as_object() else {
        return;
    };
    let acc = usage.get_or_insert_with(Usage::default);
    let field = |k: &str| obj.get(k).and_then(Value::as_u64);
    if let Some(n) = field("input_tokens") {
        acc.input = n;
    }
    if let Some(n) = field("output_tokens") {
        acc.output = n;
    }
    if let Some(n) = field("cache_read_input_tokens") {
        acc.cache_read = n;
    }
    if let Some(n) = field("cache_creation_input_tokens") {
        acc.cache_create = n;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SSE: &str = concat!(
        "event: message_start\n",
        "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"usage\":{\"input_tokens\":12,",
        "\"cache_creation_input_tokens\":3,\"cache_read_input_tokens\":40,\"output_tokens\":1}}}\n\n",
        "event: content_block_delta\n",
        "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n",
        "event: message_delta\n",
        "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":7}}\n\n",
        "event: message_stop\n",
        "data: {\"type\":\"message_stop\"}\n\n",
    );

    #[test]
    fn sse_usage_across_split_chunks() {
        let mut m = Meter::for_content_type(Some("text/event-stream; charset=utf-8"));
        for chunk in SSE.as_bytes().chunks(5) {
            m.feed(chunk);
        }
        let u = m.finish().unwrap();
        assert_eq!(
            u,
            Usage {
                input: 12,
                output: 7,
                cache_read: 40,
                cache_create: 3
            }
        );
    }

    #[test]
    fn json_usage_and_none() {
        let mut m = Meter::for_content_type(Some("application/json"));
        m.feed(br#"{"id":"msg","usage":{"input_tokens":5,"output_tokens":2}}"#);
        assert_eq!(
            m.finish(),
            Some(Usage {
                input: 5,
                output: 2,
                ..Default::default()
            })
        );
        let mut m = Meter::for_content_type(Some("application/json"));
        m.feed(br#"{"input_tokens":5}"#);
        assert_eq!(m.finish(), None);
        assert_eq!(Meter::for_content_type(None).finish(), None);
    }
}
