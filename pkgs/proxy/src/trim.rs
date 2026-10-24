//! The cut itself: a pure function from request body to request body, testable without a network.

use mandrake_meter::{Cuts, Mode};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// The rules in `cfgs/proxy/trim.toml`, compiled into the binary.
#[derive(Debug, Clone, Deserialize)]
pub struct Rules {
    /// `system` blocks whose trimmed text equals one of these are dropped.
    pub identities: Vec<String>,
    /// Message text wrapped in `<tag>…</tag>` keeps its content and loses the tag.
    pub unwrap_tag: String,
}

const TRIM_TOML: &str = include_str!("../../../cfgs/proxy/trim.toml");

/// The built-in rules, parsed once.
pub fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| toml::from_str(TRIM_TOML).expect("cfgs/proxy/trim.toml is valid"))
}

/// Whether a request is one the trim may touch: `POST /v1/messages*`.
pub fn is_messages(method: &str, path: &str) -> bool {
    method == "POST" && path.starts_with("/v1/messages")
}

/// What goes upstream for this request: trimmed in `Trim` mode on `POST /v1/messages*`, untouched otherwise.
pub fn prepare(
    mode: Mode,
    method: &str,
    path: &str,
    body: &[u8],
    rules: &Rules,
    strip: &[String],
) -> (Vec<u8>, Cuts) {
    if mode == Mode::Trim && is_messages(method, path) {
        trim(body, rules, strip)
    } else {
        (body.to_vec(), Cuts::default())
    }
}

/// Cuts a `/v1/messages` body. A body that is not a JSON object, or that nothing matched, comes back
/// byte for byte, so an untouched request is never re-encoded.
pub fn trim(body: &[u8], rules: &Rules, strip: &[String]) -> (Vec<u8>, Cuts) {
    let mut cuts = Cuts::default();
    let Ok(Value::Object(mut root)) = serde_json::from_slice::<Value>(body) else {
        return (body.to_vec(), cuts);
    };
    cuts.system_blocks = drop_identities(&mut root, rules);
    if let Some(Value::Array(messages)) = root.get_mut("messages") {
        for m in messages {
            if let Some(content) = m.get_mut("content") {
                cuts.reminders += unwrap_content(content, &rules.unwrap_tag);
            }
        }
    }
    for field in strip {
        if root.shift_remove(field).is_some() {
            cuts.fields.push(field.clone());
        }
    }
    if cuts == Cuts::default() {
        return (body.to_vec(), cuts);
    }
    let out = serde_json::to_vec(&Value::Object(root)).expect("a JSON value always serializes");
    (out, cuts)
}

fn drop_identities(root: &mut Map<String, Value>, rules: &Rules) -> u32 {
    let is_identity = |b: &Value| {
        b.get("type").and_then(Value::as_str) == Some("text")
            && b.get("text")
                .and_then(Value::as_str)
                .is_some_and(|t| rules.identities.iter().any(|i| i == t.trim()))
    };
    let Some(Value::Array(blocks)) = root.get_mut("system") else {
        return 0;
    };
    let n = blocks.len();
    blocks.retain(|b| !is_identity(b));
    let cut = (n - blocks.len()) as u32;
    if cut > 0 && blocks.is_empty() {
        root.shift_remove("system");
    }
    cut
}

/// Unwraps text in a message's `content`: a string, text blocks, and the text inside `tool_result`
/// blocks, where Claude Code also injects reminders.
fn unwrap_content(content: &mut Value, tag: &str) -> u32 {
    match content {
        Value::String(s) => unwrap_string(s, tag),
        Value::Array(blocks) => blocks.iter_mut().map(|b| unwrap_block(b, tag)).sum(),
        _ => 0,
    }
}

fn unwrap_block(block: &mut Value, tag: &str) -> u32 {
    match block.get("type").and_then(Value::as_str) {
        Some("text") => match block.get_mut("text") {
            Some(Value::String(s)) => unwrap_string(s, tag),
            _ => 0,
        },
        Some("tool_result") => match block.get_mut("content") {
            Some(c) => unwrap_content(c, tag),
            None => 0,
        },
        _ => 0,
    }
}

