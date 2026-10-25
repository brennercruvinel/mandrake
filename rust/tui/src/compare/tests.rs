use super::*;
use crate::snap::text;
use crossterm::event::KeyCode;

const WITHOUT: &[u8] = br#"{"system":[{"type":"text","text":"You are Claude Code, Anthropic's official CLI for Claude. Long default prompt about tone, tools and tasks."}],
"tools":[{"name":"Read","description":"Reads a file from the local filesystem. Many paragraphs.","input_schema":{}}],
"messages":[{"role":"user","content":[{"type":"text","text":"<system-reminder>today is a day</system-reminder>"},{"type":"text","text":"Responda so: ok"}]}]}"#;
const WITH: &[u8] = br#"{"system":[{"type":"text","text":"Core: read before editing."}],
"tools":[{"name":"Read","input_schema":{}}],
"messages":[{"role":"user","content":[{"type":"text","text":"Responda so: ok"}]}]}"#;

fn draw(v: &mut CompareView, w: u16, h: u16) -> String {
    text(&crate::snap::draw(w, h, |f| render(f, v)))
}

#[test]
fn both_sides_show_the_system_prompt_first() {
    let mut v = CompareView::new(WITHOUT, WITH, "Responda só: ok");
    let s = draw(&mut v, 100, 20);
    assert!(s.contains("sem Mandrake"), "{s}");
    assert!(s.contains("You are Claude Code"), "{s}");
    assert!(s.contains("Core: read before editing."), "{s}");
    assert!(
        s.contains(&change(WITHOUT.len() as u64, WITH.len() as u64)),
        "{s}"
    );
}

#[test]
fn keys_move_between_parts_and_quit() {
    let mut v = CompareView::new(WITHOUT, WITH, "ok");
    assert!(v.on_key(KeyCode::Right));
    assert_eq!(v.part, Part::Tools);
    let s = draw(&mut v, 100, 20);
    assert!(s.contains("● Read  (descrição"), "{s}");
    assert!(s.contains("● Read  (sem descrição"), "{s}");
    v.on_key(KeyCode::Char('3'));
    assert_eq!(v.part, Part::Messages);
    v.on_key(KeyCode::Left);
    v.on_key(KeyCode::Left);
    assert_eq!(v.part, Part::System);
    assert!(!v.on_key(KeyCode::Char('q')));
}

#[test]
fn fits_eighty_columns() {
    let mut v = CompareView::new(WITHOUT, WITH, "Responda só: ok");
    let s = draw(&mut v, 80, 24);
    assert!(s.contains(" com Mandrake"), "{s}");
    assert!(s.contains("q sair"), "{s}");
}

#[test]
fn summary_has_a_row_per_part_and_the_total() {
    let v = CompareView::new(WITHOUT, WITH, "ok");
    let s = summary(&v);
    for label in [
        "system prompt",
        "ferramentas",
        "mensagens",
        "total",
        "≈ tokens",
    ] {
        assert!(s.contains(label), "{label}: {s}");
    }
    assert_eq!(change(100, 100), "igual");
    assert_eq!(change(100, 75), "-25,0%");
    assert_eq!(change(100, 110), "+10,0%");
}

#[test]
fn a_larger_request_with_mandrake_is_not_called_the_same_size() {
    let mut bigger = CompareView::new(WITH, WITHOUT, "ok");
    let s = draw(&mut bigger, 120, 20);
    assert!(s.contains("Com o Mandrake o pedido fica"), "{s}");
    assert!(!s.contains("mesmo tamanho"), "{s}");
    let mut same = CompareView::new(WITH, WITH, "ok");
    assert!(draw(&mut same, 120, 20).contains("mesmo tamanho"));
}
