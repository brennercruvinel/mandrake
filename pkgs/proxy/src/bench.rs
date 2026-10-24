//! Fixed request bodies through the trim and `count_tokens`: what the cut saves, reproducibly.

use crate::{clock, count, trim};
use anyhow::{Context, Result, bail};
use mandrake_meter::{BenchReport, BenchRow};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Sums over the rows; tokens only over rows where both counts came back.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Total {
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub tokens_before: u64,
    pub tokens_after: u64,
}

impl Total {
    pub fn of(rows: &[BenchRow]) -> Self {
        let mut t = Total::default();
        for r in rows {
            t.bytes_before += r.bytes_before;
            t.bytes_after += r.bytes_after;
            if let (Some(b), Some(a)) = (r.tokens_before, r.tokens_after) {
                t.tokens_before += b;
                t.tokens_after += a;
            }
        }
        t
    }
}

/// The headers that authenticate `count_tokens`: `ANTHROPIC_API_KEY` as `x-api-key`, else the
/// `Header: value` lines in `MANDRAKE_BENCH_AUTH`. `None` when neither is set.
pub fn credential(api_key: Option<&str>, bench_auth: Option<&str>) -> Result<Option<HeaderMap>> {
    let mut h = HeaderMap::new();
    if let Some(key) = api_key.filter(|k| !k.trim().is_empty()) {
        h.insert(
            "x-api-key",
            HeaderValue::from_str(key.trim())
                .context("ANTHROPIC_API_KEY is not a valid header value")?,
        );
    } else if let Some(spec) = bench_auth.filter(|s| !s.trim().is_empty()) {
        for line in spec.lines().map(str::trim).filter(|l| !l.is_empty()) {
            let Some((name, value)) = line.split_once(':') else {
                bail!("MANDRAKE_BENCH_AUTH: expected `Header: value`");
            };
            let name = HeaderName::from_bytes(name.trim().as_bytes())
                .context("MANDRAKE_BENCH_AUTH: bad header name")?;
            let value = HeaderValue::from_str(value.trim())
                .context("MANDRAKE_BENCH_AUTH: bad header value")?;
            h.append(name, value);
        }
    } else {
        return Ok(None);
    }
    Ok(Some(count::auth_headers(&h)))
}

/// Every `*.json` in `dir`, by name.
pub fn fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading fixtures in {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "json"))
        .collect();
    out.sort();
    if out.is_empty() {
        bail!("no *.json fixtures in {}", dir.display());
    }
    Ok(out)
}

/// Trims each fixture and, with a credential, counts it before and after against `upstream`.
pub async fn measure(
    files: &[PathBuf],
    strip: &[String],
    upstream: &str,
    auth: Option<&HeaderMap>,
) -> Result<BenchReport> {
    let client = reqwest::Client::builder().build()?;
    let mut rows = Vec::with_capacity(files.len());
    for f in files {
        let mut before = std::fs::read(f).with_context(|| format!("reading {}", f.display()))?;
        // A fixture's final newline is the file's, not the request's.
        before.truncate(before.trim_ascii_end().len());
        let (after, cuts) = trim::trim(&before, trim::rules(), strip);
        let (tokens_before, tokens_after) = match auth {
            Some(h) => count::pair(&client, upstream, h, &before, &after).await,
            None => (None, None),
        };
        let name = f.file_stem().unwrap_or_default().to_string_lossy();
        let name = name.strip_suffix(".raw").unwrap_or(&name).to_string();
        rows.push(BenchRow {
            name,
            bytes_before: before.len() as u64,
            bytes_after: after.len() as u64,
            tokens_before,
            tokens_after,
            cuts,
        });
    }
    Ok(BenchReport {
        ts: clock::now(),
        counted: auth.is_some(),
        rows,
    })
}

/// `BenchReport` plus a `total`, which readers of `BenchReport` ignore.
pub fn to_json(report: &BenchReport) -> Result<String> {
    let mut v = serde_json::to_value(report)?;
    v["total"] = serde_json::to_value(Total::of(&report.rows))?;
    Ok(serde_json::to_string_pretty(&v)? + "\n")
}

