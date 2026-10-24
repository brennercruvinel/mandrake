//! Links of the Claude Code folder: what is there, what the profile wants, and the steps between.
//!
//! Nothing is ever deleted. A real file or folder in the way moves to `state/backups/<stamp>/`, and
//! every step goes to `state/journal/<stamp>.json`, which `rollback` replays backwards.

use anyhow::{Context, Result, bail};
use mandrake_home::{Action, Dirs, LinkSpec, Profile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::catalog::{Catalog, Kind};

/// What sits at a path, without following a symlink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase", tag = "is", content = "target")]
pub enum Found {
    Missing,
    Link(PathBuf),
    File,
    Dir,
}

pub fn inspect(path: &Path) -> Found {
    match std::fs::symlink_metadata(path) {
        Err(_) => Found::Missing,
        Ok(m) if m.file_type().is_symlink() => {
            Found::Link(std::fs::read_link(path).unwrap_or_default())
        }
        Ok(m) if m.is_dir() => Found::Dir,
        Ok(_) => Found::File,
    }
}

/// One entry and what applying the profile does to it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Step {
    /// Path inside the Claude Code folder.
    pub path: String,
    pub found: Found,
    pub action: Action,
    pub target: Option<PathBuf>,
    pub op: Op,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Op {
    /// Already as wanted, or `keep`.
    None,
    Link,
    /// A link to somewhere else is replaced.
    Relink,
    /// A real file or folder moves to the backup, then the link goes in.
    BackupLink,
    Copy,
    BackupCopy,
}

impl Op {
    pub fn changes(self) -> bool {
        self != Op::None
    }
}

/// The steps from what is there to what the profile wants, in catalog order, then the extra paths.
pub fn plan(dirs: &Dirs, catalog: &Catalog, profile: &Profile) -> Result<Vec<Step>> {
    let mut keys: Vec<String> = catalog.items.iter().map(|i| i.path.clone()).collect();
    keys.extend(
        profile
            .links
            .keys()
            .filter(|k| catalog.get(k).is_none())
            .cloned(),
    );
    let mut steps = vec![];
    for key in keys {
        check_key(&key)?;
        let spec = profile.links.get(&key).cloned().unwrap_or(LinkSpec {
            action: Action::Keep,
            target: None,
        });
        let found = inspect(&dirs.claude.join(&key));
        let target = profile.target(dirs, &key);
        if spec.action != Action::Keep && target.is_none() {
            bail!("links.{key}: {:?} needs a target", spec.action);
        }
        let op = match (spec.action, &found) {
            (Action::Keep, _) => Op::None,
            (Action::Link, Found::Link(t)) if Some(t) == target.as_ref() => Op::None,
            (Action::Link, Found::Missing) => Op::Link,
            (Action::Link, Found::Link(_)) => Op::Relink,
            (Action::Link, _) => Op::BackupLink,
            (Action::Copy, Found::Missing) => Op::Copy,
            (Action::Copy, _) => Op::BackupCopy,
        };
        steps.push(Step {
            path: key,
            found,
            action: spec.action,
            target,
            op,
        });
    }
    Ok(steps)
}

/// A profile key is a relative path that stays inside the Claude Code folder.
fn check_key(key: &str) -> Result<()> {
    let p = Path::new(key);
    if key.is_empty()
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("links: '{key}' must be a plain relative path inside the Claude Code folder");
    }
    Ok(())
}

/// One applied step, enough to undo it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Done {
    pub path: String,
    pub before: Found,
    pub op: Op,
    /// Where the real file or folder went, for `backup-*` ops.
    pub backup: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub stamp: String,
    pub claude: PathBuf,
    pub done: Vec<Done>,
}

/// Applies the steps that change something and writes the journal; returns its path.
pub fn apply(
    dirs: &Dirs,
    catalog: &Catalog,
    steps: &[Step],
    stamp: &str,
) -> Result<Option<PathBuf>> {
    let todo: Vec<&Step> = steps.iter().filter(|s| s.op.changes()).collect();
    if todo.is_empty() {
        return Ok(None);
    }
    let mut journal = Journal {
        stamp: stamp.into(),
        claude: dirs.claude.clone(),
        done: vec![],
    };
    let jpath = dirs.journal().join(format!("{stamp}.json"));
    std::fs::create_dir_all(dirs.journal())?;
    let result = (|| -> Result<()> {
        for s in todo {
            let at = dirs.claude.join(&s.path);
            let target = s.target.as_ref().expect("plan checked the target");
            let kind = catalog.get(&s.path).map(|i| i.kind);
            let mut backup = None;
            if matches!(s.op, Op::BackupLink | Op::BackupCopy) {
                let to = dirs.backups().join(stamp).join(&s.path);
                std::fs::create_dir_all(to.parent().unwrap())?;
                std::fs::rename(&at, &to)
                    .with_context(|| format!("moving {} to the backup", at.display()))?;
                backup = Some(to);
            }
            if s.op == Op::Relink {
                std::fs::remove_file(&at)
                    .with_context(|| format!("removing the old link {}", at.display()))?;
            }
            if let Some(dir) = at.parent() {
                std::fs::create_dir_all(dir)?;
            }
            match s.action {
                Action::Link => {
                    if kind == Some(Kind::Dir) && !target.exists() {
                        std::fs::create_dir_all(target)?;
                    }
                    symlink(target, &at).with_context(|| {
                        format!("linking {} -> {}", at.display(), target.display())
                    })?;
                }
                Action::Copy => copy_tree(target, &at)?,
                Action::Keep => unreachable!(),
            }
            journal.done.push(Done {
                path: s.path.clone(),
                before: s.found.clone(),
                op: s.op,
                backup,
            });
            write_journal(&jpath, &journal)?;
        }
        Ok(())
    })();
    write_journal(&jpath, &journal)?;
    result.map(|_| Some(jpath))
}

