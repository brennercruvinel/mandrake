//! A request body read as the parts a person can recognize: the system prompt, the tools and the
//! conversation, each with its size, so two bodies line up part by part.

use serde_json::Value;

/// The parts of a request, in the order the screen shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    System,
    Tools,
    Messages,
}

impl Part {
    pub const ALL: [Part; 3] = [Part::System, Part::Tools, Part::Messages];

    pub fn label(self) -> &'static str {
        match self {
            Part::System => "system prompt",
            Part::Tools => "ferramentas",
            Part::Messages => "mensagens",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Part::System => "system",
            Part::Tools => "tools",
            Part::Messages => "messages",
        }
    }
}

/// One request body, as text per part.
#[derive(Debug, Clone, Default)]
pub struct Request {
    pub bytes: u64,
    /// Bytes of each part as JSON, in [`Part::ALL`] order.
    pub part_bytes: [u64; 3],
    /// Readable text of each part, in [`Part::ALL`] order.
    pub text: [String; 3],
    pub tools: usize,
}

impl Request {
    pub fn parse(body: &[u8]) -> Self {
        let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let mut r = Request {
            bytes: body.len() as u64,
            tools: v["tools"].as_array().map_or(0, Vec::len),
            ..Request::default()
        };
        for (i, part) in Part::ALL.into_iter().enumerate() {
            let field = &v[part.key()];
            r.part_bytes[i] = if field.is_null() {
                0
            } else {
                field.to_string().len() as u64
            };
            r.text[i] = match part {
                Part::System => system(field),
                Part::Tools => tools(field),
                Part::Messages => messages(field),
            };
        }
        r
    }

    pub fn part(&self, p: Part) -> (u64, &str) {
        let i = Part::ALL.iter().position(|x| *x == p).unwrap_or(0);
        (self.part_bytes[i], &self.text[i])
    }
}

fn system(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => blocks
            .iter()
            .enumerate()
            .map(|(i, b)| {
                format!(
                    "── bloco {} ──\n{}",
                    i + 1,
                    b["text"].as_str().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n"),
        _ => "(nenhum)".into(),
    }
}

fn tools(v: &Value) -> String {
    let Some(list) = v.as_array() else {
        return "(nenhuma)".into();
    };
    list.iter()
        .map(|t| {
            let name = t["name"].as_str().unwrap_or("?");
            let desc = t["description"].as_str().unwrap_or("");
            let schema = t["input_schema"].to_string().len();
            if desc.is_empty() {
                format!("● {name}  (sem descrição; esquema {schema} B)")
            } else {
                format!(
                    "● {name}  (descrição {} B; esquema {schema} B)\n{desc}",
                    desc.len()
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn messages(v: &Value) -> String {
    let Some(list) = v.as_array() else {
        return "(nenhuma)".into();
    };
    list.iter()
        .map(|m| {
            let role = m["role"].as_str().unwrap_or("?");
            let body = match &m["content"] {
                Value::String(s) => s.clone(),
                Value::Array(blocks) => blocks
                    .iter()
                    .map(|b| match b["type"].as_str() {
                        Some("text") => b["text"].as_str().unwrap_or("").to_string(),
                        Some(other) => format!("[{other}]"),
                        None => String::new(),
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
                _ => String::new(),
            };
            format!("── {role} ──\n{body}")
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_and_sizes() {
        let body = br#"{"model":"m","system":[{"type":"text","text":"You are Claude Code."},{"type":"text","text":"Be brief."}],
            "tools":[{"name":"Read","description":"Reads a file.","input_schema":{"type":"object"}},{"name":"Bash","input_schema":{}}],
            "messages":[{"role":"user","content":[{"type":"text","text":"oi"},{"type":"image"}]}]}"#;
        let r = Request::parse(body);
        assert_eq!(r.tools, 2);
        let (sb, st) = r.part(Part::System);
        assert!(sb > 0);
        assert!(st.contains("── bloco 2 ──\nBe brief."), "{st}");
        let (_, tt) = r.part(Part::Tools);
        assert!(tt.contains("● Read  (descrição 13 B"), "{tt}");
        assert!(tt.contains("● Bash  (sem descrição"), "{tt}");
        let (_, mt) = r.part(Part::Messages);
        assert_eq!(mt, "── user ──\noi\n\n[image]");
    }

    #[test]
    fn not_json_is_empty_parts() {
        let r = Request::parse(b"not json");
        assert_eq!(r.bytes, 8);
        assert_eq!(r.part_bytes, [0, 0, 0]);
        assert_eq!(r.part(Part::Tools).1, "(nenhuma)");
    }
}
