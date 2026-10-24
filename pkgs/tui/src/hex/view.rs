//! Draws [`HexView`]: header, two hex panes, and the slider with stretch markers.

use super::{Body, HexView, Method, ROW};
use crate::{fmt, theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use tui_slider::{Slider, SliderOrientation};

/// How the bytes of a row are spaced: one gap per byte, or one per group of four.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Density {
    Spaced,
    Grouped,
}

impl Density {
    /// Pane width including borders.
    fn width(self) -> u16 {
        match self {
            Density::Spaced => 78,
            Density::Grouped => 61,
        }
    }
}

const RAIL: u16 = 3;

pub fn render(f: &mut Frame, v: &mut HexView) {
    let [title, head, main, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(5),
        Constraint::Min(6),
        Constraint::Length(1),
    ])
    .areas(f.area());
    f.render_widget(theme::title("hex", &format!("requisição {}", v.id)), title);
    header(f, head, v);

    let [panes, rail] =
        Layout::horizontal([Constraint::Min(20), Constraint::Length(RAIL)]).areas(main);
    let w = panes.width;
    // Side by side when both panes fit, tighter bytes before giving up; stacked on narrow screens.
    let (side_by_side, density) = if w >= 2 * Density::Spaced.width() {
        (true, Density::Spaced)
    } else if w >= 2 * Density::Grouped.width() {
        (true, Density::Grouped)
    } else if w >= Density::Spaced.width() {
        (false, Density::Spaced)
    } else {
        (false, Density::Grouped)
    };
    let [a, b] = if side_by_side {
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(panes)
    } else {
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(panes)
    };
    v.page = a.height.saturating_sub(2).max(1) as usize;
    v.row = v.row.min(v.max_row());
    let (lead_row, follow_row) = (v.row, v.follower_row());
    let row_of = |which: Body| {
        if which == v.lead {
            lead_row
        } else {
            follow_row
        }
    };
    pane(f, a, v, Body::Before, row_of(Body::Before), density);
    pane(f, b, v, Body::After, row_of(Body::After), density);

    // A gap, the slider, then the stretch markers.
    let bar = Rect {
        x: rail.x + 1,
        y: rail.y + 1,
        width: 2,
        height: rail.height.saturating_sub(2),
    };
    v.slider = Rect { width: 1, ..bar };
    slider(f, bar, v);

    let at = v.row * ROW;
    f.render_widget(
        Paragraph::new(theme::help(&[
            ("q", "voltar"),
            ("↑↓ PgUp PgDn", "rolar"),
            ("n p", "próximo/anterior trecho"),
            ("Tab", "guia antes/depois"),
            (
                &format!("trecho {}/{}", v.current_hunk(), v.diff.hunks.len()),
                &format!("offset {at:#x}"),
            ),
        ])),
        help,
    );
}

fn header(f: &mut Frame, area: Rect, v: &HexView) {
    let block = Block::bordered().border_style(theme::frame());
    let inner = block.inner(area);
    f.render_widget(block, area);
    let (b, a) = (v.before.len() as u64, v.after.len() as u64);
    let k = |s: &str| Span::styled(format!("{s} "), theme::label());
    let sizes = Line::from(vec![
        k("antes"),
        Span::raw(size(b)),
        k("  depois"),
        Span::raw(size(a)),
        Span::styled(
            format!("  −{} B", fmt::num(v.diff.removed_bytes() as u64)),
            theme::REMOVED,
        ),
        Span::raw(" "),
        Span::styled(
            format!("+{} B", fmt::num(v.diff.inserted_bytes() as u64)),
            theme::INSERTED,
        ),
        Span::raw(format!("  em {} trechos", v.diff.hunks.len())),
    ]);
    let tokens = match &v.event {
        None => Line::from(Span::styled(
            format!(
                "evento {} não encontrado em events.jsonl: sem tokens nem cortes",
                v.id
            ),
            theme::dim(),
        )),
        Some(e) => {
            let t = match (e.tokens_before, e.tokens_after) {
                (Some(tb), Some(ta)) => format!(
                    "{} → {} ({}, {})",
                    fmt::num(tb),
                    fmt::num(ta),
                    fmt::signed(ta as i64 - tb as i64),
                    fmt::pct(fmt::saved_ratio(tb, ta))
                ),
                _ => "não contados".into(),
            };
            Line::from(vec![
                k("tokens"),
                Span::raw(t),
                k("  modelo"),
                Span::raw(fmt::model(e.model.as_deref())),
                k("  status"),
                Span::styled(
                    e.status.to_string(),
                    theme::status(e.status, e.error.is_some()),
                ),
                k("  modo"),
                Span::raw(format!("{:?}", e.mode).to_lowercase()),
            ])
        }
    };
    let mut cuts = vec![k("cortes"), Span::raw(cuts_summary(v))];
    if v.diff.method == Method::Lines {
        cuts.push(Span::styled(
            "  diff por linhas: o diff por bytes passou de 2 s",
            theme::warn(),
        ));
    }
    f.render_widget(Paragraph::new(vec![sizes, tokens, Line::from(cuts)]), inner);
}

