//! The one marked block Mandrake keeps in the shell's rc file, and the env file it sources.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const BEGIN: &str = "# >>> mandrake >>>";
const END: &str = "# <<< mandrake <<<";

/// The rc file of the user's shell: zsh's `.zshrc`, or bash's `.bashrc` (`.bash_profile` on macOS).
pub fn rc_file(home: &Path, shell: Option<&str>) -> PathBuf {
    let name = shell
        .and_then(|s| Path::new(s).file_name())
        .map(|n| n.to_string_lossy().into_owned());
    match name.as_deref() {
        Some("bash") if cfg!(target_os = "macos") => home.join(".bash_profile"),
        Some("bash") => home.join(".bashrc"),
        _ => home.join(".zshrc"),
    }
}

/// The block that sources Mandrake's shell file.
pub fn block(script: &Path) -> String {
    let s = script.display();
    format!("{BEGIN}\n[ -f \"{s}\" ] && . \"{s}\"\n{END}\n")
}

/// Puts the block in `text`, replacing an older one; the rest is untouched.
pub fn with_block(text: &str, block: &str) -> String {
    let base = without_block(text);
    let mut out = base.trim_end_matches('\n').to_owned();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(block);
    out
}

pub fn without_block(text: &str) -> String {
    let (Some(b), Some(e)) = (text.find(BEGIN), text.find(END)) else {
        return text.to_owned();
    };
    if e < b {
        return text.to_owned();
    }
    let end = e + END.len() + text[e + END.len()..].find('\n').map_or(0, |_| 1);
    let mut out = text[..b].trim_end_matches('\n').to_owned();
    let rest = &text[end..];
    if !rest.trim().is_empty() {
        out.push('\n');
        out.push_str(rest);
    } else if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Writes the block into the rc file when it differs; returns whether it changed.
pub fn install(rc: &Path, script: &Path) -> Result<bool> {
    let old = std::fs::read_to_string(rc).unwrap_or_default();
    let new = with_block(&old, &block(script));
    if new == old {
        return Ok(false);
    }
    std::fs::write(rc, new).with_context(|| format!("writing {}", rc.display()))?;
    Ok(true)
}

/// `export` lines for the shell file, quoted for sh.
pub fn env_file(vars: &[(&str, String)]) -> String {
    let mut s =
        String::from("# Written by mandrake setup; edit the profile and run it again instead.\n");
    for (k, v) in vars {
        // The installed value wins over one a parent shell exported, an old install's or another
        // checkout's (#33). A one-off `MANDRAKE_PORT=8788 claude` still reaches the launcher: it is
        // set at the call, long after the shell sourced this file.
        s.push_str(&format!("export {k}={}\n", sh_quote(v)));
    }
    s
}

/// The variables an `env.sh` exports, in order; reads this format and the one before #33.
pub fn env_vars(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|l| l.split_once("export ").map(|(_, kv)| kv))
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), unquote(v)))
        .collect()
}

/// The inverse of `sh_quote`.
fn unquote(v: &str) -> String {
    v.strip_prefix('\'')
        .and_then(|v| v.strip_suffix('\''))
        .map(|v| v.replace("'\\''", "'"))
        .unwrap_or_else(|| v.to_string())
}

/// Single-quoted for sh.
pub fn sh_quote(v: &str) -> String {
    format!("'{}'", v.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_is_idempotent_and_removable() {
        let rc = "export A=1\nsource /old/claude.sh\n";
        let b = block(Path::new("/d/shell/mandrake.sh"));
        let once = with_block(rc, &b);
        assert!(once.starts_with("export A=1\nsource /old/claude.sh\n\n# >>> mandrake >>>"));
        assert_eq!(with_block(&once, &b), once);
        let moved = with_block(&once, &block(Path::new("/e/mandrake.sh")));
        assert_eq!(moved.matches(BEGIN).count(), 1);
        assert!(moved.contains("/e/mandrake.sh"));
        assert_eq!(without_block(&once), rc);
    }

    #[test]
    fn picks_the_rc_of_the_shell() {
        let h = Path::new("/h");
        assert_eq!(rc_file(h, Some("/bin/zsh")), h.join(".zshrc"));
        assert_eq!(rc_file(h, None), h.join(".zshrc"));
        let bash = rc_file(h, Some("/usr/bin/bash"));
        assert!(bash == h.join(".bashrc") || bash == h.join(".bash_profile"));
    }

    #[test]
    fn env_file_quotes() {
        assert_eq!(
            env_file(&[("X", "it's".into())]).lines().nth(1),
            Some("export X='it'\\''s'")
        );
    }

    #[test]
    fn env_vars_reads_both_formats_back() {
        let text = env_file(&[("A", "it's".into()), ("B", "/x y".into())]);
        let want = vec![("A".into(), "it's".into()), ("B".into(), "/x y".into())];
        assert_eq!(env_vars(&text), want);
        let old = "# old\n[ -n \"${A+x}\" ] || export A='it'\\''s'\n[ -n \"${B+x}\" ] || export B='/x y'\n";
        assert_eq!(env_vars(old), want);
    }
}
