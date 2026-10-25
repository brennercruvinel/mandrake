//! Links of the Claude Code folder: what is there, what the profile wants, and the steps between.
//!
//! Nothing is ever deleted. A real file or folder in the way moves to `state/backups/<stamp>/`, and
//! every step goes to `state/journal/<stamp>.json`, which `rollback` replays backwards. A step is
//! written as pending before anything moves, so a run that stops halfway can still be undone.
//!
//! A real folder linked to a target that does not exist yet fills the target first (`seed-link`),
//! so the entry shows the same files through the link (#27).

use anyhow::{Context, Result, bail};
use mandrake_home::{Action, Dirs, LinkSpec, Profile};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::catalog::{Catalog, Kind};
use crate::tree;

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
    /// The target is missing, or an empty folder: the real file or folder is copied there first,
    /// then it moves to the backup and the link goes in, so nothing disappears from view.
    SeedLink,
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
            bail!(
                "links.{key}: a ação {:?} precisa de um destino (target)",
                spec.action
            );
        }
        let op = match (spec.action, &found) {
            (Action::Keep, _) => Op::None,
            (Action::Link, Found::Link(t)) if Some(t) == target.as_ref() => Op::None,
            (Action::Link, Found::Missing) => Op::Link,
            (Action::Link, Found::Link(_)) => Op::Relink,
            (Action::Link, Found::File | Found::Dir)
                if target.as_deref().is_some_and(|t| vacant(t, &found)) =>
            {
                Op::SeedLink
            }
            (Action::Link, _) => Op::BackupLink,
            (Action::Copy, Found::Missing) => Op::Copy,
            (Action::Copy, Found::File | Found::Dir)
                if target
                    .as_deref()
                    .is_some_and(|t| tree::same(t, &dirs.claude.join(&key))) =>
            {
                Op::None
            }
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

/// A target with nothing to show yet: missing, or an empty folder when the entry is a folder.
fn vacant(target: &Path, entry: &Found) -> bool {
    match inspect(target) {
        Found::Missing => true,
        Found::Dir => {
            *entry == Found::Dir && std::fs::read_dir(target).is_ok_and(|mut d| d.next().is_none())
        }
        _ => false,
    }
}

impl Step {
    /// What applying does to this entry, in one line a person reads.
    pub fn say(&self, dirs: &Dirs) -> String {
        let t = self
            .target
            .as_deref()
            .map(|t| dirs.tilde(t))
            .unwrap_or_default();
        let p = &self.path;
        match self.op {
            Op::None => format!("{p}: fica como está"),
            Op::Link => format!("{p}: link para {t}"),
            Op::Relink => format!("{p}: o link passa a apontar para {t}"),
            Op::BackupLink => {
                format!("{p}: vai para o backup, e o link mostra o que já está em {t}")
            }
            Op::SeedLink => match tree::count_files(&dirs.claude.join(p)) {
                0 => format!("{p}: vira link para {t}"),
                1 => format!("{p}: 1 arquivo vai para {t}, e o backup guarda uma cópia"),
                n => format!("{p}: {n} arquivos vão para {t}, e o backup guarda uma cópia"),
            },
            Op::Copy => format!("{p}: recebe uma cópia de {t}"),
            Op::BackupCopy => format!("{p}: vai para o backup e recebe uma cópia de {t}"),
        }
    }
}

/// A profile key is a relative path that stays inside the Claude Code folder.
fn check_key(key: &str) -> Result<()> {
    let p = Path::new(key);
    if key.is_empty()
        || p.is_absolute()
        || p.components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!(
            "links: '{key}' precisa ser um caminho relativo simples, dentro da pasta do Claude Code"
        );
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
    /// Written before the step starts and cleared once it finished: a run that stopped here may
    /// have done part of it. Journals from before this field read as finished steps.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub pending: bool,
    /// For `seed-link`: the target this step filled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seeded: Option<Seeded>,
}

/// A target filled by `seed-link`. Rollback puts it aside rather than deleting it, so whatever was
/// written through the link after the run is still there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Seeded {
    pub target: PathBuf,
    /// Where rollback moves the target: `backups/<stamp>/.seeded/<path>`.
    pub aside: PathBuf,
    /// The target was an empty folder before the run, and rollback leaves one there again.
    pub was_dir: bool,
}

/// What a rollback undid, and the filled targets it put aside.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Rolled {
    pub steps: usize,
    pub aside: Vec<Seeded>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    pub stamp: String,
    pub claude: PathBuf,
    pub done: Vec<Done>,
}

/// Applies the steps that change something and writes the journal; returns its path. Each step
/// is recorded as pending before it touches anything, so when one fails the journal already holds
/// what `rollback` needs.
pub fn apply(dirs: &Dirs, catalog: &Catalog, steps: &[Step]) -> Result<Option<PathBuf>> {
    let todo: Vec<&Step> = steps.iter().filter(|s| s.op.changes()).collect();
    if todo.is_empty() {
        return Ok(None);
    }
    let (stamp, jpath) = reserve(dirs)?;
    let mut journal = Journal {
        stamp,
        claude: dirs.claude.clone(),
        done: vec![],
    };
    for s in todo {
        let run = dirs.backups().join(&journal.stamp);
        let backup = matches!(s.op, Op::BackupLink | Op::BackupCopy | Op::SeedLink)
            .then(|| run.join(&s.path));
        let seeded = s
            .target
            .as_ref()
            .filter(|_| s.op == Op::SeedLink)
            .map(|t| Seeded {
                target: t.clone(),
                aside: run.join(".seeded").join(&s.path),
                was_dir: inspect(t) == Found::Dir,
            });
        journal.done.push(Done {
            path: s.path.clone(),
            before: s.found.clone(),
            op: s.op,
            backup: backup.clone(),
            pending: true,
            seeded,
        });
        write_journal(&jpath, &journal)?;
        let kind = catalog.get(&s.path).map(|i| i.kind);
        run_step(&dirs.claude.join(&s.path), s, kind, backup.as_deref()).with_context(|| {
            format!(
                "parou em {}; `mandrake link rollback` desfaz esta rodada ({})",
                s.path,
                jpath.display()
            )
        })?;
        if let Some(d) = journal.done.last_mut() {
            d.pending = false;
        }
        write_journal(&jpath, &journal)?;
    }
    Ok(Some(jpath))
}

