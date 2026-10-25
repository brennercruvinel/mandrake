//! U3: a bench report as a table per fixture and savings bars.

use crate::term::{self, Opts, Session};
use crate::{fmt, theme};
use anyhow::{Context, Result, bail};
use crossterm::event::KeyCode;
use mandrake_meter::{BenchReport, BenchRow};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Bar, BarChart, BarGroup, Block, Cell, Paragraph, Row, Table, TableState};
use std::path::Path;
use std::time::Duration;

pub struct BenchState {
    pub report: BenchReport,
    pub selected: usize,
}

/// What a row saved, by the measure the report can back: tokens when counted, bytes otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Saving {
    pub ratio: f64,
    pub by_tokens: bool,
}

pub fn saving(row: &BenchRow, counted: bool) -> Saving {
    match (counted, row.tokens_before, row.tokens_after) {
        (true, Some(b), Some(a)) => Saving {
            ratio: fmt::saved_ratio(b, a),
            by_tokens: true,
        },
        _ => Saving {
            ratio: fmt::saved_ratio(row.bytes_before, row.bytes_after),
            by_tokens: false,
        },
    }
}

impl BenchState {
    pub fn new(report: BenchReport) -> Self {
        Self {
            report,
            selected: 0,
        }
    }

    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            bail!(
                "ainda não há medição. `mandrake bench --fixtures` mede o corte do proxy sem gastar \
                 requisições; `mandrake bench` mede de verdade, com 2 requisições curtas por prompt"
            );
        }
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("lendo o relatório {}", path.display()))?;
        let report = serde_json::from_str(&text)
            .with_context(|| format!("o relatório {} não é um BenchReport", path.display()))?;
        Ok(Self::new(report))
    }

    /// Returns false when the view should close.
    pub fn on_key(&mut self, code: KeyCode) -> bool {
        let last = self.report.rows.len().saturating_sub(1);
        match code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Down | KeyCode::Char('j') => self.selected = (self.selected + 1).min(last),
            KeyCode::Up | KeyCode::Char('k') => self.selected = self.selected.saturating_sub(1),
            KeyCode::Home => self.selected = 0,
            KeyCode::End => self.selected = last,
            _ => {}
        }
        true
    }

    /// The sum of every row, as a row of its own.
    pub fn total(&self) -> BenchRow {
        let rows = &self.report.rows;
        let sum = |f: fn(&BenchRow) -> Option<u64>| -> Option<u64> { rows.iter().map(f).sum() };
        BenchRow {
            name: "total".into(),
            bytes_before: rows.iter().map(|r| r.bytes_before).sum(),
            bytes_after: rows.iter().map(|r| r.bytes_after).sum(),
            tokens_before: sum(|r| r.tokens_before),
            tokens_after: sum(|r| r.tokens_after),
            cuts: Default::default(),
        }
    }
}

pub fn render(f: &mut Frame, s: &BenchState) {
    let r = &s.report;
    let notice = if r.counted { 0 } else { 1 };
    // rows, the total, a two-line header and the borders
    let table_h = (r.rows.len() as u16 + 5).min(f.area().height / 2);
    let [title, warn, table, chart, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(notice),
        Constraint::Length(table_h),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .areas(f.area());
    let subtitle = if r.kind == "ab" {
        format!(
            "{} prompts reais, cada um sem e com Mandrake · {}",
            r.rows.len(),
            r.ts
        )
    } else {
        format!(
            "{} corpos fixos, só o corte do proxy · {}",
            r.rows.len(),
            r.ts
        )
    };
    f.render_widget(theme::title("bench", &subtitle), title);
    if !r.counted {
        f.render_widget(
            Paragraph::new(Span::styled(
                " ⚠ sem credencial: só bytes foram medidos, a economia é em bytes",
                theme::warn(),
            )),
            warn,
        );
    }
    if r.rows.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(" o relatório não tem fixtures", theme::dim())),
            table,
        );
    } else {
        rows_table(f, table, s);
        bars(f, chart, s);
    }
    f.render_widget(
        Paragraph::new(theme::help(&[("q", "sair"), ("↑↓", "fixture")])),
        help,
    );
}

fn opt(n: Option<u64>) -> String {
    n.map(fmt::num).unwrap_or_else(|| "-".into())
}

/// The text of each column for one row, the name first.
fn texts(row: &BenchRow, counted: bool) -> Vec<String> {
    let bytes_saved = fmt::saved_ratio(row.bytes_before, row.bytes_after);
    let tokens_saved = match (counted, row.tokens_before, row.tokens_after) {
        (true, Some(b), Some(a)) => fmt::pct(fmt::saved_ratio(b, a)),
        _ => "-".into(),
    };
    let tokens = |t| if counted { opt(t) } else { "-".into() };
    vec![
        row.name.clone(),
        fmt::num(row.bytes_before),
        fmt::num(row.bytes_after),
        fmt::pct(bytes_saved),
        tokens(row.tokens_before),
        tokens(row.tokens_after),
        tokens_saved,
    ]
}