fn write_journal(path: &Path, j: &Journal) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(j)?)
        .with_context(|| format!("writing {}", path.display()))
}

/// Undoes the newest journal not rolled back yet; returns how many steps it undid.
pub fn rollback(dirs: &Dirs) -> Result<usize> {
    let Some(jpath) = latest_journal(dirs)? else {
        bail!("no journal to roll back in {}", dirs.journal().display())
    };
    let j: Journal = serde_json::from_slice(&std::fs::read(&jpath)?)?;
    for d in j.done.iter().rev() {
        let at = j.claude.join(&d.path);
        match inspect(&at) {
            Found::Link(_) => std::fs::remove_file(&at)?,
            Found::Dir => std::fs::remove_dir_all(&at)?,
            Found::File => std::fs::remove_file(&at)?,
            Found::Missing => {}
        }
        match (&d.before, &d.backup) {
            (_, Some(b)) => {
                std::fs::rename(b, &at).with_context(|| format!("restoring {}", at.display()))?
            }
            (Found::Link(t), None) => symlink(t, &at)?,
            _ => {}
        }
    }
    std::fs::rename(&jpath, jpath.with_extension("rolled-back.json"))?;
    Ok(j.done.len())
}

fn latest_journal(dirs: &Dirs) -> Result<Option<PathBuf>> {
    let Ok(rd) = std::fs::read_dir(dirs.journal()) else {
        return Ok(None);
    };
    let mut js: Vec<PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| e == "json")
                && !p.to_string_lossy().ends_with(".rolled-back.json")
        })
        .collect();
    js.sort();
    Ok(js.pop())
}

/// The current folder as a profile: every symlink at the top level and one level down becomes a
/// `link`, relative to the deepest folder all targets share when that is not `/` or the home.
pub fn capture(dirs: &Dirs) -> Result<Profile> {
    let mut found: Vec<(String, PathBuf)> = vec![];
    let top = std::fs::read_dir(&dirs.claude)
        .with_context(|| format!("reading {}", dirs.claude.display()))?;
    for e in top.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        match inspect(&e.path()) {
            Found::Link(t) => found.push((name, t)),
            Found::Dir => {
                for sub in std::fs::read_dir(e.path())?.flatten() {
                    if let Found::Link(t) = inspect(&sub.path()) {
                        found.push((format!("{name}/{}", sub.file_name().to_string_lossy()), t));
                    }
                }
            }
            _ => {}
        }
    }
    found.sort();
    let root = common_root(found.iter().map(|(_, t)| t.as_path()))
        .filter(|r| r != Path::new("/") && *r != dirs.home);
    let mut p = Profile {
        dev_root: root.as_ref().map(|r| r.to_string_lossy().into_owned()),
        ..Default::default()
    };
    for (key, t) in found {
        let rel = root
            .as_ref()
            .and_then(|r| t.strip_prefix(r).ok())
            .map(Path::to_path_buf)
            .unwrap_or(t);
        p.links.insert(
            key,
            LinkSpec {
                action: Action::Link,
                target: Some(rel.to_string_lossy().into_owned()),
            },
        );
    }
    Ok(p)
}

fn common_root<'a>(paths: impl Iterator<Item = &'a Path>) -> Option<PathBuf> {
    let mut root: Option<PathBuf> = None;
    for p in paths.filter(|p| p.is_absolute()) {
        let parent = p.parent().unwrap_or(p);
        root = Some(match root {
            None => parent.to_path_buf(),
            Some(r) => r
                .ancestors()
                .find(|a| parent.starts_with(a))
                .unwrap_or(Path::new("/"))
                .to_path_buf(),
        });
    }
    root
}

#[cfg(unix)]
fn symlink(target: &Path, at: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let m = std::fs::metadata(from).with_context(|| format!("copy source {}", from.display()))?;
    if m.is_file() {
        std::fs::copy(from, to)?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        copy_tree(&e.path(), &to.join(e.file_name()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
