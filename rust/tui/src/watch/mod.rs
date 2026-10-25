//! U1 and U5: the live split view over `events.jsonl`, full screen or inside a tui-embed pane.

mod state;
mod tail;
#[cfg(test)]
mod tests;
mod view;

pub use state::{Action, WatchState};
pub use tail::Tail;
pub use view::render;

use crate::term::{self, Opts, Session};
use anyhow::Result;
use mandrake_home::Dirs;
use std::time::{Duration, Instant};

/// How often the events file is read again.
pub const TICK: Duration = Duration::from_millis(500);

pub fn run(dirs: &Dirs, embed: bool) -> Result<()> {
    let mut tail = Tail::new(dirs.events());
    let mut state = WatchState::new(crate::fmt::home_path(tail.path(), &dirs.home));
    let polled = tail.poll()?;
    state.ingest(polled.reset, polled.events);
    let opts = if embed { Opts::EMBED } else { Opts::FULL };
    let mut session = Session::enter(opts)?;
    let mut last = Instant::now();
    loop {
        let size = session.term().draw(|f| render(f, &state))?.area;
        if let Some(ev) = term::next_event(TICK.saturating_sub(last.elapsed()))? {
            match state.on_event(&ev, size.width) {
                Action::Quit => return Ok(()),
                Action::Hex(id) => {
                    state.notice = Some(format!("calculando o diff de {id}…"));
                    session.term().draw(|f| render(f, &state))?;
                    state.notice = match crate::hex::HexView::load(dirs, &id) {
                        Ok(mut view) => crate::hex::run_loop(session.term(), &mut view)
                            .err()
                            .map(|e| e.to_string()),
                        Err(e) => Some(format!("{e:#}")),
                    };
                }
                Action::None => {}
            }
        }
        if last.elapsed() >= TICK {
            let polled = tail.poll()?;
            state.ingest(polled.reset, polled.events);
            last = Instant::now();
        }
    }
}
