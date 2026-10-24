//! Subcommands that are glue between crates. Owner: Tiamati.

use anyhow::{Result, bail};
use mandrake_home::{Dirs, Profile};
use std::path::PathBuf;

/// The saved profile, or the defaults when there is none yet.
pub fn profile(dirs: &Dirs) -> Result<Profile> {
    let f = dirs.profile();
    if f.exists() { Profile::load(&f) } else { Ok(Profile::default()) }
}

pub fn setup(dirs: &Dirs, from: Option<PathBuf>, yes: bool) -> Result<()> {
    let _ = (dirs, from, yes);
    bail!("setup: being written (plan L3, L5 to L9)")
}

pub fn capture(dirs: &Dirs, out: Option<PathBuf>) -> Result<()> {
    let _ = (dirs, out);
    bail!("capture: being written (plan L1)")
}

pub fn link_plan(dirs: &Dirs) -> Result<()> {
    let _ = dirs;
    bail!("link plan: being written (plan L2)")
}

pub fn link_apply(dirs: &Dirs) -> Result<()> {
    let _ = dirs;
    bail!("link apply: being written (plan L3)")
}

pub fn link_rollback(dirs: &Dirs) -> Result<()> {
    let _ = dirs;
    bail!("link rollback: being written (plan L4)")
}

pub fn audit(dirs: &Dirs, last: usize) -> Result<()> {
    let mut events: Vec<_> = mandrake_meter::read(&dirs.events())?.into_iter().filter(|e| e.audited).collect();
    events.reverse();
    for e in events.into_iter().take(last) {
        let saved = e.saved_tokens().map(|s| s.to_string()).unwrap_or_else(|| "-".into());
        println!("{}\t{}\t{:?}\t{}\t{} -> {} bytes\tsaved {} tokens", e.id, e.ts, e.mode, e.status, e.bytes_before, e.bytes_after, saved);
    }
    Ok(())
}

pub fn doctor(dirs: &Dirs) -> Result<()> {
    let _ = dirs;
    bail!("doctor: being written (plan L10)")
}
