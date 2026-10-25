use super::*;
use crate::testutil::{draw, ev, find, text};
use crossterm::event::KeyCode;
use mandrake_meter::Mode;
use ratatui::style::Color;
use std::path::PathBuf;

/// Two request bodies that differ in one stretch: a system block of `cut` bytes in the middle.
fn bodies(cut: usize) -> (Vec<u8>, Vec<u8>) {
    let head = format!(
        r#"{{"model":"claude-sonnet-4-5","messages":[{}],"#,
        r#"{"role":"user","content":"hi"},"#.repeat(12)
    );
    let block = format!(
        r#""system":[{{"type":"text","text":"{}"}}],"#,
        "X".repeat(cut)
    );
    let tail = r#""max_tokens":1024,"stream":true}"#;
    (
        format!("{head}{block}{tail}").into_bytes(),
        format!("{head}{tail}").into_bytes(),
    )
}

fn view() -> HexView {
    let (before, after) = bodies(300);
    let mut e = ev("req1", Mode::Trim, 1500, 1200);
    e.audited = true;
    HexView::new("req1", before, after, Some(e), BUDGET)
}

pub fn dirs(root: &std::path::Path) -> Dirs {
    let root = root.to_path_buf();
    Dirs::resolve(move |k| match k {
        "HOME" => Some(root.clone()),
        "MANDRAKE_HOME" => Some(root.join("mh")),
        "CLAUDE_CONFIG_DIR" => Some(root.join("claude")),
        _ => None::<PathBuf>,
    })
    .unwrap()
}

