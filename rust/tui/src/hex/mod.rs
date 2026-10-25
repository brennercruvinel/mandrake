//! U2: one request's audit bodies, before and after the trim, in hex side by side.

mod diff;
#[cfg(test)]
mod tests;
mod view;

pub use diff::{BUDGET, Body, Diff, Method, diff};
pub use view::render;

use crate::term::{self, Opts, Session, Term};
use anyhow::{Context, Result, bail};
use crossterm::event::{Event as TermEvent, KeyCode, MouseButton, MouseEventKind};
use mandrake_home::Dirs;
use mandrake_meter::Event;
use ratatui::layout::Rect;
use std::time::Duration;

pub const ROW: usize = 16;
/// Rows kept above a stretch when jumping to it.
const CONTEXT: usize = 2;

pub struct HexView {
    pub id: String,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub diff: Diff,
    /// The event that names these bodies, for tokens and cuts.
    pub event: Option<Event>,
    /// The body the scroll position follows; the other one lines up with it.
    pub lead: Body,
    /// Top row of the lead body.
    pub row: usize,
    /// Rows on screen, set on every draw.
    pub page: usize,
    /// Where the slider was drawn, for clicks.
    pub slider: Rect,
}

/// Whether `audit/<id>.before.json` and `.after.json` are both there.
pub fn has_bodies(dirs: &Dirs, id: &str) -> bool {
    ["before", "after"]
        .iter()
        .all(|s| dirs.audit().join(format!("{id}.{s}.json")).is_file())
}

impl HexView {
    pub fn new(
        id: &str,
        before: Vec<u8>,
        after: Vec<u8>,
        event: Option<Event>,
        budget: Duration,
    ) -> Self {
        let diff = diff(&before, &after, budget);
        Self {
            id: id.to_string(),
            before,
            after,
            diff,
            event,
            lead: Body::Before,
            row: 0,
            page: 20,
            slider: Rect::default(),
        }
    }

    /// Reads `audit/<id>.before.json`, `.after.json` and the event with that id.
    /// The newest audited request whose two bodies are still on disk.
    pub fn latest(dirs: &Dirs) -> Result<String> {
        mandrake_meter::read(&dirs.events())?
            .into_iter()
            .rev()
            .find(|e| e.audited && has_bodies(dirs, &e.id))
            .map(|e| e.id)
            .with_context(|| {
                format!(
                    "nenhuma requisição auditada com corpo guardado; ligue [proxy] audit = true em {}, \
                     ou rode `mandrake compare`",
                    dirs.profile().display()
                )
            })
    }

