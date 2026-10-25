//! Shared colors and the title and help lines, so every screen reads the same way.
//! Named ANSI colors keep contrast on both light and dark terminal themes; nothing blinks.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// The untrimmed side: requests as Claude Code sent them.
pub const PASS: Color = Color::Yellow;
/// The trimmed side.
pub const TRIM: Color = Color::Green;
pub const FG: Color = Color::White;
pub const BG: Color = Color::Black;
pub const GAUGE_BG: Color = Color::DarkGray;
/// Bytes only the "before" body has.
pub const REMOVED: Style = Style::new().fg(Color::Black).bg(Color::Red);
/// Bytes only the "after" body has.
pub const INSERTED: Style = Style::new().fg(Color::Black).bg(Color::Green);

pub fn accent() -> Style {
    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}
pub fn label() -> Style {
    Style::new().fg(Color::Cyan)
}
pub fn dim() -> Style {
    Style::new().fg(Color::Gray)
}
pub fn frame() -> Style {
    Style::new().fg(Color::Gray)
}
pub fn warn() -> Style {
    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
}
pub fn error() -> Style {
    Style::new()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD)
}
pub fn selected() -> Style {
    Style::new()
        .fg(BG)
        .bg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
}

/// Green for 2xx, red for errors, yellow for the rest.
pub fn status(code: u16, failed: bool) -> Style {
    match code {
        _ if failed => error(),
        200..=299 => Style::new().fg(Color::Green),
        400.. => error(),
        _ => Style::new().fg(Color::Yellow),
    }
}

/// ` Mandrake  <screen>  <subtitle>` on the first row.
pub fn title(screen: &str, subtitle: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            " Mandrake ",
            Style::new()
                .fg(BG)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {screen} "), accent()),
        Span::styled(subtitle.to_string(), dim()),
    ])
}

/// `key description · key description` for the last row.
pub fn help(pairs: &[(&str, &str)]) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (i, (key, what)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", dim()));
        }
        spans.push(Span::styled(key.to_string(), accent()));
        spans.push(Span::styled(format!(" {what}"), dim()));
    }
    Line::from(spans)
}