#[test]
fn header_tells_sizes_tokens_and_cuts() {
    let mut v = view();
    let screen = text(&draw(170, 30, |f| render(f, &mut v)));
    let removed = v.before.len() - v.after.len();
    for needle in [
        format!("antes {} B", v.before.len()),
        format!("−{} B", removed),
        "+0 B".into(),
        "em 1 trechos".into(),
        "1.500 → 1.200 (-300, 20,0%)".into(),
        "1 bloco(s) de sistema · 2 system-reminder(s) · campos metadata".into(),
        "antes · before.json".into(),
        "depois · after.json".into(),
        "trecho 0/1".into(),
    ] {
        assert!(screen.contains(&needle), "missing {needle:?} in\n{screen}");
    }
    assert!(
        screen.contains("00000000  7b 22 6d 6f"),
        "offset, then bytes: {screen}"
    );
    assert!(screen.contains(r#"{"model":"claude"#), "the ASCII column");
}

#[test]
fn removed_bytes_are_red_on_the_left_only() {
    let mut v = view();
    v.next_hunk();
    let buf = draw(170, 30, |f| render(f, &mut v));
    // Pane rows start below the title, the header and the pane border.
    let cells = |xs: std::ops::Range<u16>, bg: Color| {
        xs.flat_map(|x| (7..29).map(move |y| (x, y)))
            .filter(|&p| buf[p].bg == bg)
            .count()
    };
    let start = v.diff.hunks[0].before.start;
    let (ox, oy) =
        find(&buf, &format!("{:08x}", start / ROW * ROW)).expect("the cut is on screen after `n`");
    assert!(ox < 85, "the row with the cut is in the left pane");
    assert!(
        (ox..ox + 76).any(|x| buf[(x, oy)].bg == Color::Red),
        "that row has red bytes"
    );
    assert!(cells(0..85, Color::Red) > 0);
    assert_eq!(
        cells(85..167, Color::Red),
        0,
        "after.json has nothing removed"
    );
    assert_eq!(cells(0..170, Color::Green), 0, "nothing was inserted");
    // The right pane follows the left one: its top row is the counterpart of the left top row.
    let fr = v.follower_row() * ROW;
    let ascii: String = v.after[fr..(fr + ROW).min(v.after.len())]
        .iter()
        .map(|&b| b as char)
        .collect();
    let right: Vec<String> = text(&buf)
        .lines()
        .map(|l| l.chars().skip(85).collect())
        .collect();
    assert!(
        right.iter().any(|l| l.contains(&ascii)),
        "the follower row is drawn on the right"
    );
}

#[test]
#[ignore = "prints the screens for a visual check: cargo test -p mandrake-tui -- --ignored --nocapture"]
fn print_screens() {
    let mut v = view();
    v.next_hunk();
    println!("{}", text(&draw(170, 24, |f| render(f, &mut v))));
    println!("{}", text(&draw(130, 24, |f| render(f, &mut v))));
}

#[test]
fn keys_scroll_and_jump_between_stretches() {
    let mut v = view();
    let _ = draw(170, 24, |f| render(f, &mut v));
    let page = v.page;
    let hunk_row = v.diff.hunks[0].before.start / ROW;
    assert!(v.max_row() > 0);

    v.on_key(KeyCode::Char('n'));
    assert_eq!(v.row, hunk_row - 2, "lands two rows above the stretch");
    assert_eq!(v.current_hunk(), 1);
    v.on_key(KeyCode::Char('n'));
    assert_eq!(v.row, hunk_row - 2, "no stretch after the last one");
    v.on_key(KeyCode::Char('p'));
    assert_eq!(v.row, hunk_row - 2, "already on the first stretch");

    v.on_key(KeyCode::Home);
    v.on_key(KeyCode::PageDown);
    assert_eq!(v.row, page.min(v.max_row()));
    v.on_key(KeyCode::End);
    assert_eq!(v.row, v.max_row());
    v.on_key(KeyCode::Down);
    assert_eq!(v.row, v.max_row(), "clamped at the end");
    v.on_key(KeyCode::PageUp);
    v.on_key(KeyCode::Up);
    assert_eq!(v.row, v.max_row().saturating_sub(page + 1));

    v.seek_slider(0);
    assert_eq!(v.row, 0);
    v.seek_slider(v.slider.height - 1);
    assert_eq!(v.row, v.max_row());
    assert!(!v.on_key(KeyCode::Char('q')));
}

#[test]
fn tab_swaps_the_lead_keeping_the_place() {
    let mut v = view();
    let _ = draw(170, 24, |f| render(f, &mut v));
    v.on_key(KeyCode::End);
    let follower = v.follower_row();
    v.on_key(KeyCode::Tab);
    assert_eq!(v.lead, Body::After);
    assert_eq!(v.row, follower.min(v.max_row()));
    let screen = text(&draw(170, 24, |f| render(f, &mut v)));
    assert!(screen.contains("depois · after.json  guia"));
}

#[test]
fn narrow_terminals_stack_the_panes() {
    let mut v = view();
    let buf = draw(90, 40, |f| render(f, &mut v));
    let (_, before_y) = find(&buf, "antes · before.json").unwrap();
    let (_, after_y) = find(&buf, "depois · after.json").unwrap();
    assert!(after_y > before_y + 5, "after sits below before");
}

#[test]
fn load_reads_audit_bodies_and_the_event() {
    let tmp = tempfile::tempdir().unwrap();
    let d = dirs(tmp.path());
    let (before, after) = bodies(40);
    std::fs::create_dir_all(d.audit()).unwrap();
    std::fs::write(d.audit().join("r7.before.json"), &before).unwrap();
    std::fs::write(d.audit().join("r7.after.json"), &after).unwrap();
    mandrake_meter::append(&d.events(), &ev("r7", Mode::Trim, 90, 80)).unwrap();

    let v = HexView::load(&d, "r7").unwrap();
    assert_eq!(v.before, before);
    assert_eq!(v.event.as_ref().unwrap().tokens_after, Some(80));
    assert_eq!(v.diff.hunks.len(), 1);

    let err = HexView::load(&d, "nope").err().unwrap().to_string();
    assert!(err.contains("nope.before.json"), "{err}");
    assert!(HexView::load(&d, "../r7").is_err());
    assert!(HexView::load(&d, "").is_err());
}