/// Claims this run's journal file first, so no other run can take the same name.
fn reserve(dirs: &Dirs) -> Result<(String, PathBuf)> {
    std::fs::create_dir_all(dirs.journal())?;
    loop {
        let stamp = dirs.fresh_stamp();
        let jpath = dirs.journal().join(format!("{stamp}.json"));
        let created = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&jpath);
        match created {
            Ok(_) => return Ok((stamp, jpath)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e).with_context(|| format!("criando {}", jpath.display())),
        }
    }
}

fn run_step(at: &Path, s: &Step, kind: Option<Kind>, backup: Option<&Path>) -> Result<()> {
    let Some(target) = s.target.as_ref() else {
        bail!("{} não tem destino no perfil", s.path)
    };
    if s.op == Op::SeedLink {
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir)?;
        }
        tree::copy(at, target)
            .with_context(|| format!("copiando {} para {}", at.display(), target.display()))?;
    }
    if let Some(to) = backup {
        if let Some(dir) = to.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::rename(at, to)
            .with_context(|| format!("movendo {} para o backup", at.display()))?;
    }
    if s.op == Op::Relink {
        std::fs::remove_file(at)
            .with_context(|| format!("removendo o link antigo {}", at.display()))?;
    }
    if let Some(dir) = at.parent() {
        std::fs::create_dir_all(dir)?;
    }
    match s.action {
        Action::Link => {
            if kind == Some(Kind::Dir) && !target.exists() {
                std::fs::create_dir_all(target)?;
            }
            symlink(target, at)
                .with_context(|| format!("criando o link {} → {}", at.display(), target.display()))
        }
        Action::Copy => tree::copy(target, at),
        Action::Keep => unreachable!(),
    }
}

fn write_journal(path: &Path, j: &Journal) -> Result<()> {
    std::fs::write(path, serde_json::to_vec_pretty(j)?)
        .with_context(|| format!("escrevendo {}", path.display()))
}

/// Undoes the newest journal not rolled back yet.
pub fn rollback(dirs: &Dirs) -> Result<Rolled> {
    let Some(jpath) = latest_journal(dirs)? else {
        bail!(
            "nada para desfazer: nenhum journal em {}",
            dirs.journal().display()
        )
    };
    let j: Journal = serde_json::from_slice(&std::fs::read(&jpath)?)
        .with_context(|| format!("lendo {}", jpath.display()))?;
    let mut rolled = Rolled {
        steps: j.done.len(),
        aside: vec![],
    };
    for d in j.done.iter().rev() {
        if let Some(s) = &d.seeded
            && put_aside(s)?
        {
            rolled.aside.push(s.clone());
        }
        undo(&j.claude.join(&d.path), d)?;
    }
    std::fs::rename(&jpath, jpath.with_extension("rolled-back.json"))?;
    Ok(rolled)
}

/// Moves a filled target out of the way, and leaves an empty folder when there was one before.
/// Returns whether there was anything to move.
fn put_aside(s: &Seeded) -> Result<bool> {
    if s.target.symlink_metadata().is_err() {
        return Ok(false);
    }
    if let Some(dir) = s.aside.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::rename(&s.target, &s.aside)
        .with_context(|| format!("guardando {} no backup", s.target.display()))?;
    if s.was_dir {
        std::fs::create_dir_all(&s.target)?;
    }
    Ok(true)
}

/// Puts back what was at `at` before the step. A pending step may have stopped anywhere: when the
/// original never reached the backup, or nothing changed yet, there is nothing to undo.
fn undo(at: &Path, d: &Done) -> Result<()> {
    let moved = d.backup.as_deref().filter(|b| b.symlink_metadata().is_ok());
    if d.pending && moved.is_none() && (d.backup.is_some() || inspect(at) == d.before) {
        return Ok(());
    }
    tree::remove(at)?;
    match (&d.before, &d.backup) {
        (_, Some(b)) => {
            std::fs::rename(b, at).with_context(|| format!("restaurando {}", at.display()))?
        }
        (Found::Link(t), None) => symlink(t, at)?,
        _ => {}
    }
    Ok(())
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

/// The current folder as a profile: every symlink at the top level and one level down that points
/// outside the folder becomes a `link`, relative to the deepest folder all targets share when that is not `/` or the home.
pub fn capture(dirs: &Dirs) -> Result<Profile> {
    let mut found: Vec<(String, PathBuf)> = vec![];
    let top = std::fs::read_dir(&dirs.claude)
        .with_context(|| format!("lendo {}", dirs.claude.display()))?;
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
    // Links the CLI keeps inside its own folder (debug/latest) are its business, not the profile's.
    let own = std::fs::canonicalize(&dirs.claude).unwrap_or_else(|_| dirs.claude.clone());
    found.retain(|(_, t)| t.is_absolute() && !t.starts_with(&dirs.claude) && !t.starts_with(&own));
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

#[cfg(test)]
#[path = "links_tests.rs"]
mod tests;
