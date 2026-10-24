//! What the watch screen knows and how keys change it; no terminal here.

use crossterm::event::{Event as TermEvent, KeyCode, MouseEventKind};
use mandrake_meter::{Event, Mode, Totals};

/// One column of the split: every event of one mode, oldest first.
#[derive(Debug, Default)]
pub struct Side {
    pub events: Vec<Event>,
    pub totals: Totals,
    /// Requests that carry both token counts.
    pub counted: u64,
    /// Requests the API answered with a usage block.
    pub with_usage: u64,
    /// Index into the newest-first list.
    pub selected: usize,
}

impl Side {
    fn push(&mut self, new: Vec<Event>) {
        if new.is_empty() {
            return;
        }
        // Keep the same request under the cursor, unless it was following the newest one.
        if self.selected > 0 {
            self.selected += new.len();
        }
        for e in &new {
            self.counted += u64::from(e.tokens_before.is_some() && e.tokens_after.is_some());
            self.with_usage += u64::from(e.usage.is_some());
        }
        self.events.extend(new);
        self.totals = Totals::of(&self.events);
    }

    /// The events newest first, as the list shows them.
    pub fn newest_first(&self) -> impl Iterator<Item = &Event> {
        self.events.iter().rev()
    }

    pub fn selected_event(&self) -> Option<&Event> {
        self.newest_first().nth(self.selected)
    }

    /// Tokens sent per counted request.
    pub fn tokens_per_request(&self) -> Option<f64> {
        (self.counted > 0).then(|| self.totals.tokens_after as f64 / self.counted as f64)
    }

    /// Input the API billed per request: fresh input plus cache read and cache creation.
    pub fn usage_per_request(&self) -> Option<f64> {
        let u = &self.totals.usage;
        (self.with_usage > 0)
            .then(|| (u.input + u.cache_read + u.cache_create) as f64 / self.with_usage as f64)
    }

    fn step(&mut self, delta: isize) {
        let last = self.events.len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Pass,
    Trim,
}

/// What the loop should do after an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    None,
    Quit,
    /// Open the hex view of this event id.
    Hex(String),
}

#[derive(Debug, Default)]
pub struct WatchState {
    pub pass: Side,
    pub trim: Side,
    pub focus: Focus,
    /// Shown in the footer until the next key.
    pub notice: Option<String>,
    /// Where the events come from, for the empty state.
    pub source: String,
}

/// Rows a page key moves.
const PAGE: isize = 10;

impl WatchState {
    pub fn new(source: String) -> Self {
        Self {
            source,
            ..Default::default()
        }
    }

    /// Adds newly read events; `reset` drops what was there first.
    pub fn ingest(&mut self, reset: bool, events: Vec<Event>) {
        if reset {
            self.pass = Side::default();
            self.trim = Side::default();
        }
        let (pass, trim): (Vec<_>, Vec<_>) = events.into_iter().partition(|e| e.mode == Mode::Pass);
        self.pass.push(pass);
        self.trim.push(trim);
    }

    pub fn side(&self, focus: Focus) -> &Side {
        match focus {
            Focus::Pass => &self.pass,
            Focus::Trim => &self.trim,
        }
    }

    fn focused_mut(&mut self) -> &mut Side {
        match self.focus {
            Focus::Pass => &mut self.pass,
            Focus::Trim => &mut self.trim,
        }
    }

    /// Tokens the trim saved so far and the share of what it received.
    pub fn saved(&self) -> (i64, f64) {
        let t = &self.trim.totals;
        let saved = t.tokens_before as i64 - t.tokens_after as i64;
        (saved, t.saved_ratio().clamp(0.0, 1.0))
    }

    /// Tokens per request without Mandrake minus with it, when both sides have counted requests.
    pub fn difference(&self) -> Option<(f64, f64, f64)> {
        let pass = self.pass.tokens_per_request()?;
        let trim = self.trim.tokens_per_request()?;
        Some((pass, trim, pass - trim))
    }

    pub fn on_event(&mut self, ev: &TermEvent, width: u16) -> Action {
        match ev {
            TermEvent::Mouse(m) => {
                match m.kind {
                    MouseEventKind::ScrollDown => self.focused_mut().step(1),
                    MouseEventKind::ScrollUp => self.focused_mut().step(-1),
                    MouseEventKind::Down(_) => {
                        self.focus = if m.column < width / 2 {
                            Focus::Pass
                        } else {
                            Focus::Trim
                        };
                    }
                    _ => {}
                }
                Action::None
            }
            _ => match crate::term::pressed(ev) {
                Some(k) => self.on_key(k.code),
                None => Action::None,
            },
        }
    }

    pub fn on_key(&mut self, code: KeyCode) -> Action {
        self.notice = None;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            KeyCode::Tab | KeyCode::BackTab => {
                self.focus = match self.focus {
                    Focus::Pass => Focus::Trim,
                    Focus::Trim => Focus::Pass,
                }
            }
            KeyCode::Left => self.focus = Focus::Pass,
            KeyCode::Right => self.focus = Focus::Trim,
            KeyCode::Up | KeyCode::Char('k') => self.focused_mut().step(-1),
            KeyCode::Down | KeyCode::Char('j') => self.focused_mut().step(1),
            KeyCode::PageUp => self.focused_mut().step(-PAGE),
            KeyCode::PageDown => self.focused_mut().step(PAGE),
            KeyCode::Home => self.focused_mut().selected = 0,
            KeyCode::End => self.focused_mut().step(isize::MAX),
            KeyCode::Enter => return self.open(),
            _ => {}
        }
        Action::None
    }

    fn open(&mut self) -> Action {
        match self.side(self.focus).selected_event() {
            None => Action::None,
            Some(e) if e.audited => Action::Hex(e.id.clone()),
            Some(e) => {
                self.notice = Some(format!(
                    "{} não tem corpos de auditoria (ligue a auditoria no perfil)",
                    e.id
                ));
                Action::None
            }
        }
    }
}