pub fn markdown(report: &BenchReport) -> String {
    let opt = |n: Option<u64>| n.map_or_else(|| "-".to_string(), |n| n.to_string());
    let mut md = format!(
        "# Mandrake bench\n\n{}, {} fixture(s). ",
        report.ts,
        report.rows.len()
    );
    if report.counted {
        md.push_str("Tokens from `count_tokens`; `-` is a count that failed.\n\n");
    } else {
        md.push_str(
            "No credential (`ANTHROPIC_API_KEY` or `MANDRAKE_BENCH_AUTH`): only bytes were measured, no tokens.\n\n",
        );
    }
    md.push_str("| fixture | bytes before | bytes after | bytes saved | tokens before | tokens after | tokens saved | system | reminders | fields |\n");
    md.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for r in &report.rows {
        let saved = match (r.tokens_before, r.tokens_after) {
            (Some(b), Some(a)) => share(b, a),
            _ => "-".into(),
        };
        let fields = if r.cuts.fields.is_empty() {
            "-".to_string()
        } else {
            r.cuts.fields.join(", ")
        };
        md.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.name,
            r.bytes_before,
            r.bytes_after,
            share(r.bytes_before, r.bytes_after),
            opt(r.tokens_before),
            opt(r.tokens_after),
            saved,
            r.cuts.system_blocks,
            r.cuts.reminders,
            fields,
        ));
    }
    let t = Total::of(&report.rows);
    let (tb, ta, ts) = if report.counted && t.tokens_before > 0 {
        (
            t.tokens_before.to_string(),
            t.tokens_after.to_string(),
            share(t.tokens_before, t.tokens_after),
        )
    } else {
        ("-".into(), "-".into(), "-".into())
    };
    let sys: u32 = report.rows.iter().map(|r| r.cuts.system_blocks).sum();
    let rem: u32 = report.rows.iter().map(|r| r.cuts.reminders).sum();
    md.push_str(&format!(
        "| **total** | {} | {} | {} | {tb} | {ta} | {ts} | {sys} | {rem} | |\n",
        t.bytes_before,
        t.bytes_after,
        share(t.bytes_before, t.bytes_after),
    ));
    md
}

fn share(before: u64, after: u64) -> String {
    if before == 0 {
        return "-".into();
    }
    let saved = before as f64 - after as f64;
    format!("{} ({:.1}%)", saved as i64, 100.0 * saved / before as f64)
}

/// Writes `<out>.json` and `<out>.md`, the same names `bench --view` reads.
pub fn write(report: &BenchReport, out: &Path) -> Result<(PathBuf, PathBuf)> {
    if let Some(dir) = out.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let (json, md) = (out.with_extension("json"), out.with_extension("md"));
    std::fs::write(&json, to_json(report)?)
        .with_context(|| format!("writing {}", json.display()))?;
    std::fs::write(&md, markdown(report)).with_context(|| format!("writing {}", md.display()))?;
    Ok((json, md))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
    }

    #[test]
    fn credential_sources() {
        assert!(credential(None, None).unwrap().is_none());
        let h = credential(Some("sk-test"), Some("Authorization: Bearer x"))
            .unwrap()
            .unwrap();
        assert_eq!(h["x-api-key"], "sk-test");
        assert!(h.get("authorization").is_none());
        let h = credential(None, Some("Authorization: Bearer x\nanthropic-beta: oauth"))
            .unwrap()
            .unwrap();
        assert_eq!(h["authorization"], "Bearer x");
        assert_eq!(h["anthropic-beta"], "oauth");
        assert_eq!(h["anthropic-version"], "2023-06-01");
        assert!(credential(None, Some("no colon")).is_err());
    }

    #[test]
    fn shipped_fixtures_trim_and_report_without_credential() {
        let files = fixtures(&repo_fixtures()).unwrap();
        let names: Vec<_> = files
            .iter()
            .filter_map(|f| f.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        for want in ["reminders", "short", "tools"] {
            assert!(names.contains(&want.to_string()), "{names:?}");
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let report = rt
            .block_on(measure(
                &files,
                &["metadata".into()],
                "http://127.0.0.1:1",
                None,
            ))
            .unwrap();
        assert!(!report.counted);
        for r in &report.rows {
            assert!(r.bytes_after < r.bytes_before, "{}", r.name);
            assert_eq!(r.cuts.system_blocks, 1, "{}", r.name);
            assert_eq!(r.cuts.fields, vec!["metadata".to_string()], "{}", r.name);
            assert!(r.tokens_before.is_none());
        }
        let many = report.rows.iter().find(|r| r.name == "reminders").unwrap();
        assert!(many.cuts.reminders >= 5);

        let dir = tempfile::tempdir().unwrap();
        let (json, md) = write(&report, &dir.path().join("b/latest")).unwrap();
        let back: BenchReport =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(back, report);
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(
            v["total"]["bytes_before"],
            Total::of(&report.rows).bytes_before
        );
        let md = std::fs::read_to_string(md).unwrap();
        assert!(md.contains("only bytes were measured"));
        assert!(md.contains("| **total** |"));
    }
}
