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

/// `CLAUDE_CODE_PLUGIN_DIRS` is a list Claude Code splits on `:`. Ours joins it: the other folders
/// stay in their order, and a prompt-trim from an older install is replaced, not doubled.
pub fn with_plugin_dir(old: Option<&str>, ours: &str) -> String {
    let mut dirs: Vec<&str> = old
        .unwrap_or("")
        .split(':')
        .map(str::trim)
        .filter(|d| !d.is_empty() && *d != ours && !is_a_prompt_trim(d))
        .collect();
    dirs.insert(0, ours);
    dirs.join(":")
}

/// Whether the list already holds this folder.
pub fn has_plugin_dir(doc: &Value, ours: &str) -> bool {
    doc["env"][PLUGIN_DIRS]
        .as_str()
        .is_some_and(|v| v.split(':').any(|d| d.trim() == ours))
}

/// A Mandrake prompt-trim folder, from this install or an older one.
fn is_a_prompt_trim(dir: &str) -> bool {
    dir.trim_end_matches('/')
        .ends_with("data/plugins/prompt-trim")
}

/// Whether a base url is Mandrake's own proxy: local, on the profile's port or 8787, the port an
/// older Mandrake wrote. Another local proxy (LiteLLM, a router, a cost meter) is not ours.
pub fn is_ours(url: &str, port: u16) -> bool {
    let Some(rest) = url
        .strip_prefix("http://127.0.0.1:")
        .or_else(|| url.strip_prefix("http://localhost:"))
    else {
        return false;
    };
    let p = rest.split(['/', '?']).next().unwrap_or("");
    p.parse::<u16>().is_ok_and(|p| p == port || p == 8787)
}

/// Removes an `ANTHROPIC_BASE_URL` that points at Mandrake's own proxy (an older Mandrake wrote
/// it). Any other value is someone else's and stays.
pub fn drop_local_base_url(doc: &mut Value, port: u16) -> Option<(String, Option<String>, String)> {
    let env = doc.get_mut("env")?.as_object_mut()?;
    let old = env.get(BASE_URL)?.as_str()?.to_owned();
    if !is_ours(&old, port) {
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
pub fn merge(
    doc: &mut Value,
    keys: &[(&str, String)],
) -> Result<Vec<(String, Option<String>, String)>> {
    let Some(root) = doc.as_object_mut() else {
        anyhow::bail!("settings.json não é um objeto JSON");
    };
    let env = root
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(env) = env.as_object_mut() else {
        anyhow::bail!("settings.json: `env` não é um objeto");
    };
    let mut changed = vec![];
    for (k, v) in keys {
        let old = env.get(*k).and_then(Value::as_str).map(str::to_owned);
        let new = if *k == PLUGIN_DIRS {
            with_plugin_dir(old.as_deref(), v)
        } else {
            v.clone()
        };
        if old.as_deref() != Some(new.as_str()) {
            env.insert((*k).to_owned(), Value::String(new.clone()));
            changed.push(((*k).to_owned(), old, new));
        }
    }
    Ok(changed)
}

/// Reads, merges and writes back when something changed; the old text goes to `backup_dir`.
pub fn apply(
    path: &Path,
    keys: &[(&str, String)],
    port: u16,
    backup_dir: &Path,
) -> Result<Vec<(String, Option<String>, String)>> {
    let mut doc = read(path)?;
    let mut changed =
        merge(&mut doc, keys).with_context(|| format!("merging into {}", path.display()))?;
    changed.extend(drop_local_base_url(&mut doc, port));
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
    fn a_settings_file_of_the_wrong_shape_is_an_error_not_a_panic() {
        let keys = [("K", "v".to_string())];
        assert!(merge(&mut serde_json::json!([1, 2]), &keys).is_err());
        assert!(merge(&mut serde_json::json!({"env": "x"}), &keys).is_err());
        let mut ok = serde_json::json!({"env": {}});
        assert_eq!(merge(&mut ok, &keys).unwrap().len(), 1);
    }

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
        let changed = apply(&link, &keys, 8787, &tmp.path().join("bk")).unwrap();
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
            apply(&link, &keys, 8787, &tmp.path().join("bk2"))
                .unwrap()
                .is_empty(),
            "idempotent"
        );
    }

    #[test]
    fn other_plugin_folders_and_other_proxies_stay() {
        let mut doc = serde_json::json!({"env": {
            "CLAUDE_CODE_PLUGIN_DIRS": "/opt/other-plugin:/old/mandrake/src/data/plugins/prompt-trim:/x/y",
            "ANTHROPIC_BASE_URL": "http://localhost:4000",
        }});
        let keys = wanted(8787, Path::new("/d/data/plugins/prompt-trim"));
        merge(&mut doc, &keys).unwrap();
        assert_eq!(
            doc["env"][PLUGIN_DIRS], "/d/data/plugins/prompt-trim:/opt/other-plugin:/x/y",
            "ours first, the others kept in order, the old prompt-trim replaced"
        );
        assert!(has_plugin_dir(&doc, "/d/data/plugins/prompt-trim"));
        assert!(drop_local_base_url(&mut doc, 8787).is_none());
        assert_eq!(
            doc["env"][BASE_URL], "http://localhost:4000",
            "another proxy stays"
        );
        assert!(merge(&mut doc, &keys).unwrap().is_empty(), "idempotent");
    }

    #[test]
    fn only_mandrakes_own_port_counts_as_ours() {
        assert!(is_ours("http://127.0.0.1:8787", 9000), "the old default");
        assert!(is_ours("http://localhost:9000/", 9000));
        assert!(!is_ours("http://127.0.0.1:4000", 8787));
        assert!(!is_ours("https://gateway.example.com", 8787));
        assert!(!is_ours("http://127.0.0.1:87870", 8787));
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
