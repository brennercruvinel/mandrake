//! Where Mandrake lives on a machine, and the profile that describes one.
//!
//! Directories follow XDG on Linux and macOS alike, so a profile written on one reads the same on the
//! other. `MANDRAKE_HOME` moves all three under one folder, which is how tests and dry runs stay off
//! the real machine.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The three folders Mandrake owns, plus the Claude Code folder it arranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// `mandrake.toml` lives here.
    pub config: PathBuf,
    /// Installed copies: plugins, skills, prompts.
    pub data: PathBuf,
    /// Logs, audit bodies, link journal and backups.
    pub state: PathBuf,
    /// The Claude Code config folder, `CLAUDE_CONFIG_DIR` or `~/.claude`.
    pub claude: PathBuf,
    /// The user's home, for `~` in profile paths.
    pub home: PathBuf,
}

impl Dirs {
    /// Resolves from the process environment.
    pub fn from_env() -> Result<Self> {
        Self::resolve(|k| {
            std::env::var_os(k)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
        })
    }

    /// Resolves from any lookup, so tests never read the real environment.
    pub fn resolve(var: impl Fn(&str) -> Option<PathBuf>) -> Result<Self> {
        let home = var("HOME").context("HOME is not set")?;
        let claude = var("CLAUDE_CONFIG_DIR").unwrap_or_else(|| home.join(".claude"));
        if let Some(root) = var("MANDRAKE_HOME") {
            return Ok(Self {
                config: root.join("config"),
                data: root.join("data"),
                state: root.join("state"),
                claude,
                home,
            });
        }
        let xdg = |k: &str, rel: &str| var(k).unwrap_or_else(|| home.join(rel)).join("mandrake");
        Ok(Self {
            config: xdg("XDG_CONFIG_HOME", ".config"),
            data: xdg("XDG_DATA_HOME", ".local/share"),
            state: xdg("XDG_STATE_HOME", ".local/state"),
            claude,
            home,
        })
    }

    pub fn profile(&self) -> PathBuf {
        self.config.join("mandrake.toml")
    }
    /// One JSON line per API request, see `mandrake-meter`.
    pub fn events(&self) -> PathBuf {
        self.state.join("events.jsonl")
    }
    /// Request bodies before and after the trim, only when the profile turns audit on.
    pub fn audit(&self) -> PathBuf {
        self.state.join("audit")
    }
    pub fn journal(&self) -> PathBuf {
        self.state.join("journal")
    }
    pub fn backups(&self) -> PathBuf {
        self.state.join("backups")
    }
    pub fn logs(&self) -> PathBuf {
        self.state.join("logs")
    }

    /// A name no earlier run used for its journal or its backup folder: `stamp()` plus a counter,
    /// so two runs in the same second never share one, and the names still sort by time.
    pub fn fresh_stamp(&self) -> String {
        let base = stamp();
        let taken = |s: &str| {
            [
                self.journal().join(format!("{s}.json")),
                self.journal().join(format!("{s}.rolled-back.json")),
                self.backups().join(s),
            ]
            .iter()
            .any(|p| p.symlink_metadata().is_ok())
        };
        let mut n = 1u32;
        loop {
            let s = format!("{base}-{n:03}");
            if !taken(&s) {
                return s;
            }
            n += 1;
        }
    }

    /// Expands `~/` and joins a relative path onto `root`; absolute paths pass through.
    pub fn expand(&self, path: &str, root: Option<&Path>) -> PathBuf {
        if let Some(rest) = path.strip_prefix("~/") {
            return self.home.join(rest);
        }
        if path == "~" {
            return self.home.clone();
        }
        let p = PathBuf::from(path);
        match root {
            Some(r) if p.is_relative() => r.join(p),
            _ => p,
        }
    }
}

/// What to do with one entry of the Claude Code folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Leave whatever is there.
    Keep,
    /// Replace it by a symlink to `target`.
    Link,
    /// Put a copy of `target` there, no link.
    Copy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkSpec {
    pub action: Action,
    /// Relative to `dev_root`, absolute, or starting with `~/`. Unused by `keep`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyCfg {
    pub port: u16,
    pub upstream: String,
    /// Top-level request fields removed on the way out. The product default keeps `safeguards`.
    pub strip_fields: Vec<String>,
    /// Keep request bodies before and after the trim in `state/audit`, mode 600.
    pub audit: bool,
    pub audit_keep_days: u32,
    /// Ask `count_tokens` for the body before and after, off the response path.
    pub count_tokens: bool,
    /// Drop the CLI identity block from `system`. Off by default: measured on a subscription login,
    /// a long session without it cost more (269,338 -> 291,637 input tokens), not less.
    pub cut_identity: bool,
    /// Remove the `<system-reminder>` tags around message text.
    pub unwrap_reminders: bool,
}

impl Default for ProxyCfg {
    fn default() -> Self {
        Self {
            port: 8787,
            upstream: "https://api.anthropic.com".into(),
            strip_fields: vec!["metadata".into()],
            audit: false,
            audit_keep_days: 7,
            count_tokens: true,
            cut_identity: false,
            unwrap_reminders: true,
        }
    }
}

