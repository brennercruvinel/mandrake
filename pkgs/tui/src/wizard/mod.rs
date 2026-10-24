//! U4: the setup wizard, full screen, step by step. It only returns a profile; applying it is the
//! caller's job.

mod field;
mod profile;
mod state;
#[cfg(test)]
mod tests;
mod view;

pub use state::{Outcome, Wizard};
pub use view::render;

use crate::term::{self, Opts, Session};
use anyhow::Result;
use mandrake_home::{Dirs, Profile};
use mandrake_layout::Catalog;
use std::time::Duration;

pub fn run(dirs: &Dirs, catalog: &Catalog, current: &Profile) -> Result<Option<Profile>> {
    let mut wizard = Wizard::new(dirs, catalog, current);
    // No mouse capture: the wizard is all keys, and the terminal keeps text selection.
    let mut session = Session::enter(Opts {
        alt_screen: true,
        mouse: false,
    })?;
    loop {
        session.term().draw(|f| render(f, &wizard))?;
        let Some(ev) = term::next_event(Duration::from_millis(250))? else {
            continue;
        };
        let Some(k) = term::pressed(&ev) else {
            continue;
        };
        match wizard.on_key(*k) {
            Outcome::Continue => {}
            Outcome::Quit => return Ok(None),
            Outcome::Done(p) => return Ok(Some(*p)),
        }
    }
}
