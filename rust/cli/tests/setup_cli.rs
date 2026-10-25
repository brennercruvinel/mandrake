//! `mandrake setup --cli` with its input closed: it stops before the summary and applies nothing.

#![allow(
    clippy::unwrap_used,
    reason = "an integration test: a failed unwrap is the failure report"
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Sandbox(PathBuf);

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sandbox() -> Sandbox {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("mandrake-cli-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(root.join("home/.claude")).unwrap();
    Sandbox(root)
}

fn setup_cli(root: &Path, stdin: Stdio) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_mandrake"))
        .args(["setup", "--cli"])
        .env_clear()
        .env("HOME", root.join("home"))
        .env("MANDRAKE_HOME", root.join("mh"))
        .env("CLAUDE_CONFIG_DIR", root.join("home/.claude"))
        .stdin(stdin)
        .output()
        .unwrap()
}

#[test]
fn closed_input_applies_nothing() {
    let s = sandbox();
    let settings = s.0.join("home/.claude/settings.json");
    std::fs::write(&settings, "{\"mine\":1}").unwrap();

    let out = setup_cli(&s.0, Stdio::null());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !out.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(stderr.contains("nada foi aplicado"), "{stderr}");
    assert!(!stdout.contains("Instalando"), "{stdout}");
    assert!(!s.0.join("mh").exists(), "no profile, journal or backup");
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), "{\"mine\":1}");
}
