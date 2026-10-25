//! Draws [`WatchState`] into a frame.

use super::state::{Focus, Side, WatchState};
use crate::fmt;
use crate::theme;
use mandrake_meter::Event;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Cell, LineGauge, Paragraph, Row, Table, TableState, Wrap,
};

pub fn render(f: &mut Frame, s: &WatchState) {
    let [title, main, footer, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(8),
        Constraint::Length(6),
        Constraint::Length(1),
    ])
    .areas(f.area());
    f.render_widget(
        theme::title("watch", "requisições ao vivo, relidas a cada 0,5 s"),
        title,
    );
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(main);
    side(f, left, s, Focus::Pass);
    side(f, right, s, Focus::Trim);
    savings(f, footer, s);
    let help_line = match &s.notice {
        Some(n) => Line::from(Span::styled(format!(" {n}"), theme::warn())),
        None => theme::help(&[
            ("q", "sair"),
            ("Tab ←→", "lado"),
            ("↑↓", "requisição"),
            ("Enter", "hex (auditada ◆)"),
        ]),
    };
    f.render_widget(Paragraph::new(help_line), help);
}

fn side(f: &mut Frame, area: Rect, s: &WatchState, which: Focus) {
    let (label, accent) = match which {
        Focus::Pass => (" Sem Mandrake · pass ", theme::PASS),
        Focus::Trim => (" Com Mandrake · trim ", theme::TRIM),
    };
    let focused = s.focus == which;
    let block = Block::bordered()
        .border_type(if focused {
            BorderType::Thick
        } else {
            BorderType::Plain
        })
        .border_style(if focused {
            Style::new().fg(accent)
        } else {
            theme::frame()
        })
        .title(Line::from(Span::styled(
            label,
            Style::new().fg(accent).bold(),
        )));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [head, list] = Layout::vertical([Constraint::Length(5), Constraint::Min(2)]).areas(inner);
    let data = s.side(which);
    f.render_widget(Paragraph::new(header(data)), head);
    if data.events.is_empty() {
        let how = match which {
            // Nothing passes this side unless a second proxy, without the cut, is running.
            Focus::Pass => {
                "vazio: este lado mostra o Claude Code sem o Mandrake e só enche com \
                 `mandrake proxy run --pass` (como, em docs/guide/measure.md)"
            }
            Focus::Trim => "vazio: abra uma sessão com `claude` num shell com o Mandrake",
        };
        let msg = vec![
            Line::from(Span::styled(how, theme::dim())),
            Line::from(""),
            Line::from(Span::styled("lendo", theme::dim())),
            Line::from(Span::styled(s.source.clone(), theme::dim())),
        ];
        f.render_widget(Paragraph::new(msg).wrap(Wrap { trim: true }), list);
        return;
    }
    // Under 52 columns (80x24 split in two), the model and the milliseconds give way.
    let wide = list.width >= 52;
    let rows = data.newest_first().map(|e| row(e, wide));
    let (widths, names): (Vec<Constraint>, Vec<&str>) = if wide {
        (
            vec![
                Constraint::Length(8),
                Constraint::Min(8),
                Constraint::Length(6),
                Constraint::Min(13),
                Constraint::Length(7),
                Constraint::Length(3),
            ],
            vec!["hora UTC", "modelo", "status", "tokens", "ms", "aud"],
        )
    } else {
        (
            vec![
                Constraint::Length(8),
                Constraint::Length(6),
                Constraint::Min(11),
                Constraint::Length(3),
            ],
            vec!["hora UTC", "status", "tokens", "aud"],
        )
    };
    let table = Table::new(rows, widths)
        .header(Row::new(names.into_iter().map(Cell::from)).style(theme::label()))
        .column_spacing(1)
        .row_highlight_style(if focused {
            Style::new()
                .fg(theme::BG)
                .bg(accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        });
    let mut state = TableState::default().with_selected(Some(data.selected));
    f.render_stateful_widget(table, list, &mut state);
}

fn header(d: &Side) -> Vec<Line<'static>> {
    let t = &d.totals;
    let kv = |k: &str, v: String| {
        Line::from(vec![
            Span::styled(format!(" {k:<12}"), theme::label()),
            Span::raw(v),
        ])
    };
    let tokens = if d.counted == 0 {
        "sem contagem".to_string()
    } else {
        format!(
            "{} → {}  ({})",
            fmt::num(t.tokens_before),
            fmt::num(t.tokens_after),
            fmt::pct(t.saved_ratio())
        )
    };
    vec![
        kv("requisições", fmt::num(t.requests)),
        kv("tokens", tokens),
        kv(
            "bytes",
            format!(
                "{} → {}",
                fmt::bytes(t.bytes_before),
                fmt::bytes(t.bytes_after)
            ),
        ),
        kv(
            "usage",
            format!(
                "entrada {} · saída {}",
                fmt::num(t.usage.input),
                fmt::num(t.usage.output)
            ),
        ),
        kv(
            "cache",
            format!(
                "lido {} · criado {}",
                fmt::num(t.usage.cache_read),
                fmt::num(t.usage.cache_create)
            ),
        ),
    ]
}

