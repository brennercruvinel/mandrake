use super::state::{Presence, Step};
use super::*;
use crate::testutil::{draw, text};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mandrake_home::{Action, LinkSpec};
use mandrake_layout::Group;
use std::path::Path;

fn dirs(root: &Path) -> Dirs {
    let root = root.to_path_buf();
    Dirs::resolve(move |k| match k {
        "HOME" => Some(root.clone()),
        "MANDRAKE_HOME" => Some(root.join("mh")),
        "CLAUDE_CONFIG_DIR" => Some(root.join("claude")),
        _ => None,
    })
    .unwrap()
}

fn current() -> Profile {
    let mut p = Profile {
        dev_root: Some("~/dev".into()),
        ..Default::default()
    };
    p.links.insert(
        "skills".into(),
        LinkSpec {
            action: Action::Link,
            target: Some("agent/skills".into()),
        },
    );
    p.links.insert(
        "agents/tiamati.md".into(),
        LinkSpec {
            action: Action::Link,
            target: Some("agent/t.md".into()),
        },
    );
    p
}

/// A wizard over the built-in catalog and a temp Claude folder with a file and a link in it.
fn setup() -> (tempfile::TempDir, Wizard) {
    let tmp = tempfile::tempdir().unwrap();
    let d = dirs(tmp.path());
    std::fs::create_dir_all(&d.claude).unwrap();
    std::fs::write(d.claude.join("CLAUDE.md"), "hi").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("/elsewhere/skills", d.claude.join("skills")).unwrap();
    let w = Wizard::new(&d, &Catalog::builtin().unwrap(), &current());
    (tmp, w)
}