fn unwrap_string(s: &mut String, tag: &str) -> u32 {
    let (out, n) = unwrap_tags(s, tag);
    // An empty text block is a 400 from the API; an empty reminder stays as it came.
    if n > 0 && !out.trim().is_empty() {
        *s = out;
        n
    } else {
        0
    }
}

/// Replaces every complete `<tag>…</tag>` block in `text` by its content, dropping the newline right
/// after the opening tag and right before the closing one. Trailing whitespace after the last block
/// goes too. An opening tag with no closing one is left as it is. Returns the text and the blocks unwrapped.
pub fn unwrap_tags(text: &str, tag: &str) -> (String, u32) {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    let mut n = 0;
    while let Some(start) = rest.find(&open) {
        let after_open = &rest[start + open.len()..];
        let Some(end) = after_open.find(&close) else {
            break;
        };
        out.push_str(&rest[..start]);
        let inner = &after_open[..end];
        let inner = inner.strip_prefix('\n').unwrap_or(inner);
        let inner = inner.strip_suffix('\n').unwrap_or(inner);
        out.push_str(inner);
        n += 1;
        rest = &after_open[end + close.len()..];
        if rest.trim().is_empty() {
            rest = "";
        }
    }
    out.push_str(rest);
    (out, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const ID: &str = "You are Claude Code, Anthropic's official CLI for Claude.";

    fn run(body: Value, strip: &[&str]) -> (Value, Cuts) {
        let strip: Vec<String> = strip.iter().map(|s| s.to_string()).collect();
        let (out, cuts) = trim(&serde_json::to_vec(&body).unwrap(), rules(), &strip);
        (serde_json::from_slice(&out).unwrap(), cuts)
    }

    #[test]
    fn rules_compile_in() {
        assert_eq!(rules().identities.len(), 3);
        assert_eq!(rules().unwrap_tag, "system-reminder");
    }

    #[test]
    fn drops_identity_blocks_only() {
        let body = json!({"system": [
            {"type": "text", "text": format!("  {ID}\n"), "cache_control": {"type": "ephemeral"}},
            {"type": "text", "text": "You are an interactive CLI tool."},
            {"type": "text", "text": format!("{ID} And more.")},
        ]});
        let (out, cuts) = run(body, &[]);
        assert_eq!(cuts.system_blocks, 1);
        let sys = out["system"].as_array().unwrap();
        assert_eq!(sys.len(), 2);
        assert_eq!(sys[0]["text"], "You are an interactive CLI tool.");
    }

    #[test]
    fn system_left_empty_is_removed() {
        let (out, cuts) = run(
            json!({"model": "m", "system": [{"type": "text", "text": ID}]}),
            &[],
        );
        assert_eq!(cuts.system_blocks, 1);
        assert!(out.get("system").is_none());
    }

    #[test]
    fn unwraps_simple_reminder() {
        let (t, n) = unwrap_tags(
            "<system-reminder>\nUse the todo list.\n</system-reminder>\n",
            "system-reminder",
        );
        assert_eq!((t.as_str(), n), ("Use the todo list.", 1));
    }

    #[test]
    fn reminder_followed_by_content_leaves_no_orphan() {
        let text = "<system-reminder>\nctx\n</system-reminder>\nWhat does this do?";
        let (t, n) = unwrap_tags(text, "system-reminder");
        assert_eq!(n, 1);
        assert_eq!(t, "ctx\nWhat does this do?");
        assert!(!t.contains("</system-reminder>"));
    }

    #[test]
    fn unwraps_every_block_in_one_text() {
        let text = "Hi\n<system-reminder>\na\n</system-reminder>\nmid\n<system-reminder>b</system-reminder>\n\n";
        let (t, n) = unwrap_tags(text, "system-reminder");
        assert_eq!(n, 2);
        assert_eq!(t, "Hi\na\nmid\nb");
        assert!(!t.contains("system-reminder"));
    }

    #[test]
    fn unclosed_tag_is_left_alone() {
        let text = "a <system-reminder> b";
        assert_eq!(unwrap_tags(text, "system-reminder"), (text.to_string(), 0));
        let text = "<system-reminder>x</system-reminder> then <system-reminder> open";
        assert_eq!(
            unwrap_tags(text, "system-reminder"),
            ("x then <system-reminder> open".to_string(), 1)
        );
    }

    #[test]
    fn unwraps_in_strings_text_blocks_and_tool_results() {
        let r = "<system-reminder>\nnote\n</system-reminder>";
        let body = json!({"messages": [
            {"role": "user", "content": format!("{r}\nhello")},
            {"role": "user", "content": [
                {"type": "text", "text": r},
                {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AA=="}},
                {"type": "tool_result", "tool_use_id": "t1", "content": format!("file body\n{r}")},
                {"type": "tool_result", "tool_use_id": "t2", "content": [{"type": "text", "text": r}]},
            ]},
        ]});
        let (out, cuts) = run(body, &[]);
        assert_eq!(cuts.reminders, 4);
        assert_eq!(out["messages"][0]["content"], "note\nhello");
        let parts = &out["messages"][1]["content"];
        assert_eq!(parts[0]["text"], "note");
        assert_eq!(parts[2]["content"], "file body\nnote");
        assert_eq!(parts[3]["content"][0]["text"], "note");
    }

    #[test]
    fn empty_reminder_is_kept() {
        let body = json!({"messages": [{"role": "user", "content": [{"type": "text", "text": "<system-reminder>\n</system-reminder>"}]}]});
        let (_, cuts) = run(body, &[]);
        assert_eq!(cuts.reminders, 0);
    }

    #[test]
    fn strips_listed_fields_and_keeps_key_order() {
        let raw = br#"{"model":"m","metadata":{"user_id":"u"},"max_tokens":5,"safeguards":{},"stream":true}"#;
        let (out, cuts) = trim(raw, rules(), &["metadata".into(), "absent".into()]);
        assert_eq!(cuts.fields, vec!["metadata".to_string()]);
        assert_eq!(
            out,
            br#"{"model":"m","max_tokens":5,"safeguards":{},"stream":true}"#
        );
    }

    #[test]
    fn untouched_and_non_json_bodies_come_back_byte_for_byte() {
        let raw = br#"{ "model" : "m", "messages": [] }"#;
        assert_eq!(
            trim(raw, rules(), &["metadata".into()]),
            (raw.to_vec(), Cuts::default())
        );
        assert_eq!(trim(b"not json", rules(), &[]).0, b"not json");
        assert_eq!(trim(b"[1,2]", rules(), &[]).0, b"[1,2]");
    }

    #[test]
    fn pass_mode_and_other_routes_forward_untouched() {
        let raw = br#"{"metadata":{},"system":[{"type":"text","text":"You are a Claude agent, built on Anthropic's Claude Agent SDK."}]}"#;
        let strip = vec!["metadata".to_string()];
        let (out, cuts) = prepare(Mode::Pass, "POST", "/v1/messages", raw, rules(), &strip);
        assert_eq!((out.as_slice(), cuts), (raw.as_slice(), Cuts::default()));
        let (out, _) = prepare(Mode::Trim, "GET", "/v1/messages", raw, rules(), &strip);
        assert_eq!(out, raw);
        let (out, _) = prepare(Mode::Trim, "POST", "/v1/models", raw, rules(), &strip);
        assert_eq!(out, raw);
        let (_, cuts) = prepare(
            Mode::Trim,
            "POST",
            "/v1/messages/count_tokens",
            raw,
            rules(),
            &strip,
        );
        assert_eq!((cuts.system_blocks, cuts.fields.len()), (1, 1));
    }
}