fn row(e: &Event, wide: bool) -> Row<'static> {
    let tokens = match (e.tokens_before, e.tokens_after) {
        (Some(b), Some(a)) if a == b => fmt::num(b),
        (Some(b), Some(a)) => format!("{}→{}", fmt::num(b), fmt::num(a)),
        (Some(b), None) => fmt::num(b),
        _ => "-".into(),
    };
    let status = Cell::from(e.status.to_string()).style(theme::status(e.status, e.error.is_some()));
    let clock = Cell::from(fmt::clock(&e.ts).to_string());
    let audited = Cell::from(if e.audited { " ◆" } else { "" }).style(theme::accent());
    if !wide {
        return Row::new(vec![clock, status, Cell::from(tokens), audited]);
    }
    Row::new(vec![
        clock,
        Cell::from(fmt::model(e.model.as_deref())),
        status,
        Cell::from(tokens),
        Cell::from(Line::from(fmt::num(e.ms)).right_aligned()),
        audited,
    ])
}

/// "economizou 98 tokens", or "cresceu 12 tokens" when the cut made it bigger: never a sign.
fn saving(n: i64) -> String {
    match n {
        n if n < 0 => format!("cresceu {} tokens", fmt::num(n.unsigned_abs())),
        n => format!("economizou {} tokens", fmt::num(n as u64)),
    }
}

fn savings(f: &mut Frame, area: Rect, s: &WatchState) {
    let block = Block::bordered()
        .border_style(theme::frame())
        .title(Span::styled(" Economia ", theme::accent()));
    let inner = block.inner(area);
    f.render_widget(block, area);
    let [head, gauge, per, proxy] = Layout::vertical([Constraint::Length(1); 4]).areas(inner);
    let (saved, ratio) = s.saved();
    // With both sides counted, the headline is the A/B: everything Mandrake removes, the plugin's
    // cut included, which happens before the request and the proxy alone cannot see.
    let ab = s.difference().filter(|(pass, _, _)| *pass > 0.0);
    let (label, ratio) = match ab {
        Some((pass, _, delta)) => (
            format!(
                "com Mandrake, cada requisição {} ({})",
                saving(delta.round() as i64),
                fmt::pct(delta / pass)
            ),
            (delta / pass).clamp(0.0, 1.0),
        ),
        None if s.trim.counted == 0 => (
            "sem tokens contados no lado com Mandrake ainda".to_string(),
            ratio,
        ),
        None => (
            format!(
                "o proxy {} ({})",
                saving(saved),
                fmt::pct(s.trim.totals.saved_ratio())
            ),
            ratio,
        ),
    };
    // The words above the bar, in the text color: over a filled bar they lose contrast.
    f.render_widget(
        Paragraph::new(Span::styled(
            format!(" {label}"),
            Style::new().fg(theme::FG).bold(),
        )),
        head,
    );
    f.render_widget(
        LineGauge::default()
            .ratio(ratio)
            .label("")
            .filled_style(Style::new().fg(theme::TRIM).add_modifier(Modifier::BOLD))
            .unfilled_style(Style::new().fg(theme::GAUGE_BG))
            .filled_symbol("█")
            .unfilled_symbol("─"),
        gauge,
    );
    let (line, proxy_line) = match ab {
        Some((pass, trim, _)) => {
            let mut spans = vec![
                Span::styled(" por requisição ", theme::label()),
                Span::styled(
                    format!("sem {}", fmt::num(pass.round() as u64)),
                    Style::new().fg(theme::PASS),
                ),
                Span::raw(" · "),
                Span::styled(
                    format!("com {}", fmt::num(trim.round() as u64)),
                    Style::new().fg(theme::TRIM),
                ),
                Span::raw(" tokens"),
            ];
            if let (Some(up), Some(ut)) = (s.pass.usage_per_request(), s.trim.usage_per_request()) {
                spans.push(Span::styled("  usage ", theme::label()));
                spans.push(Span::raw(format!(
                    "{} vs {}",
                    fmt::num(up.round() as u64),
                    fmt::num(ut.round() as u64)
                )));
            }
            let proxy_line = Line::from(vec![
                Span::styled(" o proxy ", theme::label()),
                Span::raw(format!(
                    "{} ({}); o resto sai no prompt-trim, antes da requisição",
                    saving(saved),
                    fmt::pct(s.trim.totals.saved_ratio())
                )),
            ]);
            (Line::from(spans), proxy_line)
        }
        None => (
            Line::from(Span::styled(
                " a diferença entre os lados aparece quando os dois",
                theme::dim(),
            )),
            Line::from(Span::styled(" tiverem requisições contadas", theme::dim())),
        ),
    };
    f.render_widget(Paragraph::new(line), per);
    f.render_widget(Paragraph::new(proxy_line), proxy);
}
