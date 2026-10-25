//! The two keys Mandrake owns in Claude Code's `settings.json`, merged without touching the rest.
//!
//! The file is written through a symlink to wherever it lives, keeping key order and two-space
//! indentation; the previous text goes to the backup first.

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

pub const BASE_URL: &str = "ANTHROPIC_BASE_URL";
pub const PLUGIN_DIRS: &str = "CLAUDE_CODE_PLUGIN_DIRS";

/// What Mandrake wants in `env`. Never `ANTHROPIC_BASE_URL`: in settings.json it sends every
/// session to the proxy, and every session dies when the proxy is down. The shell launcher sets it
/// per session, only when the proxy answers.
pub fn wanted(_port: u16, prompt_trim: &Path) -> Vec<(&'static str, String)> {
    vec![(PLUGIN_DIRS, prompt_trim.to_string_lossy().into_owned())]
}

/// Removes an `ANTHROPIC_BASE_URL` that points at a local proxy (an older Mandrake wrote it).
pub fn drop_local_base_url(doc: &mut Value) -> Option<(String, Option<String>, String)> {
    let env = doc.get_mut("env")?.as_object_mut()?;
    let old = env.get(BASE_URL)?.as_str()?.to_owned();
    if !old.starts_with("http://127.0.0.1") && !old.starts_with("http://localhost") {
        return None;
    }
    env.remove(BASE_URL);
    Some((
        BASE_URL.to_owned(),
        Some(old),
        "removido (o launcher define por sessão)".to_owned(),
    ))
}

pub fn read(path: &Path) -> Result<Value> {
    match std::fs::read_to_string(path) {
        Ok(t) if t.trim().is_empty() => Ok(Value::Object(Map::new())),
        Ok(t) => serde_json::from_str(&t).with_context(|| format!("parsing {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Value::Object(Map::new())),
        Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
    }
}

/// Sets the keys in `env`; returns the ones that changed, as (key, old, new).
pub fn merge(doc: &mut Value, keys: &[(&str, String)]) -> Vec<(String, Option<String>, String)> {
    let root = doc.as_object_mut().expect("settings.json is an object");
    let env = root
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));
    let env = env.as_object_mut().expect("settings.env is an object");
    let mut changed = vec![];
    for (k, v) in keys {
        let old = env.get(*k).and_then(Value::as_str).map(str::to_owned);
        if old.as_deref() != Some(v.as_str()) {
            env.insert((*k).to_owned(), Value::String(v.clone()));
            changed.push(((*k).to_owned(), old, v.clone()));
        }
    }
    changed
}

/// Reads, merges and writes back when something changed; the old text goes to `backup_dir`.
pub fn apply(
    path: &Path,
    keys: &[(&str, String)],
    backup_dir: &Path,
) -> Result<Vec<(String, Option<String>, String)>> {
    let mut doc = read(path)?;
    if !doc.is_object() {
        anyhow::bail!("{} is not a JSON object", path.display());
    }
    let mut changed = merge(&mut doc, keys);
    changed.extend(drop_local_base_url(&mut doc));
    if changed.is_empty() {
        return Ok(changed);
    }
    let real = real_path(path);
    if real.exists() {
        std::fs::create_dir_all(backup_dir)?;
        std::fs::copy(&real, backup_dir.join("settings.json"))?;
    } else if let Some(d) = real.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut text = serde_json::to_string_pretty(&doc)?;
    text.push('\n');
    std::fs::write(&real, text).with_context(|| format!("writing {}", real.display()))?;
    Ok(changed)
}

/// Follows a symlink so the write lands on the file and the link stays a link.
fn real_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| match std::fs::read_link(path) {
        Ok(t) if t.is_absolute() => t,
        Ok(t) => path.parent().unwrap_or(Path::new(".")).join(t),
        Err(_) => path.to_path_buf(),
    })
}

/// Whether any plugin of this name is enabled, from any marketplace (`fancy@tiamati` counts for `fancy`).
pub fn plugin_enabled(doc: &Value, name: &str) -> Option<String> {
    doc.get("enabledPlugins")?
        .as_object()?
        .iter()
        .find(|(k, v)| k.split('@').next() == Some(name) && v.as_bool() == Some(true))
        .map(|(k, _)| k.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_keeps_order_and_follows_the_link() {
        let tmp = tempfile::tempdir().unwrap();
        let real = tmp.path().join("devops/settings.json");
        std::fs::create_dir_all(real.parent().unwrap()).unwrap();
        std::fs::write(
            &real,
            r#"{"zeta":1,"env":{"KEEP":"x","ANTHROPIC_BASE_URL":"http://127.0.0.1:8787"},"alpha":2}"#,
        )
        .unwrap();
        let link = tmp.path().join("settings.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let keys = wanted(8787, Path::new("/d/plugins/prompt-trim"));
        let changed = apply(&link, &keys, &tmp.path().join("bk")).unwrap();
        assert_eq!(changed.len(), 2, "plugin dir set, local base url dropped");
        assert!(
            std::fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let text = std::fs::read_to_string(&real).unwrap();
        assert!(
            text.find("zeta").unwrap() < text.find("alpha").unwrap(),
            "key order kept"
        );
        let doc = read(&real).unwrap();
        assert_eq!(doc["env"]["KEEP"], "x");
        assert!(
            doc["env"].get(BASE_URL).is_none(),
            "the local base url is gone"
        );
        assert!(tmp.path().join("bk/settings.json").exists());
        assert!(
            apply(&link, &keys, &tmp.path().join("bk2"))
                .unwrap()
                .is_empty(),
            "idempotent"
        );
    }

    #[test]
    fn finds_a_plugin_from_any_marketplace() {
        let doc: Value =
            serde_json::from_str(r#"{"enabledPlugins":{"fancy@tiamati":true,"mermaid@x":false}}"#)
                .unwrap();
        assert_eq!(
            plugin_enabled(&doc, "fancy").as_deref(),
            Some("fancy@tiamati")
        );
        assert_eq!(plugin_enabled(&doc, "mermaid"), None);
    }
}
