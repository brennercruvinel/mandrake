//! Drawing the comparison: totals on top, one tab per part, the two bodies side by side.

use super::{CompareView, Part, change};
use crate::{fmt, theme};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

pub fn render(f: &mut Frame, v: &mut CompareView) {
    let [title, totals, tabs, body, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .areas(f.area());
    f.render_widget(
        Paragraph::new(theme::title("compare", &format!("\"{}\"", v.prompt))),
        title,
    );
    f.render_widget(Paragraph::new(total_lines(v)), totals);
    f.render_widget(Paragraph::new(tab_line(v)), tabs);
    let [left, right] =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(body);
    v.page = body.height.saturating_sub(2);
    let (a, ta) = v.without.part(v.part);
    let (b, tb) = v.with.part(v.part);
    let count = |n: usize| match v.part {
        Part::Tools => format!(" · {n} ferramentas"),
        _ => String::new(),
    };
    for (area, side, bytes, text, color, extra) in [
        (
            left,
            "sem Mandrake",
            a,
            ta,
            theme::PASS,
            count(v.without.tools),
        ),
        (
            right,
            "com Mandrake",
            b,
            tb,
            theme::TRIM,
            count(v.with.tools),
        ),
    ] {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(theme::frame())
            .title(Span::styled(
                format!(" {side} · {}{extra} ", fmt::bytes(bytes)),
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ));
        let text = if text.is_empty() { "(vazio)" } else { text };
        f.render_widget(
            Paragraph::new(text.to_string())
                .block(block)
                .wrap(Wrap { trim: false })
                .scroll((v.scroll, 0)),
            area,
        );
    }
    f.render_widget(
        Paragraph::new(theme::help(&[
            ("←/→", "parte"),
            ("↑/↓ PgUp/PgDn", "rolar"),
            ("q", "sair"),
        ])),
        help,
    );
}

fn total_lines(v: &CompareView) -> Vec<Line<'static>> {
    let (a, b) = (v.without.bytes, v.with.bytes);
    let side = |label: &str, bytes: u64, color| {
        vec![
            Span::styled(
                format!(" {label} "),
                Style::new().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{} ≈ {} tokens",
                fmt::bytes(bytes),
                fmt::num(bytes / 4)
            )),
        ]
    };
    let mut first = side("sem Mandrake", a, theme::PASS);
    first.push(Span::styled("   →  ", theme::dim()));
    first.extend(side("com Mandrake", b, theme::TRIM));
    first.push(Span::styled(
        format!("   {}", change(a, b)),
        theme::accent(),
    ));
    let second = match a.cmp(&b) {
        std::cmp::Ordering::Greater => format!(
            " O Mandrake corta {} de cada mensagem enviada · tokens estimados: bytes ÷ 4",
            fmt::bytes(a - b)
        ),
        std::cmp::Ordering::Less => format!(
            " Com o Mandrake o pedido fica {} maior: o core pesa mais que os cortes.",
            fmt::bytes(b - a)
        ),
        std::cmp::Ordering::Equal => {
            " Os dois pedidos têm o mesmo tamanho: o prompt-trim não cortou nada.".to_string()
        }
    };
    vec![Line::from(first), Line::styled(second, theme::dim())]
}

fn tab_line(v: &CompareView) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (i, p) in Part::ALL.into_iter().enumerate() {
        let (a, b) = (v.without.part(p).0, v.with.part(p).0);
        let text = format!(
            " {} {} {} → {} ",
            i + 1,
            p.label(),
            fmt::bytes(a),
            fmt::bytes(b)
        );
        let style = if p == v.part {
            theme::selected()
        } else {
            theme::label()
        };
        spans.push(Span::styled(text, style));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}
