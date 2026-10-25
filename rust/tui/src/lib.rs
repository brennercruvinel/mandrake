//! Mandrake's screens. Owner: agent B (plan items U1 to U6).
//!
//! Each screen keeps its state apart from the terminal: a state type that keys change, a `render`
//! that draws it into any [`ratatui::Frame`], and a thin loop that only [`term`] connects to stdout.

mod bench;
mod compare;
mod fmt;
mod hex;
mod snap;
mod term;
#[cfg(test)]
mod testutil;
mod theme;
mod watch;
mod wizard;

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

/// Side-by-side hex of `audit/<id>.before.json` and `.after.json`; without an id, the newest.
pub fn hex(dirs: &Dirs, id: Option<&str>) -> Result<()> {
    let id = match id {
        Some(id) => id.to_string(),
        None => hex::HexView::latest(dirs)?,
    };
    hex::run(dirs, &id)
}

/// Whether both audited bodies of a request are still on disk.
pub fn has_bodies(dirs: &Dirs, id: &str) -> bool {
    hex::has_bodies(dirs, id)
}

/// Side by side, part by part, the request Claude Code sends without Mandrake and with it.
/// On a terminal it opens the screen; otherwise, or with `print`, it prints the table.
pub fn compare(without: &[u8], with: &[u8], prompt: &str, print: bool) -> Result<()> {
    use std::io::IsTerminal;
    let mut view = compare::CompareView::new(without, with, prompt);
    if print || !std::io::stdout().is_terminal() {
        print!("{}", compare::summary(&view));
        return Ok(());
    }
    compare::run(&mut view)
}

/// One frame of `compare` as text, on part `part` (0 system, 1 tools, 2 messages).
pub fn compare_snapshot(
    without: &[u8],
    with: &[u8],
    prompt: &str,
    size: &str,
    part: usize,
) -> Result<String> {
    let (w, h) = snap::size(size)?;
    let mut view = compare::CompareView::new(without, with, prompt);
    view.part = compare::Part::ALL[part.min(2)];
    Ok(snap::text(&snap::draw(w, h, |f| {
        compare::render(f, &mut view)
    })))
}

/// Draws a bench report written by `mandrake_proxy::bench`.
pub fn bench_view(report: &Path) -> Result<()> {
    bench::run(report)
}

/// Walks the catalog item by item and returns the profile to apply, or None when the user quits.
pub fn setup_wizard(dirs: &Dirs, catalog: &Catalog, current: &Profile) -> Result<Option<Profile>> {
    wizard::run(dirs, catalog, current)
}

/// One frame of `watch` as text, from the events on disk.
pub fn watch_snapshot(dirs: &Dirs, size: &str) -> Result<String> {
    let (w, h) = snap::size(size)?;
    let mut tail = watch::Tail::new(dirs.events());
    let mut state = watch::WatchState::new(crate::fmt::home_path(tail.path(), &dirs.home));
    let polled = tail.poll()?;
    state.ingest(polled.reset, polled.events);
    Ok(snap::text(&snap::draw(w, h, |f| watch::render(f, &state))))
}

/// One frame of `hex` as text, scrolled to the `hunk`-th changed stretch (0 is the top).
pub fn hex_snapshot(dirs: &Dirs, id: Option<&str>, size: &str, hunk: usize) -> Result<String> {
    let (w, h) = snap::size(size)?;
    let id = match id {
        Some(id) => id.to_string(),
        None => hex::HexView::latest(dirs)?,
    };
    let mut view = hex::HexView::load(dirs, &id)?;
    for _ in 0..hunk {
        view.next_hunk();
    }
    Ok(snap::text(&snap::draw(w, h, |f| hex::render(f, &mut view))))
}

/// One frame of the bench view as text.
pub fn bench_snapshot(report: &Path, size: &str) -> Result<String> {
    let (w, h) = snap::size(size)?;
    let state = bench::BenchState::load(report)?;
    Ok(snap::text(&snap::draw(w, h, |f| bench::render(f, &state))))
}
