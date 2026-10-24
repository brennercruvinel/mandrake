//! Mandrake's screens. Owner: agent B (plan items U1 to U6).

use anyhow::{Result, bail};
use mandrake_home::{Dirs, Profile};
use mandrake_layout::Catalog;
use std::path::Path;

pub struct WatchOpts {
    /// Draw without the alternate screen, for a tui-embed pane inside Claude Code.
    pub embed: bool,
}

/// Live split view over `dirs.events()`: `pass` on the left, `trim` on the right.
pub fn watch(dirs: &Dirs, opts: WatchOpts) -> Result<()> {
    let _ = (dirs, opts.embed);
    bail!("watch: not built yet (plan U1)")
}

/// Side-by-side hex of `audit/<id>.before.json` and `.after.json`.
pub fn hex(dirs: &Dirs, id: &str) -> Result<()> {
    let _ = (dirs, id);
    bail!("hex: not built yet (plan U2)")
}

/// Draws a bench report written by `mandrake_proxy::bench`.
pub fn bench_view(report: &Path) -> Result<()> {
    let _ = report;
    bail!("bench view: not built yet (plan U3)")
}

/// Walks the catalog item by item and returns the profile to apply, or None when the user quits.
pub fn setup_wizard(dirs: &Dirs, catalog: &Catalog, current: &Profile) -> Result<Option<Profile>> {
    let _ = (dirs, catalog, current);
    bail!("setup wizard: not built yet (plan U4)")
}