fn cells(texts: Vec<String>) -> Vec<Cell<'static>> {
    texts
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            if i == 0 {
                Cell::from(t)
            } else {
                Cell::from(Line::from(t).right_aligned())
            }
        })
        .collect()
}

/// Nine cells fit 80 columns; a longer number widens its column instead of losing digits.
fn widths(rows: &[Vec<String>]) -> Vec<Constraint> {
    let mut w = vec![Constraint::Min(10)];
    w.extend((1..7).map(|c| {
        let longest = rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0);
        Constraint::Length(longest.max(9) as u16)
    }));
    w
}

fn rows_table(f: &mut Frame, area: ratatui::layout::Rect, s: &BenchState) {
    let counted = s.report.counted;
    // Two lines, the unit over the column, so every word stays whole at 80 columns.
    let (name, before, after) = if s.report.kind == "ab" {
        ("prompt", "sem", "com")
    } else {
        ("fixture", "antes", "depois")
    };
    let header = [
        ("", name),
        ("bytes", before),
        ("bytes", after),
        ("bytes", "economia"),
        ("tokens", before),
        ("tokens", after),
        ("tokens", "economia"),
    ];
    let mut all: Vec<Vec<String>> = s.report.rows.iter().map(|r| texts(r, counted)).collect();
    all.push(texts(&s.total(), counted));
    let widths = widths(&all);
    let last = all.len() - 1;
    let rows: Vec<Row> = all
        .into_iter()
        .enumerate()
        .map(|(i, t)| {
            let row = Row::new(cells(t));
            if i == last {
                row.style(Style::new().add_modifier(Modifier::BOLD))
            } else {
                row
            }
        })
        .collect();
    let table = Table::new(rows, widths)
        .header(
            Row::new(header.iter().enumerate().map(|(i, &(unit, h))| {
                let lines = [Line::from(unit), Line::from(h)];
                Cell::from(Text::from(if i == 0 {
                    lines.to_vec()
                } else {
                    lines.map(|l| l.right_aligned()).to_vec()
                }))
            }))
            .height(2)
            .style(theme::label()),
        )
        .block(
            Block::bordered()
                .border_style(theme::frame())
                .title(Span::styled(
                    if s.report.kind == "ab" {
                        " Por prompt: sem e com Mandrake "
                    } else {
                        " Por fixture "
                    },
                    theme::accent(),
                )),
        )
        .row_highlight_style(theme::selected());
    let mut state = TableState::default().with_selected(Some(s.selected));
    f.render_stateful_widget(table, area, &mut state);
}

fn bars(f: &mut Frame, area: ratatui::layout::Rect, s: &BenchState) {
    let counted = s.report.counted;
    let tokens_rows = s
        .report
        .rows
        .iter()
        .filter(|r| saving(r, counted).by_tokens)
        .count();
    let mixed = tokens_rows > 0 && tokens_rows < s.report.rows.len();
    let name_w = s
        .report
        .rows
        .iter()
        .map(|r| r.name.chars().count())
        .max()
        .unwrap_or(0);
    let bars: Vec<Bar> = s
        .report
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let sv = saving(r, counted);
            let color = if i == s.selected {
                Color::Cyan
            } else {
                theme::TRIM
            };
            let unit = if mixed && !sv.by_tokens {
                " (bytes)"
            } else {
                ""
            };
            // The figure goes in the label, so a short bar never hides it.
            Bar::default()
                .label(Line::from(format!(
                    "{:<name_w$} {:>6}{unit}",
                    r.name,
                    fmt::pct(sv.ratio)
                )))
                .value((sv.ratio.clamp(0.0, 1.0) * 1000.0).round() as u64)
                .text_value(String::new())
                .style(Style::new().fg(color))
        })
        .collect();
    let title = match (tokens_rows, mixed) {
        (0, _) => " Economia por linha, em bytes ",
        (_, true) => " Economia por linha, em tokens (bytes onde não houve contagem) ",
        _ => " Economia por linha, em tokens ",
    };
    f.render_widget(
        BarChart::default()
            .direction(Direction::Horizontal)
            .data(BarGroup::default().bars(&bars))
            .max(1000)
            .bar_width(1)
            .bar_gap(0)
            .label_style(Style::new().fg(theme::FG))
            .block(
                Block::bordered()
                    .border_style(theme::frame())
                    .title(Span::styled(title, theme::accent())),
            ),
        area,
    );
}

