//! Request bodies kept before and after the trim, when the profile asks. Never headers; secrets redacted.

use anyhow::{Context, Result};
use regex::Regex;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::OnceLock;
use std::time::{Duration, SystemTime};

/// Values that look like a credential. The private key pattern runs to the matching `END` line, or to
/// the end of the JSON string when there is none, so no key body is left behind.
#[allow(
    clippy::expect_used,
    reason = "a fixed pattern that the tests compile, so it cannot fail at run time"
)]
fn secrets() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(concat!(
            r"sk-ant-[A-Za-z0-9_\-]+",
            r"|github_pat_[A-Za-z0-9_]+",
            r"|ghp_[A-Za-z0-9]+",
            r"|AKIA[0-9A-Z]{16}",
            r"|xox[bp]-[A-Za-z0-9\-]+",
            r"|-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----(?:[^\x22]*?-----END [A-Z0-9 ]*PRIVATE KEY-----|[^\x22]*)",
        ))
        .expect("secret patterns compile")
    })
}

pub fn redact(text: &str) -> String {
    secrets().replace_all(text, "[redacted]").into_owned()
}

/// Writes `<id>.before.json` and `<id>.after.json` under `dir`, mode 0600, folder 0700.
pub fn write(dir: &Path, id: &str, before: &[u8], after: &[u8]) -> Result<()> {
    create_private_dir(dir)?;
    for (side, body) in [("before", before), ("after", after)] {
        let path = dir.join(format!("{id}.{side}.json"));
        let text = redact(&String::from_utf8_lossy(body));
        let mut f = private_file(&path).with_context(|| format!("writing {}", path.display()))?;
        f.write_all(text.as_bytes())?;
    }
    Ok(())
}

/// Deletes audit bodies older than `keep_days`; returns how many went. A missing folder is fine.
pub fn sweep(dir: &Path, keep_days: u32) -> Result<usize> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    let cutoff = SystemTime::now() - Duration::from_secs(u64::from(keep_days) * 86_400);
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.ends_with(".before.json") || name.ends_with(".after.json")) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| t < cutoff);
        if old && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(unix)]
fn private_file(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // The mode above only applies on creation.
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    Ok(f)
}

#[cfg(not(unix))]
fn private_file(path: &Path) -> std::io::Result<std::fs::File> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
}

#[cfg(unix)]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_each_kind() {
        let cases = [
            ("key sk-ant-api03-AbC_d-9 end", "key [redacted] end"),
            ("ghp_0123456789abcdefABCDEF0123456789abcd.", "[redacted]."),
            ("github_pat_11ABCDEFG0_abcdefXYZ here", "[redacted] here"),
            ("aws AKIAIOSFODNN7EXAMPLE x", "aws [redacted] x"),
            (
                "slack xoxb-123-456-abc and xoxp-9-z",
                "slack [redacted] and [redacted]",
            ),
            ("AKIAshort stays", "AKIAshort stays"),
        ];
        for (input, want) in cases {
            assert_eq!(redact(input), want, "{input}");
        }
    }

    #[test]
    fn redacts_private_keys_inside_json() {
        let pem = "-----BEGIN OPENSSH PRIVATE KEY-----\\nb3BlbnNzaC1rZXk=\\n-----END OPENSSH PRIVATE KEY-----";
        let json = format!(r#"{{"text":"see {pem} ok","n":1}}"#);
        let out = redact(&json);
        assert_eq!(out, r#"{"text":"see [redacted] ok","n":1}"#);
        serde_json::from_str::<serde_json::Value>(&out).unwrap();
        let cut = r#"{"text":"-----BEGIN RSA PRIVATE KEY-----\nMIIEow","n":1}"#;
        assert_eq!(redact(cut), r#"{"text":"[redacted]","n":1}"#);
    }

    #[test]
    fn writes_private_files_and_sweeps() {
        let dir = tempfile::tempdir().unwrap();
        let audit = dir.path().join("audit");
        write(&audit, "id1", br#"{"k":"sk-ant-xyz"}"#, b"{}").unwrap();
        let before = std::fs::read_to_string(audit.join("id1.before.json")).unwrap();
        assert_eq!(before, r#"{"k":"[redacted]"}"#);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(audit.join("id1.after.json"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::write(audit.join("notes.txt"), "x").unwrap();
        assert_eq!(sweep(&audit, 7).unwrap(), 0);
        assert_eq!(sweep(&audit, 0).unwrap(), 2);
        assert!(audit.join("notes.txt").exists());
        assert_eq!(sweep(&dir.path().join("missing"), 1).unwrap(), 0);
    }
}