/// `12,1 KB (12.345 B)`, or just `784 B` when the two would say the same.
fn size(n: u64) -> String {
    if n < 1024 {
        format!("{n} B")
    } else {
        format!("{} ({} B)", fmt::bytes(n), fmt::num(n))
    }
}

pub fn cuts_summary(v: &HexView) -> String {
    let Some(e) = &v.event else { return "-".into() };
    let c = &e.cuts;
    let mut parts = vec![];
    if c.system_blocks > 0 {
        parts.push(format!("{} bloco(s) de sistema", c.system_blocks));
    }
    if c.reminders > 0 {
        parts.push(format!("{} system-reminder(s)", c.reminders));
    }
    if !c.fields.is_empty() {
        parts.push(format!("campos {}", c.fields.join(", ")));
    }
    if parts.is_empty() {
        "nenhum".into()
    } else {
        parts.join(" · ")
    }
}

fn pane(f: &mut Frame, area: Rect, v: &HexView, which: Body, top: usize, density: Density) {
    let (name, mark) = match which {
        Body::Before => ("antes · before.json", theme::REMOVED),
        Body::After => ("depois · after.json", theme::INSERTED),
    };
    let lead = v.lead == which;
    let mut title = vec![Span::styled(format!(" {name} "), theme::accent())];
    if lead {
        title.push(Span::styled(" guia ", theme::selected()));
    }
    let block = Block::bordered()
        .border_type(if lead {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(if lead {
            Style::new().fg(Color::Cyan)
        } else {
            theme::frame()
        })
        .title(Line::from(title));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let (body, marks) = match which {
        Body::Before => (&v.before[..], &v.diff.removed[..]),
        Body::After => (&v.after[..], &v.diff.inserted[..]),
    };
    let lines: Vec<Line> = (top..)
        .take(inner.height as usize)
        .map(|r| r * ROW)
        .take_while(|&start| start < body.len() || (start == 0 && body.is_empty()))
        .map(|start| {
            let end = (start + ROW).min(body.len());
            hex_row(start, &body[start..end], &marks[start..end], mark, density)
        })
        .collect();
    f.render_widget(Paragraph::new(lines), inner);
}

/// `offset  hh hh … |ascii|` with the changed bytes in `mark`.
fn hex_row(
    offset: usize,
    bytes: &[u8],
    marks: &[bool],
    mark: Style,
    density: Density,
) -> Line<'static> {
    let off = match density {
        Density::Spaced => format!("{offset:08x}  "),
        Density::Grouped => format!("{offset:06x} "),
    };
    let mut spans = vec![Span::styled(off, theme::dim())];
    let style = |i: usize| {
        if marks.get(i).copied().unwrap_or(false) {
            mark
        } else {
            Style::new()
        }
    };
    for i in 0..ROW {
        let hex = bytes
            .get(i)
            .map(|b| format!("{b:02x}"))
            .unwrap_or_else(|| "  ".into());
        spans.push(Span::styled(hex, style(i)));
        let gap = match density {
            Density::Spaced if i == 7 => "  ",
            Density::Spaced => " ",
            Density::Grouped if i % 4 == 3 => " ",
            Density::Grouped => "",
        };
        if !gap.is_empty() {
            let joined = i + 1 < bytes.len() && style(i) == mark && style(i + 1) == mark;
            spans.push(Span::styled(gap, if joined { mark } else { Style::new() }));
        }
    }
    for (i, &b) in bytes.iter().enumerate() {
        let printable = (0x20..0x7f).contains(&b);
        let ch = if printable {
            (b as char).to_string()
        } else {
            ".".into()
        };
        let base = if printable {
            Style::new()
        } else {
            theme::dim()
        };
        spans.push(Span::styled(ch, if style(i) == mark { mark } else { base }));
    }
    Line::from(spans)
}

/// The vertical slider for the lead body, with a marker column for where stretches start.
fn slider(f: &mut Frame, bar: Rect, v: &HexView) {
    if bar.height == 0 {
        return;
    }
    let max = v.max_row().max(1) as f64;
    let value = max - v.row.min(v.max_row()) as f64;
    f.render_widget(
        Slider::new(value, 0.0, max)
            .orientation(SliderOrientation::Vertical)
            .filled_symbol("┃")
            .empty_symbol("│")
            .handle_symbol("█")
            .filled_color(Color::Cyan)
            .empty_color(Color::Gray)
            .handle_color(Color::White),
        Rect { width: 1, ..bar },
    );
    let rows = v.rows(v.lead) as f64;
    let h = bar.height as f64;
    let buf = f.buffer_mut();
    for hunk in &v.diff.hunks {
        let start = match v.lead {
            Body::Before => hunk.before.start,
            Body::After => hunk.after.start,
        };
        let y = bar.y + ((start / ROW) as f64 / rows * h).floor().min(h - 1.0) as u16;
        let removed = !hunk.before.is_empty();
        let style = Style::new().fg(if removed { Color::Red } else { Color::Green });
        buf.set_string(bar.x + 1, y, "◆", style);
    }
}
