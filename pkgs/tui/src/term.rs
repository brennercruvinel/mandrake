//! Entering and leaving the real terminal. Every screen draws into a [`Frame`] from its own state;
//! only this module touches stdout, so tests render the same screens on a `TestBackend`.

use anyhow::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{Stdout, stdout};
use std::sync::Once;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

/// How a screen takes the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opts {
    /// Draw on the alternate screen, leaving the shell scrollback untouched.
    pub alt_screen: bool,
    /// Report clicks and the wheel.
    pub mouse: bool,
}

impl Opts {
    /// A full screen of its own.
    pub const FULL: Opts = Opts {
        alt_screen: true,
        mouse: true,
    };
    /// Inside a tui-embed pane: the pane is the screen, and the host keeps the mouse.
    pub const EMBED: Opts = Opts {
        alt_screen: false,
        mouse: false,
    };
}

const ACTIVE: u8 = 1;
const ALT: u8 = 2;
const MOUSE: u8 = 4;

/// What the panic hook must undo; zero when no session is open.
static STATE: AtomicU8 = AtomicU8::new(0);
static HOOK: Once = Once::new();

/// An open terminal session; dropping it restores the terminal, and so does a panic.
pub struct Session {
    term: Term,
}

impl Session {
    pub fn enter(opts: Opts) -> Result<Self> {
        install_panic_hook();
        enable_raw_mode()?;
        let mut flags = ACTIVE;
        STATE.store(flags, Ordering::SeqCst);
        let mut out = stdout();
        if opts.alt_screen {
            execute!(out, EnterAlternateScreen)?;
            flags |= ALT;
        } else {
            execute!(out, Clear(ClearType::All))?;
        }
        if opts.mouse {
            execute!(out, EnableMouseCapture)?;
            flags |= MOUSE;
        }
        STATE.store(flags, Ordering::SeqCst);
        let mut term = Terminal::new(CrosstermBackend::new(out))?;
        term.clear()?;
        Ok(Self { term })
    }

    pub fn term(&mut self) -> &mut Term {
        &mut self.term
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.term.show_cursor();
        restore();
    }
}

/// Undoes whatever [`Session::enter`] did; safe to call twice.
fn restore() {
    let flags = STATE.swap(0, Ordering::SeqCst);
    if flags & ACTIVE == 0 {
        return;
    }
    let mut out = stdout();
    if flags & MOUSE != 0 {
        let _ = execute!(out, DisableMouseCapture);
    }
    if flags & ALT != 0 {
        let _ = execute!(out, LeaveAlternateScreen);
    }
    let _ = execute!(out, crossterm::cursor::Show);
    let _ = disable_raw_mode();
}

fn install_panic_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
    });
}

/// The next terminal event, or None when `wait` passes without one.
pub fn next_event(wait: Duration) -> Result<Option<Event>> {
    if event::poll(wait)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

/// True for a key press (crossterm also reports releases and repeats on some platforms).
pub fn pressed(ev: &Event) -> Option<&crossterm::event::KeyEvent> {
    match ev {
        Event::Key(k) if k.kind != crossterm::event::KeyEventKind::Release => Some(k),
        _ => None,
    }
}
