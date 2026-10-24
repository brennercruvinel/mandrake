//! The measurement contract: one [`Event`] per API request, one JSON line each in
//! `state/events.jsonl`. The proxy writes it; `watch`, `hex`, `bench` and `audit` read it.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// Whether the proxy trimmed the request or passed it untouched (the "before" side of an A/B).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Trim,
    Pass,
}

/// What the trim removed, by kind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cuts {
    /// `system` blocks equal to a CLI identity line.
    pub system_blocks: u32,
    /// `<system-reminder>` wrappers removed from message text.
    pub reminders: u32,
    /// Top-level fields removed, by name.
    pub fields: Vec<String>,
}

/// Token usage as the API reported it for the response.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_create: u64,
}

/// One API request through the proxy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Unique per request; names the audit bodies `audit/<id>.before.json` and `.after.json`.
    pub id: String,
    /// RFC 3339, UTC.
    pub ts: String,
    pub mode: Mode,
    /// Request path without the query, e.g. `/v1/messages`.
    pub path: String,
    pub model: Option<String>,
    /// HTTP status from upstream; 502 when the proxy could not reach it.
    pub status: u16,
    /// Time to the end of the response, in milliseconds.
    pub ms: u64,
    pub bytes_before: u64,
    pub bytes_after: u64,
    /// From `count_tokens`; absent when it was off or failed.
    pub tokens_before: Option<u64>,
    pub tokens_after: Option<u64>,
    pub usage: Option<Usage>,
    pub cuts: Cuts,
    /// Whether audit bodies were kept for this id.
    #[serde(default)]
    pub audited: bool,
    /// Set when the request failed or the client went away.
    pub error: Option<String>,
}

impl Event {
    /// Tokens the trim saved on this request, when both counts exist.
    pub fn saved_tokens(&self) -> Option<i64> {
        Some(self.tokens_before? as i64 - self.tokens_after? as i64)
    }
}

/// Appends one event as a JSON line, creating the file and its folder.
pub fn append(path: &Path, event: &Event) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let mut line = serde_json::to_vec(event)?;
    line.push(b'\n');
    f.write_all(&line)?;
    Ok(())
}

/// Reads every event; a line that does not parse is skipped, so a torn last write never blocks a reader.
pub fn read(path: &Path) -> Result<Vec<Event>> {
    let f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    Ok(BufReader::new(f)
        .lines()
        .map_while(Result::ok)
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect())
}

/// Totals over a set of events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Totals {
    pub requests: u64,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub tokens_before: u64,
    pub tokens_after: u64,
    pub usage: Usage,
}

impl Totals {
    pub fn of<'a>(events: impl IntoIterator<Item = &'a Event>) -> Self {
        let mut t = Totals::default();
        for e in events {
            t.requests += 1;
            t.bytes_before += e.bytes_before;
            t.bytes_after += e.bytes_after;
            if let (Some(b), Some(a)) = (e.tokens_before, e.tokens_after) {
                t.tokens_before += b;
                t.tokens_after += a;
            }
            if let Some(u) = &e.usage {
                t.usage.input += u.input;
                t.usage.output += u.output;
                t.usage.cache_read += u.cache_read;
                t.usage.cache_create += u.cache_create;
            }
        }
        t
    }

    /// Share of tokens the trim removed, 0.0 to 1.0.
    pub fn saved_ratio(&self) -> f64 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        1.0 - self.tokens_after as f64 / self.tokens_before as f64
    }
}

/// One fixture in a bench run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchRow {
    /// Fixture file stem.
    pub name: String,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub tokens_before: Option<u64>,
    pub tokens_after: Option<u64>,
    pub cuts: Cuts,
}

/// What `mandrake bench` writes as `<out>.json` and `bench --view` draws.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BenchReport {
    /// RFC 3339, UTC.
    pub ts: String,
    /// False when no credential was available and only bytes were measured.
    pub counted: bool,
    pub rows: Vec<BenchRow>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, before: u64, after: u64) -> Event {
        Event {
            id: id.into(),
            ts: "2026-10-24T00:00:00Z".into(),
            mode: Mode::Trim,
            path: "/v1/messages".into(),
            model: None,
            status: 200,
            ms: 10,
            bytes_before: before * 4,
            bytes_after: after * 4,
            tokens_before: Some(before),
            tokens_after: Some(after),
            usage: None,
            cuts: Cuts { system_blocks: 1, reminders: 2, fields: vec!["metadata".into()] },
            audited: false,
            error: None,
        }
    }

    #[test]
    fn append_read_and_totals() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("s/events.jsonl");
        append(&f, &ev("a", 100, 80)).unwrap();
        append(&f, &ev("b", 100, 60)).unwrap();
        std::fs::OpenOptions::new().append(true).open(&f).unwrap().write_all(b"{torn").unwrap();
        let all = read(&f).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].saved_tokens(), Some(40));
        let t = Totals::of(&all);
        assert_eq!((t.tokens_before, t.tokens_after), (200, 140));
        assert!((t.saved_ratio() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn missing_file_reads_empty() {
        assert!(read(Path::new("/nonexistent/events.jsonl")).unwrap().is_empty());
    }
}
