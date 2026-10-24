//! Mandrake's screens. Owner: agent B (plan items U1 to U6).
//!
//! Each screen keeps its state apart from the terminal: a state type that keys change, a `render`
//! that draws it into any [`ratatui::Frame`], and a thin loop that only [`term`] connects to stdout.

mod fmt;
mod hex;
mod term;
#[cfg(test)]
mod testutil;
mod theme;
mod watch;

use anyhow::Result;
use mandrake_home::{Dirs, Profile};
use mandrake_layout::Catalog;
use std::path::Path;

pub struct WatchOpts {
    /// Draw without the alternate screen, for a tui-embed pane inside Claude Code.
    pub embed: bool,
}

/// Live split view over `dirs.events()`: `pass` on the left, `trim` on the right.
pub fn watch(dirs: &Dirs, opts: WatchOpts) -> Result<()> {
    watch::run(dirs, opts.embed)
}

/// Side-by-side hex of `audit/<id>.before.json` and `.after.json`.
pub fn hex(dirs: &Dirs, id: &str) -> Result<()> {
    hex::run(dirs, id)
}

/// Draws a bench report written by `mandrake_proxy::bench`.
pub fn bench_view(report: &Path) -> Result<()> {
    let _ = report;
    anyhow::bail!("bench view: not built yet (plan U3)")
}

/// Walks the catalog item by item and returns the profile to apply, or None when the user quits.
pub fn setup_wizard(dirs: &Dirs, catalog: &Catalog, current: &Profile) -> Result<Option<Profile>> {
    let _ = (dirs, catalog, current);
    anyhow::bail!("setup wizard: not built yet (plan U4)")
}
