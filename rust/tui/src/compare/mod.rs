//! `mandrake compare`: one request as Claude Code sends it, without Mandrake on the left and with
//! it on the right, part by part (system prompt, tools, conversation), so the cut can be seen
//! instead of read in numbers.

mod model;
mod view;

pub use model::{Part, Request};
pub use view::render;

use crate::fmt;
use crate::term::{self, Opts, Session};
use anyhow::Result;
use crossterm::event::KeyCode;
use std::time::Duration;

pub struct CompareView {
    pub without: Request,
    pub with: Request,
    /// The prompt both requests carried, for the title.
    pub prompt: String,
    pub part: Part,
    pub scroll: u16,
    /// Rows of text on screen, set on every draw.
    pub page: u16,
}

impl CompareView {
    pub fn new(without: &[u8], with: &[u8], prompt: &str) -> Self {
        Self {
            without: Request::parse(without),
            with: Request::parse(with),
            prompt: prompt.to_string(),
            part: Part::System,
            scroll: 0,
            page: 10,
        }
    }

    /// Applies a key; false when the screen should close.
    pub fn on_key(&mut self, key: KeyCode) -> bool {
        let at = Part::ALL.iter().position(|p| *p == self.part).unwrap_or(0);
        let n = Part::ALL.len();
        match key {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Right | KeyCode::Tab | KeyCode::Char('l') => self.go(Part::ALL[(at + 1) % n]),
            KeyCode::Left | KeyCode::BackTab | KeyCode::Char('h') => {
                self.go(Part::ALL[(at + n - 1) % n])
            }
            KeyCode::Char(c @ '1'..='3') => self.go(Part::ALL[c as usize - '1' as usize]),
            KeyCode::Down | KeyCode::Char('j') => self.scroll = self.scroll.saturating_add(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll = self.scroll.saturating_sub(1),
            KeyCode::PageDown | KeyCode::Char(' ') => {
                self.scroll = self.scroll.saturating_add(self.page.max(1))
            }
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(self.page.max(1)),
            KeyCode::Home | KeyCode::Char('g') => self.scroll = 0,
            _ => {}
        }
        true
    }

    fn go(&mut self, part: Part) {
        self.part = part;
        self.scroll = 0;
    }
}

/// The comparison as a table, for a pipe or a log: one row per part, then the totals.
pub fn summary(v: &CompareView) -> String {
    let mut out = format!(
        "{:<22}{:>15}{:>15}{:>12}\n",
        "", "sem Mandrake", "com Mandrake", "diferença"
    );
    let row = |name: &str, a: u64, b: u64| {
        format!(
            "{name:<22}{:>15}{:>15}{:>12}\n",
            fmt::bytes(a),
            fmt::bytes(b),
            change(a, b)
        )
    };
    for p in Part::ALL {
        out += &row(p.label(), v.without.part(p).0, v.with.part(p).0);
    }
    out += &row("total", v.without.bytes, v.with.bytes);
    out += &format!(
        "{:<22}{:>15}{:>15}\n",
        "≈ tokens (bytes ÷ 4)",
        fmt::num(v.without.bytes / 4),
        fmt::num(v.with.bytes / 4)
    );
    out
}

/// `-33,2%`, `+1,0%` or `igual`.
pub fn change(before: u64, after: u64) -> String {
    if before == after {
        return "igual".into();
    }
    let r = fmt::saved_ratio(before, after);
    if r > 0.0 {
        format!("-{}", fmt::pct(r))
    } else {
        format!("+{}", fmt::pct(-r))
    }
}

pub fn run(view: &mut CompareView) -> Result<()> {
    let mut session = Session::enter(Opts::KEYS)?;
    loop {
        session.term().draw(|f| render(f, view))?;
        if let Some(ev) = term::next_event(Duration::from_millis(250))?
            && let Some(k) = term::pressed(&ev)
            && !view.on_key(k.code)
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests;
