use super::state::Focus;
use super::*;
use crate::testutil::{draw, ev, find, text};
use crossterm::event::KeyCode;
use mandrake_meter::{Mode, append};
use std::io::Write;

fn sample() -> WatchState {
    let mut s = WatchState::new("/tmp/events.jsonl".into());
    let mut audited = ev("t2", Mode::Trim, 1000, 600);
    audited.audited = true;
    s.ingest(
        false,
        vec![
            ev("p1", Mode::Pass, 1000, 1000),
            ev("t1", Mode::Trim, 1000, 800),
            ev("p2", Mode::Pass, 1200, 1200),
            audited,
        ],
    );
    s
}

#[test]
fn tail_reads_only_what_was_appended() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    let mut tail = Tail::new(path.clone());
    assert!(tail.poll().unwrap().events.is_empty());

    append(&path, &ev("a", Mode::Pass, 10, 10)).unwrap();
    append(&path, &ev("b", Mode::Trim, 10, 8)).unwrap();
    let got = tail.poll().unwrap();
    assert_eq!(
        got.events.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert!(
        tail.poll().unwrap().events.is_empty(),
        "nothing new, nothing read"
    );

    // A torn write is left for the next poll, then read whole once it ends.
    let line = serde_json::to_string(&ev("c", Mode::Trim, 5, 4)).unwrap();
    let (head, rest) = line.split_at(20);
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    f.write_all(head.as_bytes()).unwrap();
    assert!(tail.poll().unwrap().events.is_empty());
    f.write_all(format!("{rest}\nnot json\n").as_bytes())
        .unwrap();
    let got = tail.poll().unwrap();
    assert_eq!(got.events.len(), 1);
    assert_eq!(got.events[0].id, "c");

    // Truncation starts over.
    std::fs::write(&path, b"").unwrap();
    append(&path, &ev("d", Mode::Pass, 1, 1)).unwrap();
    let got = tail.poll().unwrap();
    assert!(got.reset);
    assert_eq!(got.events[0].id, "d");
}

#[test]
fn ingest_splits_by_mode_and_keeps_the_cursor() {
    let mut s = sample();
    assert_eq!(s.pass.totals.requests, 2);
    assert_eq!(s.trim.totals.requests, 2);
    assert_eq!(s.trim.selected_event().unwrap().id, "t2", "newest first");
    let (saved, ratio) = s.saved();
    assert_eq!(saved, 600);
    assert!((ratio - 0.3).abs() < 1e-9);
    let (pass, trim, delta) = s.difference().unwrap();
    assert_eq!((pass, trim, delta), (1100.0, 700.0, 400.0));

    s.focus = Focus::Trim;
    s.on_key(KeyCode::Down);
    assert_eq!(s.trim.selected_event().unwrap().id, "t1");
    s.ingest(false, vec![ev("t3", Mode::Trim, 10, 9)]);
    assert_eq!(
        s.trim.selected_event().unwrap().id,
        "t1",
        "new rows do not move the cursor"
    );
    s.on_key(KeyCode::Down);
    s.on_key(KeyCode::Down);
    assert_eq!(s.trim.selected, 2, "stops at the last row");

    s.ingest(true, vec![ev("p9", Mode::Pass, 1, 1)]);
    assert_eq!(s.trim.totals.requests, 0);
    assert!(s.difference().is_none());
}

#[test]
fn keys_switch_sides_and_open_audited_events() {
    let mut s = sample();
    assert_eq!(s.focus, Focus::Pass);
    s.on_key(KeyCode::Tab);
    assert_eq!(s.focus, Focus::Trim);
    s.on_key(KeyCode::Left);
    assert_eq!(s.focus, Focus::Pass);

    assert_eq!(s.on_key(KeyCode::Enter), Action::None);
    assert!(s.notice.as_deref().unwrap().contains("p2"));
    s.on_key(KeyCode::Right);
    assert!(s.notice.is_none(), "a key clears the notice");
    assert_eq!(s.on_key(KeyCode::Enter), Action::Hex("t2".into()));
    assert_eq!(s.on_key(KeyCode::Char('q')), Action::Quit);
}

#[test]
fn renders_both_sides_and_the_savings() {
    let s = sample();
    let buf = draw(150, 26, |f| render(f, &s));
    let screen = text(&buf);
    for needle in [
        "Sem Mandrake · pass",
        "Com Mandrake · trim",
        "2.200 → 2.200",
        "2.000 → 1.400  (30,0%)",
        "cache",
        "sonnet-4-5",
        "1.000→600",
        "com Mandrake: -400 tokens por requisição (36,4%)",
        "por requisição sem 1.100 · com 700 tokens",
        "corte do proxy +600 (30,0%); o resto sai no prompt-trim",
        "Enter hex",
    ] {
        assert!(screen.contains(needle), "missing {needle:?} in\n{screen}");
    }
    // The audited request carries the marker, on the trim side.
    let (x, _) = find(&buf, "◆").unwrap();
    assert!(x > 75, "the audit marker belongs to the right half");
}

#[test]
fn empty_file_says_where_it_waits() {
    let s = WatchState::new("/tmp/x/events.jsonl".into());
    let screen = text(&draw(120, 24, |f| render(f, &s)));
    assert!(screen.contains("aguardando requisições em"));
    assert!(screen.contains("/tmp/x/events.jsonl"));
    assert!(screen.contains("sem tokens contados"));
    assert!(screen.contains("quando os dois tiverem"));
}

#[test]
#[ignore = "prints the screen for a visual check: cargo test -p mandrake-tui -- --ignored --nocapture"]
fn print_screen() {
    let mut s = sample();
    s.focus = Focus::Trim;
    println!("{}", text(&draw(150, 22, |f| render(f, &s))));
}