fn key(w: &mut Wizard, code: KeyCode) -> Outcome {
    w.on_key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn keys(w: &mut Wizard, codes: &[KeyCode]) {
    for &c in codes {
        assert_eq!(key(w, c), Outcome::Continue, "{c:?} on {:?}", w.step);
    }
}

fn typed(w: &mut Wizard, s: &str) {
    for c in s.chars() {
        key(w, KeyCode::Char(c));
    }
}

fn go_to(w: &mut Wizard, step: Step) {
    while w.step != step {
        assert_eq!(key(w, KeyCode::Tab), Outcome::Continue);
        assert!(w.notice.is_none(), "{:?}", w.notice);
    }
}

fn select(w: &mut Wizard, path: &str) {
    w.item = w.items.iter().position(|i| i.path == path).unwrap();
}

#[test]
fn walks_forward_and_back_through_every_step() {
    let (_t, mut w) = setup();
    assert_eq!(w.step, Step::DevRoot);
    assert_eq!(
        w.dev_root.value(),
        "~/dev",
        "prefilled from the current profile"
    );
    typed(&mut w, "/q");
    assert_eq!(w.dev_root.value(), "~/dev/q", "q types in a text field");
    key(&mut w, KeyCode::Backspace);
    key(&mut w, KeyCode::Backspace);
    for want in [
        Step::Items,
        Step::Plugins,
        Step::Proxy,
        Step::Souls,
        Step::Summary,
    ] {
        key(&mut w, KeyCode::Enter);
        assert_eq!(w.step, want);
    }
    key(&mut w, KeyCode::BackTab);
    assert_eq!(w.step, Step::Souls);
    for _ in 0..10 {
        key(&mut w, KeyCode::BackTab);
    }
    assert_eq!(w.step, Step::DevRoot, "stops at the first step");
    assert_eq!(key(&mut w, KeyCode::Esc), Outcome::Quit);
}

#[test]
fn items_are_grouped_and_prefilled() {
    let (_t, w) = setup();
    let groups: Vec<Group> = w.items.iter().map(|i| i.group).collect();
    let first_session = groups.iter().position(|&g| g == Group::Session).unwrap();
    assert!(groups[..first_session].iter().all(|&g| g != Group::Session));
    assert!(
        groups[first_session..].iter().all(|&g| g == Group::Session),
        "grouped, config first"
    );

    let skills = w.items.iter().find(|i| i.path == "skills").unwrap();
    assert_eq!(
        (skills.action, skills.target.value()),
        (Action::Link, "agent/skills")
    );
    let rules = w.items.iter().find(|i| i.path == "rules").unwrap();
    assert_eq!(
        (rules.action, rules.target.value()),
        (Action::Keep, "claude/rules"),
        "suggested"
    );
    assert_eq!(
        w.resolved(rules).unwrap(),
        w.dirs.home.join("dev/claude/rules").display().to_string()
    );

    let claude_md = w.items.iter().find(|i| i.path == "CLAUDE.md").unwrap();
    assert_eq!(claude_md.present, Presence::File);
    #[cfg(unix)]
    assert_eq!(skills.present, Presence::Link("/elsewhere/skills".into()));
    assert_eq!(
        w.items.iter().find(|i| i.path == "hooks").unwrap().present,
        Presence::Missing
    );
}

#[test]
fn choosing_actions_and_editing_targets() {
    let (_t, mut w) = setup();
    go_to(&mut w, Step::Items);
    select(&mut w, "rules");
    keys(&mut w, &[KeyCode::Right]);
    assert_eq!(w.selected().unwrap().action, Action::Link);
    keys(&mut w, &[KeyCode::Right]);
    assert_eq!(w.selected().unwrap().action, Action::Copy);
    keys(
        &mut w,
        &[
            KeyCode::Right,
            KeyCode::Left,
            KeyCode::Char('m'),
            KeyCode::Char('l'),
        ],
    );
    assert_eq!(w.selected().unwrap().action, Action::Link);

    keys(&mut w, &[KeyCode::Char('e'), KeyCode::End]);
    typed(&mut w, "-x");
    assert_eq!(
        w.step,
        Step::Items,
        "Enter and q do not leave while editing"
    );
    keys(&mut w, &[KeyCode::Enter]);
    assert_eq!(w.selected().unwrap().target.value(), "claude/rules-x");

    keys(&mut w, &[KeyCode::Char('e')]);
    typed(&mut w, "zzz");
    keys(&mut w, &[KeyCode::Esc]);
    assert_eq!(
        w.selected().unwrap().target.value(),
        "claude/rules-x",
        "Esc undoes the edit"
    );
    assert_eq!(w.step, Step::Items, "and does not quit");

    keys(&mut w, &[KeyCode::Up]);
    assert_ne!(w.selected().unwrap().path, "rules");
}

#[test]
fn sensitive_items_warn() {
    let (_t, mut w) = setup();
    go_to(&mut w, Step::Items);
    select(&mut w, "CLAUDE.md");
    assert!(w.warning().is_none());
    select(&mut w, "projects");
    assert!(w.warning().unwrap().contains("guarda conversas"));
    let screen = text(&draw(130, 30, |f| render(f, &w)));
    assert!(screen.contains("guarda conversas"), "{screen}");
    key(&mut w, KeyCode::Char('l'));
    let change = w
        .changes()
        .into_iter()
        .find(|c| c.what == "projects")
        .unwrap();
    assert!(change.sensitive);
    assert_eq!(change.after, "linkar claude/projects");
}

#[test]
fn prompt_trim_is_locked_on() {
    let (_t, mut w) = setup();
    go_to(&mut w, Step::Plugins);
    keys(&mut w, &[KeyCode::Char(' ')]);
    assert!(w.notice.as_deref().unwrap().contains("obrigatório"));
    keys(
        &mut w,
        &[
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Down,
            KeyCode::Down,
            KeyCode::Char(' '),
        ],
    );
    assert!(w.fancy && w.tui_embed);
    let screen = text(&draw(120, 20, |f| render(f, &w)));
    assert!(
        screen.contains("[x] prompt-trim") && screen.contains("travado"),
        "{screen}"
    );
}

#[test]
fn proxy_safeguards_off_by_default_and_validated() {
    let (_t, mut w) = setup();
    go_to(&mut w, Step::Proxy);
    assert!(!w.strip_safeguards);
    let screen = text(&draw(130, 20, |f| render(f, &w)));
    assert!(
        screen.contains("classificador de segurança da API"),
        "{screen}"
    );

    keys(
        &mut w,
        &[
            KeyCode::Backspace,
            KeyCode::Backspace,
            KeyCode::Backspace,
            KeyCode::Backspace,
        ],
    );
    keys(&mut w, &[KeyCode::Enter]);
    assert_eq!(w.step, Step::Proxy, "an empty port blocks");
    assert!(w.notice.as_deref().unwrap().contains("porta"));
    typed(&mut w, "9100");
    keys(
        &mut w,
        &[
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Down,
        ],
    );
    keys(&mut w, &[KeyCode::Backspace]);
    typed(&mut w, "30");
    assert_eq!(
        key(&mut w, KeyCode::Char('q')),
        Outcome::Quit,
        "digits only, so q still quits"
    );
}

#[test]
fn relative_targets_need_a_dev_root() {
    let (_t, mut w) = setup();
    w.dev_root.set("");
    go_to(&mut w, Step::Items);
    keys(&mut w, &[KeyCode::Enter]);
    assert_eq!(w.step, Step::Items);
    assert!(
        w.notice.as_deref().unwrap().contains("skills"),
        "{:?}",
        w.notice
    );
}

#[test]
fn confirming_returns_the_profile_and_quitting_returns_nothing() {
    let (_t, mut w) = setup();
    w.dev_root.set("~/code");
    go_to(&mut w, Step::Items);
    select(&mut w, "rules");
    keys(&mut w, &[KeyCode::Char('c')]);
    select(&mut w, "skills");
    keys(&mut w, &[KeyCode::Char('m')]);
    go_to(&mut w, Step::Plugins);
    keys(&mut w, &[KeyCode::Down, KeyCode::Char(' ')]);
    go_to(&mut w, Step::Proxy);
    keys(
        &mut w,
        &[
            KeyCode::Down,
            KeyCode::Char(' '),
            KeyCode::Down,
            KeyCode::Char(' '),
        ],
    );
    go_to(&mut w, Step::Souls);
    typed(&mut w, "~/souls");
    keys(&mut w, &[KeyCode::Down, KeyCode::Char(' '), KeyCode::Down]);
    typed(&mut w, "mem.sh find");
    go_to(&mut w, Step::Summary);

    let summary = text(&draw(130, 30, |f| render(f, &w)));
    for needle in [
        "pasta de desenvolvimento",
        "~/dev → ~/code",
        "rules",
        "copiar claude/rules",
        "plugin fancy",
        "safeguards",
    ] {
        assert!(summary.contains(needle), "missing {needle:?} in\n{summary}");
    }
    assert!(
        !w.changes().iter().any(|c| c.what == "plugin tui-embed"),
        "unchanged lines stay out"
    );

    let Outcome::Done(p) = key(&mut w, KeyCode::Enter) else {
        panic!("Enter confirms")
    };
    assert_eq!(p.dev_root.as_deref(), Some("~/code"));
    assert_eq!(
        p.links["rules"],
        LinkSpec {
            action: Action::Copy,
            target: Some("claude/rules".into())
        }
    );
    assert_eq!(
        p.links["skills"],
        LinkSpec {
            action: Action::Keep,
            target: None
        },
        "was in the profile"
    );
    assert!(
        !p.links.contains_key("hooks"),
        "untouched keep entries stay out"
    );
    assert!(
        p.links.contains_key("agents/tiamati.md"),
        "entries outside the catalog carry over"
    );
    assert!(p.plugins.fancy && !p.plugins.tui_embed);
    assert_eq!(
        p.proxy.strip_fields,
        vec!["metadata".to_string(), "safeguards".to_string()]
    );
    assert!(p.proxy.audit);
    assert_eq!(p.souls.dir.as_deref(), Some("~/souls"));
    assert!(p.skills.invoke);
    assert_eq!(p.skills.memory_cmd.as_deref(), Some("mem.sh find"));

    let (_t, mut w) = setup();
    w.first = false; // a machine that already has a saved profile
    go_to(&mut w, Step::Summary);
    assert!(text(&draw(120, 20, |f| render(f, &w))).contains("Nenhuma mudança"));
    assert_eq!(key(&mut w, KeyCode::Char('q')), Outcome::Quit);
}

#[test]
fn every_step_renders_on_small_and_large_screens() {
    let (_t, mut w) = setup();
    for size in [(80, 24), (160, 40)] {
        w.step = Step::DevRoot;
        loop {
            let screen = text(&draw(size.0, size.1, |f| render(f, &w)));
            assert!(
                screen.contains(w.step.title()),
                "{:?} at {size:?}:\n{screen}",
                w.step
            );
            if w.step == Step::Summary {
                break;
            }
            key(&mut w, KeyCode::Tab);
        }
    }
}

#[test]
#[ignore = "prints the screens for a visual check: cargo test -p mandrake-tui -- --ignored --nocapture"]
fn print_screens() {
    let (_t, mut w) = setup();
    w.step = Step::Items;
    select(&mut w, "projects");
    for step in [
        Step::DevRoot,
        Step::Items,
        Step::Plugins,
        Step::Proxy,
        Step::Souls,
        Step::Summary,
    ] {
        w.step = step;
        println!("{}\n", text(&draw(120, 22, |f| render(f, &w))));
    }
}

#[test]
fn a_first_setup_lists_what_it_writes_instead_of_no_change() {
    let (_t, mut w) = setup();
    assert!(w.first, "no saved profile in the fixture");
    go_to(&mut w, Step::Summary);
    let screen = text(&draw(120, 20, |f| render(f, &w)));
    assert!(screen.contains("Primeira instalação"), "{screen}");
    assert!(!screen.contains("Nenhuma mudança"), "{screen}");
    assert!(
        screen.contains("settings.json") && screen.contains("127.0.0.1:"),
        "{screen}"
    );
}
