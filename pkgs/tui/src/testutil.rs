//! Synthetic events and a text dump of a rendered frame, shared by the screen tests.

use mandrake_meter::{Cuts, Event, Mode, Usage};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

pub fn ev(id: &str, mode: Mode, before: u64, after: u64) -> Event {
    Event {
        id: id.into(),
        ts: "2026-10-24T13:05:09.120Z".into(),
        mode,
        path: "/v1/messages".into(),
        model: Some("claude-sonnet-4-5-20250929".into()),
        status: 200,
        ms: 1234,
        bytes_before: before * 4,
        bytes_after: after * 4,
        tokens_before: Some(before),
        tokens_after: Some(after),
        usage: Some(Usage {
            input: 10,
            output: 50,
            cache_read: after,
            cache_create: 5,
        }),
        cuts: Cuts {
            system_blocks: 1,
            reminders: 2,
            fields: vec!["metadata".into()],
        },
        audited: false,
        error: None,
    }
}

/// Draws once on a `width` x `height` test terminal and returns the buffer.
pub fn draw(width: u16, height: u16, render: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
    t.draw(render).unwrap();
    t.backend().buffer().clone()
}

/// The buffer as text, one line per row, trailing spaces trimmed.
pub fn text(buf: &Buffer) -> String {
    let area = buf.area;
    (0..area.height)
        .map(|y| {
            let row: String = (0..area.width).map(|x| buf[(x, y)].symbol()).collect();
            row.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Position of the first cell where `needle` starts.
pub fn find(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    let lines = text(buf);
    lines.lines().enumerate().find_map(|(y, l)| {
        let byte = l.find(needle)?;
        Some((l[..byte].chars().count() as u16, y as u16))
    })
}
