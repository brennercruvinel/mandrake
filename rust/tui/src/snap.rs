//! One frame of a screen as plain text, for `--snapshot`: evidence in a PR, a doc or a log,
//! where an interactive terminal is not available.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

/// Draws once on a `width` x `height` off-screen terminal and returns the buffer.
pub fn draw(width: u16, height: u16, render: impl FnOnce(&mut ratatui::Frame)) -> Buffer {
    let mut t = Terminal::new(TestBackend::new(width, height)).expect("test backend");
    t.draw(render).expect("draw on test backend");
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

/// `120x40` into (120, 40).
pub fn size(spec: &str) -> anyhow::Result<(u16, u16)> {
    let (w, h) = spec
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("--snapshot wants WIDTHxHEIGHT, like 120x40"))?;
    Ok((w.parse()?, h.parse()?))
}