pub fn run(report: &Path) -> Result<()> {
    let mut state = BenchState::load(report)?;
    let mut session = Session::enter(Opts::KEYS)?;
    loop {
        session.term().draw(|f| render(f, &state))?;
        if let Some(ev) = term::next_event(Duration::from_millis(250))?
            && let Some(k) = term::pressed(&ev)
            && !state.on_key(k.code)
        {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{draw, text};
    use mandrake_meter::Cuts;

    fn row(name: &str, bytes: (u64, u64), tokens: Option<(u64, u64)>) -> BenchRow {
        BenchRow {
            name: name.into(),
            bytes_before: bytes.0,
            bytes_after: bytes.1,
            tokens_before: tokens.map(|t| t.0),
            tokens_after: tokens.map(|t| t.1),
            cuts: Cuts::default(),
        }
    }

    fn report(counted: bool) -> BenchReport {
        let t = |b, a| counted.then_some((b, a));
        BenchReport {
            ts: "2026-10-24T12:00:00Z".into(),
            counted,
            rows: vec![
                row("first-turn", (40_000, 30_000), t(10_000, 7_500)),
                row("long-session", (200_000, 180_000), t(50_000, 45_000)),
            ],
            kind: String::new(),
        }
    }

    #[test]
    fn counted_report_shows_tokens_and_bars() {
        let s = BenchState::new(report(true));
        let screen = text(&draw(120, 24, |f| render(f, &s)));
        for needle in [
            "first-turn",
            "long-session",
            "10.000",
            "7.500",
            "25,0%",
            "total",
            "60.000",
            "52.500",
            "em tokens",
        ] {
            assert!(screen.contains(needle), "missing {needle:?} in\n{screen}");
        }
        assert!(!screen.contains("só bytes"));
        assert!(
            screen.contains("▏") || screen.contains("█"),
            "bars are drawn"
        );
    }

    #[test]
    fn uncounted_report_warns_and_measures_bytes() {
        let mut s = BenchState::new(report(false));
        let screen = text(&draw(120, 24, |f| render(f, &s)));
        assert!(screen.contains("só bytes foram medidos"));
        assert!(screen.contains("em bytes"));
        assert!(screen.contains("first-turn    25,0%"));
        let sv = saving(&s.report.rows[1], false);
        assert!(!sv.by_tokens && (sv.ratio - 0.1).abs() < 1e-9);
        assert!(s.on_key(KeyCode::Down));
        assert!(s.on_key(KeyCode::Down));
        assert_eq!(s.selected, 1);
        assert!(!s.on_key(KeyCode::Char('q')));
    }

    #[test]
    fn a_total_over_ten_million_keeps_every_digit() {
        let mut r = report(false);
        r.rows = vec![
            row("a", (6_000_000, 5_000_000), None),
            row("b", (6_345_678, 5_000_000), None),
        ];
        let s = BenchState::new(r);
        let screen = text(&draw(80, 24, |f| render(f, &s)));
        assert!(screen.contains("12.345.678"), "{screen}");
        assert!(screen.contains("10.000.000"), "{screen}");
    }

    #[test]
    fn load_reads_json_and_explains_bad_files() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("latest.json");
        std::fs::write(&p, serde_json::to_string(&report(true)).unwrap()).unwrap();
        assert_eq!(BenchState::load(&p).unwrap().report.rows.len(), 2);
        std::fs::write(&p, "{}").unwrap();
        assert!(
            BenchState::load(&p)
                .err()
                .unwrap()
                .to_string()
                .contains("BenchReport")
        );
        assert!(BenchState::load(&dir.path().join("none.json")).is_err());
    }

    #[test]
    fn at_80_columns_every_header_word_is_whole() {
        for kind in ["", "ab"] {
            let mut r = report(false);
            r.kind = kind.into();
            let screen = text(&draw(80, 24, |f| render(f, &BenchState::new(r))));
            let words = |l: &str| {
                l.split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .map(String::from)
                    .collect::<Vec<_>>()
            };
            let all: Vec<String> = screen.lines().flat_map(words).collect();
            let want: &[&str] = if kind == "ab" {
                &["prompt", "sem", "com", "economia", "bytes", "tokens"]
            } else {
                &["fixture", "antes", "depois", "economia", "bytes", "tokens"]
            };
            for w in want {
                assert!(
                    all.iter().any(|a| a == w),
                    "{w} whole at 80 columns:\n{screen}"
                );
            }
            for cut in ["ytes", "okens", "ntes", "epois", "conomia"] {
                assert!(
                    !all.iter().any(|a| a == cut),
                    "{cut} is a cut header:\n{screen}"
                );
            }
            assert!(screen.contains("a economia é em bytes"), "{screen}");
        }
    }

    #[test]
    fn no_report_says_how_to_measure() {
        let dir = tempfile::tempdir().unwrap();
        let e = BenchState::load(&dir.path().join("latest.json"))
            .err()
            .unwrap();
        let msg = format!("{e:#}");
        assert!(
            msg.contains("bench --fixtures") && !msg.contains("os error"),
            "{msg}"
        );
    }

    #[test]
    #[ignore = "prints the screen for a visual check: cargo test -p mandrake-tui -- --ignored --nocapture"]
    fn print_screen() {
        println!(
            "{}",
            text(&draw(110, 20, |f| render(
                f,
                &BenchState::new(report(false))
            )))
        );
    }
}