/// prompt-trim is not here: it always installs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Plugins {
    pub fancy: bool,
    pub tui_embed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Souls {
    /// A folder of souls, each `<name>/` with `<NAME>.md` (or `SOUL.md`) and `settings.json`.
    /// Unset means the souls shipped with Mandrake.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    /// The core prompt put before every soul: a path, `"none"`, or unset for the shipped
    /// `data/prompt/cores/coding.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<String>,
    /// The core only in sessions without a soul, for souls whose prompt is written to stand alone.
    pub core_only_plain: bool,
    /// Extra names per soul folder, for `claude invoke <name>`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Skills {
    /// Install the `invoke` skill, which opens a session with what the memory recorded.
    pub invoke: bool,
    /// The memory command `invoke` calls; without one, `invoke` says so and still opens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_cmd: Option<String>,
}

/// One machine's arrangement, written by `mandrake setup` or `mandrake capture`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    /// The development folder relative link targets hang from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_root: Option<String>,
    /// Keyed by the entry's path inside the Claude Code folder, as in `cfgs/setup/items.toml`.
    pub links: BTreeMap<String, LinkSpec>,
    pub proxy: ProxyCfg,
    pub plugins: Plugins,
    pub souls: Souls,
    pub skills: Skills,
}

impl Profile {
    pub fn load(path: &Path) -> Result<Self> {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)
            .with_context(|| format!("writing {}", path.display()))
    }

    pub fn dev_root(&self, dirs: &Dirs) -> Option<PathBuf> {
        self.dev_root.as_deref().map(|r| dirs.expand(r, None))
    }

    /// The absolute target of a link entry, if it has one.
    pub fn target(&self, dirs: &Dirs, key: &str) -> Option<PathBuf> {
        let spec = self.links.get(key)?;
        let t = spec.target.as_deref()?;
        Some(dirs.expand(t, self.dev_root(dirs).as_deref()))
    }
}

/// UTC now as `YYYYMMDD-HHMMSS`, for backup and journal names.
pub fn stamp() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

/// UTC now as RFC 3339 with seconds.
pub fn rfc3339() -> String {
    let (y, mo, d, h, mi, s) = utc_now();
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn utc_now() -> (i64, u32, u32, u32, u32, u32) {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (
        y,
        m,
        d,
        (rem / 3600) as u32,
        (rem % 3600 / 60) as u32,
        (rem % 60) as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<PathBuf> + 'a {
        move |k| {
            pairs
                .iter()
                .find(|(n, _)| *n == k)
                .map(|(_, v)| PathBuf::from(v))
        }
    }

    #[test]
    fn stamps_look_right() {
        let s = stamp();
        assert_eq!(s.len(), 15);
        assert!(s.starts_with("20"));
        assert!(rfc3339().ends_with('Z'));
    }

    #[test]
    fn fresh_stamps_skip_names_in_use() {
        let tmp = std::env::temp_dir().join(format!("mandrake-stamp-{}", std::process::id()));
        let d = Dirs::resolve(env(&[
            ("HOME", "/h"),
            ("MANDRAKE_HOME", tmp.to_str().unwrap()),
        ]))
        .unwrap();
        let a = d.fresh_stamp();
        assert_eq!(a.len(), 19, "{a}");
        std::fs::create_dir_all(d.journal()).unwrap();
        std::fs::write(d.journal().join(format!("{a}.json")), "{}").unwrap();
        let b = d.fresh_stamp();
        assert_ne!(a, b);
        std::fs::create_dir_all(d.backups().join(&b)).unwrap();
        let c = d.fresh_stamp();
        assert!(c != a && c != b);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn xdg_defaults_and_overrides() {
        let d = Dirs::resolve(env(&[("HOME", "/h")])).unwrap();
        assert_eq!(d.config, PathBuf::from("/h/.config/mandrake"));
        assert_eq!(d.state, PathBuf::from("/h/.local/state/mandrake"));
        assert_eq!(d.claude, PathBuf::from("/h/.claude"));
        let d = Dirs::resolve(env(&[("HOME", "/h"), ("XDG_DATA_HOME", "/x")])).unwrap();
        assert_eq!(d.data, PathBuf::from("/x/mandrake"));
    }

    #[test]
    fn mandrake_home_isolates_everything_but_claude() {
        let d = Dirs::resolve(env(&[
            ("HOME", "/h"),
            ("MANDRAKE_HOME", "/t"),
            ("CLAUDE_CONFIG_DIR", "/t/claude"),
        ]))
        .unwrap();
        assert_eq!(d.config, PathBuf::from("/t/config"));
        assert_eq!(d.claude, PathBuf::from("/t/claude"));
    }

    #[test]
    fn profile_round_trips_and_resolves_targets() {
        let mut p = Profile {
            dev_root: Some("~/dev".into()),
            ..Default::default()
        };
        p.links.insert(
            "skills".into(),
            LinkSpec {
                action: Action::Link,
                target: Some("agent/skills".into()),
            },
        );
        p.links.insert(
            "rules".into(),
            LinkSpec {
                action: Action::Link,
                target: Some("/abs/rules".into()),
            },
        );
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("m.toml");
        p.save(&f).unwrap();
        let q = Profile::load(&f).unwrap();
        assert_eq!(p, q);
        let d = Dirs::resolve(env(&[("HOME", "/h")])).unwrap();
        assert_eq!(
            q.target(&d, "skills"),
            Some(PathBuf::from("/h/dev/agent/skills"))
        );
        assert_eq!(q.target(&d, "rules"), Some(PathBuf::from("/abs/rules")));
        assert_eq!(q.proxy.strip_fields, vec!["metadata".to_string()]);
    }
}