    pub fn load(dirs: &Dirs, id: &str) -> Result<Self> {
        if id.is_empty() || id.contains(['/', '\\']) || id.contains("..") {
            bail!("id inválido: {id:?}");
        }
        let read = |suffix: &str| {
            let path = dirs.audit().join(format!("{id}.{suffix}.json"));
            std::fs::read(&path).map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => anyhow::anyhow!(
                    "o corpo de {id} não está mais em {}: a auditoria apaga os corpos mais velhos que \
                     audit_keep_days. `mandrake audit` lista os que existem",
                    dirs.audit().display()
                ),
                _ => anyhow::anyhow!("não deu para ler {}: {e}", path.display()),
            })
        };
        let (before, after) = (read("before")?, read("after")?);
        let event = mandrake_meter::read(&dirs.events())?
            .into_iter()
            .rev()
            .find(|e| e.id == id);
        Ok(Self::new(id, before, after, event, BUDGET))
    }

    pub fn body(&self, which: Body) -> &[u8] {
        match which {
            Body::Before => &self.before,
            Body::After => &self.after,
        }
    }

    pub fn rows(&self, which: Body) -> usize {
        self.body(which).len().div_ceil(ROW).max(1)
    }

    /// The last row the lead body can scroll to while keeping the screen full.
    pub fn max_row(&self) -> usize {
        self.rows(self.lead).saturating_sub(self.page.max(1))
    }

    /// The top row of the other body, lined up with the lead.
    pub fn follower_row(&self) -> usize {
        self.diff.map(self.row * ROW, self.lead) / ROW
    }

    fn scroll(&mut self, delta: isize) {
        self.row = self.row.saturating_add_signed(delta).min(self.max_row());
    }

    /// Hunk starts in rows of the lead body.
    fn hunk_rows(&self) -> impl Iterator<Item = usize> + '_ {
        self.diff.hunks.iter().map(|h| match self.lead {
            Body::Before => h.before.start / ROW,
            Body::After => h.after.start / ROW,
        })
    }

    /// Which stretch is at the cursor, counting from 1; 0 before the first.
    pub fn current_hunk(&self) -> usize {
        self.hunk_rows()
            .filter(|&r| r <= self.row + CONTEXT)
            .count()
    }

    pub fn next_hunk(&mut self) {
        let at = self.row + CONTEXT;
        let next = self.hunk_rows().find(|&r| r > at);
        if let Some(r) = next {
            self.row = r.saturating_sub(CONTEXT).min(self.max_row());
        }
    }

    pub fn prev_hunk(&mut self) {
        let at = self.row + CONTEXT;
        let prev = self.hunk_rows().filter(|&r| r < at).last();
        if let Some(r) = prev {
            self.row = r.saturating_sub(CONTEXT);
        }
    }

    /// Swaps which body drives the scroll, keeping the same place on screen.
    pub fn swap_lead(&mut self) {
        self.row = self.follower_row();
        self.lead = self.lead.other();
        self.row = self.row.min(self.max_row());
    }

    /// Moves to a cell of the slider, top being the start of the file.
    pub fn seek_slider(&mut self, row_in_bar: u16) {
        let h = self.slider.height.max(1);
        let frac = if h <= 1 {
            0.0
        } else {
            row_in_bar.min(h - 1) as f64 / (h - 1) as f64
        };
        self.row = (frac * self.max_row() as f64).round() as usize;
    }

    /// Returns false when the view should close.
    pub fn on_event(&mut self, ev: &TermEvent) -> bool {
        if let TermEvent::Mouse(m) = ev {
            let in_slider = self.slider.contains((m.column, m.row).into());
            match m.kind {
                MouseEventKind::ScrollDown => self.scroll(3),
                MouseEventKind::ScrollUp => self.scroll(-3),
                MouseEventKind::Down(MouseButton::Left)
                | MouseEventKind::Drag(MouseButton::Left)
                    if in_slider =>
                {
                    self.seek_slider(m.row - self.slider.y)
                }
                _ => {}
            }
            return true;
        }
        match term::pressed(ev) {
            Some(k) => self.on_key(k.code),
            None => true,
        }
    }

    pub fn on_key(&mut self, code: KeyCode) -> bool {
        let page = self.page.max(1) as isize;
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Down | KeyCode::Char('j') => self.scroll(1),
            KeyCode::Up | KeyCode::Char('k') => self.scroll(-1),
            KeyCode::PageDown | KeyCode::Char(' ') => self.scroll(page),
            KeyCode::PageUp => self.scroll(-page),
            KeyCode::Home | KeyCode::Char('g') => self.row = 0,
            KeyCode::End | KeyCode::Char('G') => self.row = self.max_row(),
            KeyCode::Char('n') => self.next_hunk(),
            KeyCode::Char('p') => self.prev_hunk(),
            KeyCode::Tab => self.swap_lead(),
            _ => {}
        }
        true
    }
}

/// Runs the view on a terminal another screen already holds, until `q` or `Esc`.
pub fn run_loop(term: &mut Term, view: &mut HexView) -> Result<()> {
    loop {
        term.draw(|f| render(f, view))?;
        if let Some(ev) = term::next_event(Duration::from_millis(250))?
            && !view.on_event(&ev)
        {
            return Ok(());
        }
    }
}

pub fn run(dirs: &Dirs, id: &str) -> Result<()> {
    let mut view = HexView::load(dirs, id)?;
    let mut session = Session::enter(Opts::FULL)?;
    run_loop(session.term(), &mut view)
}
