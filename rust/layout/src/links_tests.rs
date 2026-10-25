use super::*;

struct T {
    _tmp: tempfile::TempDir,
    dirs: Dirs,
    dev: PathBuf,
}

fn setup() -> T {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    let dirs = Dirs::resolve(|k| match k {
        "HOME" => Some(root.join("home")),
        "MANDRAKE_HOME" => Some(root.join("mh")),
        "CLAUDE_CONFIG_DIR" => Some(root.join("home/.claude")),
        _ => None,
    })
    .unwrap();
    let dev = root.join("dev");
    std::fs::create_dir_all(dev.join("devops/rules")).unwrap();
    std::fs::create_dir_all(dev.join("agent/skills")).unwrap();
    std::fs::write(dev.join("devops/settings.json"), "{}").unwrap();
    std::fs::create_dir_all(&dirs.claude).unwrap();
    T {
        _tmp: tmp,
        dirs,
        dev,
    }
}

fn link(target: &str) -> LinkSpec {
    LinkSpec {
        action: Action::Link,
        target: Some(target.into()),
    }
}

fn profile(t: &T) -> Profile {
    let mut p = Profile {
        dev_root: Some(t.dev.to_string_lossy().into()),
        ..Default::default()
    };
    p.links
        .insert("settings.json".into(), link("devops/settings.json"));
    p.links.insert("rules".into(), link("devops/rules"));
    p.links.insert("skills".into(), link("agent/skills"));
    p.links
        .insert("agents/toth.md".into(), link("agent/toth.md"));
    p
}

#[test]
fn plan_apply_is_idempotent_and_rollback_restores() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    // A real settings.json and a link to elsewhere are in the way.
    std::fs::write(t.dirs.claude.join("settings.json"), "{\"old\":1}").unwrap();
    symlink(Path::new("/elsewhere/rules"), &t.dirs.claude.join("rules")).unwrap();
    let p = profile(&t);

    let steps = plan(&t.dirs, &cat, &p).unwrap();
    let op = |k: &str| steps.iter().find(|s| s.path == k).unwrap().op;
    assert_eq!(op("settings.json"), Op::BackupLink);
    assert_eq!(op("rules"), Op::Relink);
    assert_eq!(op("skills"), Op::Link);
    assert_eq!(op("agents/toth.md"), Op::Link);
    assert_eq!(op("projects"), Op::None);

    let j = apply(&t.dirs, &cat, &steps).unwrap().unwrap();
    assert!(j.exists());
    assert_eq!(
        inspect(&t.dirs.claude.join("skills")),
        Found::Link(t.dev.join("agent/skills"))
    );
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("settings.json")).unwrap(),
        "{}"
    );
    // A second plan has nothing to do.
    assert!(
        plan(&t.dirs, &cat, &p)
            .unwrap()
            .iter()
            .all(|s| !s.op.changes())
    );

    assert_eq!(rollback(&t.dirs).unwrap().steps, 4);
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("settings.json")).unwrap(),
        "{\"old\":1}"
    );
    assert_eq!(
        inspect(&t.dirs.claude.join("rules")),
        Found::Link("/elsewhere/rules".into())
    );
    assert_eq!(inspect(&t.dirs.claude.join("skills")), Found::Missing);
    assert!(rollback(&t.dirs).is_err(), "nothing left to roll back");
}

#[test]
fn capture_round_trips_through_apply() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    let p = profile(&t);
    apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap()).unwrap();
    // The CLI's own links inside its folder stay out of the profile.
    std::fs::create_dir_all(t.dirs.claude.join("debug")).unwrap();
    symlink(
        &t.dirs.claude.join("debug/x.txt"),
        &t.dirs.claude.join("debug/latest"),
    )
    .unwrap();
    symlink(Path::new("x.txt"), &t.dirs.claude.join("debug/rel")).unwrap();
    let c = capture(&t.dirs).unwrap();
    assert_eq!(
        c.dev_root.as_deref(),
        Some(t.dev.to_string_lossy().as_ref())
    );
    assert_eq!(c.links, p.links);
}

#[test]
fn keys_cannot_leave_the_folder() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    let mut p = profile(&t);
    p.links.insert("../escape".into(), link("x"));
    assert!(plan(&t.dirs, &cat, &p).is_err());
}

fn copy_spec(target: &str) -> LinkSpec {
    LinkSpec {
        action: Action::Copy,
        target: Some(target.into()),
    }
}

fn only(t: &T, key: &str, spec: LinkSpec) -> Profile {
    let mut p = Profile {
        dev_root: Some(t.dev.to_string_lossy().into()),
        ..Default::default()
    };
    p.links.insert(key.into(), spec);
    p
}

fn journals(t: &T) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(t.dirs.journal())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn a_copy_that_fails_midway_is_rolled_back() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::create_dir_all(t.dirs.claude.join("rules")).unwrap();
    std::fs::write(t.dirs.claude.join("rules/mine.md"), "mine").unwrap();
    // The source has a good file and a dangling link: the copy starts, then stops on the link.
    std::fs::write(t.dev.join("devops/rules/a.md"), "a").unwrap();
    symlink(
        Path::new("/nowhere/at/all"),
        &t.dev.join("devops/rules/z-broken"),
    )
    .unwrap();
    let p = only(&t, "rules", copy_spec("devops/rules"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert_eq!(
        steps.iter().find(|s| s.path == "rules").unwrap().op,
        Op::BackupCopy
    );

    let err = apply(&t.dirs, &cat, &steps).unwrap_err();
    assert!(
        format!("{err:#}").contains("mandrake link rollback"),
        "{err:#}"
    );
    let jfile = t.dirs.journal().join(&journals(&t)[0]);
    let j: Journal = serde_json::from_slice(&std::fs::read(jfile).unwrap()).unwrap();
    assert!(
        j.done[0].pending,
        "the step is in the journal before it ran"
    );
    assert_eq!(
        inspect(&t.dirs.claude.join("rules/mine.md")),
        Found::Missing
    );

    assert_eq!(rollback(&t.dirs).unwrap().steps, 1);
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("rules/mine.md")).unwrap(),
        "mine"
    );
    assert_eq!(inspect(&t.dirs.claude.join("rules/a.md")), Found::Missing);
}

#[test]
fn a_pending_step_that_never_moved_the_original_leaves_it_alone() {
    let t = setup();
    std::fs::create_dir_all(t.dirs.journal()).unwrap();
    std::fs::write(t.dirs.claude.join("settings.json"), "{\"mine\":1}").unwrap();
    let j = Journal {
        stamp: "x".into(),
        claude: t.dirs.claude.clone(),
        done: vec![Done {
            path: "settings.json".into(),
            before: Found::File,
            op: Op::BackupLink,
            backup: Some(t.dirs.backups().join("x/settings.json")),
            pending: true,
            seeded: None,
        }],
    };
    write_journal(&t.dirs.journal().join("x.json"), &j).unwrap();
    assert_eq!(rollback(&t.dirs).unwrap().steps, 1);
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("settings.json")).unwrap(),
        "{\"mine\":1}"
    );
}

#[test]
fn two_applies_in_a_row_keep_two_journals_and_two_backups() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    let p = only(&t, "settings.json", link("devops/settings.json"));
    let at = t.dirs.claude.join("settings.json");
    std::fs::write(&at, "first").unwrap();
    apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap()).unwrap();
    std::fs::remove_file(&at).unwrap();
    std::fs::write(&at, "second").unwrap();
    apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap()).unwrap();

    let js = journals(&t);
    assert_eq!(js.len(), 2, "{js:?}");
    let saved: Vec<String> = js
        .iter()
        .map(|j| {
            let stamp = j.trim_end_matches(".json");
            std::fs::read_to_string(t.dirs.backups().join(stamp).join("settings.json")).unwrap()
        })
        .collect();
    assert_eq!(
        saved,
        ["first", "second"],
        "names sort in the order of the runs"
    );
    assert_eq!(rollback(&t.dirs).unwrap().steps, 1);
    assert_eq!(std::fs::read_to_string(&at).unwrap(), "second");
}

#[test]
fn a_correct_copy_is_left_alone_and_a_stale_one_is_redone() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::write(t.dev.join("devops/rules/a.md"), "a").unwrap();
    std::fs::write(t.dev.join("devops/settings.json"), "{\"k\":1}").unwrap();
    let mut p = only(&t, "rules", copy_spec("devops/rules"));
    p.links
        .insert("settings.json".into(), copy_spec("devops/settings.json"));
    apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap()).unwrap();
    let before = journals(&t);

    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert!(steps.iter().all(|s| !s.op.changes()), "{steps:?}");
    assert_eq!(apply(&t.dirs, &cat, &steps).unwrap(), None);
    assert_eq!(journals(&t), before, "no new journal");
    assert!(
        !t.dirs.backups().exists(),
        "no backup of a copy that is already right"
    );

    std::fs::write(t.dirs.claude.join("rules/a.md"), "edited").unwrap();
    let op = |k: &str| {
        plan(&t.dirs, &cat, &p)
            .unwrap()
            .into_iter()
            .find(|s| s.path == k)
            .unwrap()
            .op
    };
    assert_eq!(op("rules"), Op::BackupCopy);
    assert_eq!(op("settings.json"), Op::None);
}

#[test]
fn a_folder_linked_to_a_missing_target_keeps_its_files_in_view() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    let agents = t.dirs.claude.join("agents");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::write(agents.join("reviewer.md"), "r").unwrap();
    let target = t.dev.join("claude/agents");
    let p = only(&t, "agents", link("claude/agents"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    let s = steps.iter().find(|s| s.path == "agents").unwrap();
    assert_eq!(s.op, Op::SeedLink);
    assert_eq!(
        s.say(&t.dirs),
        format!(
            "agents: 1 arquivo vai para {}, e o backup guarda uma cópia",
            target.display()
        )
    );

    apply(&t.dirs, &cat, &steps).unwrap().unwrap();
    assert_eq!(inspect(&agents), Found::Link(target.clone()));
    assert_eq!(
        std::fs::read_to_string(agents.join("reviewer.md")).unwrap(),
        "r",
        "the file shows through the link"
    );
    let stamp = journals(&t)[0].trim_end_matches(".json").to_string();
    assert_eq!(
        std::fs::read_to_string(t.dirs.backups().join(&stamp).join("agents/reviewer.md")).unwrap(),
        "r",
        "the backup keeps the original"
    );
    assert!(
        plan(&t.dirs, &cat, &p)
            .unwrap()
            .iter()
            .all(|s| !s.op.changes())
    );

    // Something written through the link after the run survives the rollback, put aside.
    std::fs::write(agents.join("new.md"), "n").unwrap();
    let rolled = rollback(&t.dirs).unwrap();
    assert_eq!(rolled.steps, 1);
    assert_eq!(inspect(&agents), Found::Dir);
    assert_eq!(
        std::fs::read_to_string(agents.join("reviewer.md")).unwrap(),
        "r"
    );
    assert_eq!(inspect(&agents.join("new.md")), Found::Missing);
    assert_eq!(
        inspect(&target),
        Found::Missing,
        "the filled target is gone from dev"
    );
    let aside = t.dirs.backups().join(&stamp).join(".seeded/agents");
    assert_eq!(rolled.aside.len(), 1);
    assert_eq!(rolled.aside[0].aside, aside);
    assert_eq!(std::fs::read_to_string(aside.join("new.md")).unwrap(), "n");
}

#[test]
fn an_empty_target_folder_is_filled_and_comes_back_empty() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::create_dir_all(t.dirs.claude.join("rules")).unwrap();
    std::fs::write(t.dirs.claude.join("rules/mine.md"), "mine").unwrap();
    // devops/rules exists and is empty.
    let p = only(&t, "rules", link("devops/rules"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert_eq!(
        steps.iter().find(|s| s.path == "rules").unwrap().op,
        Op::SeedLink
    );
    apply(&t.dirs, &cat, &steps).unwrap();
    assert_eq!(
        std::fs::read_to_string(t.dev.join("devops/rules/mine.md")).unwrap(),
        "mine"
    );
    rollback(&t.dirs).unwrap();
    assert_eq!(inspect(&t.dev.join("devops/rules")), Found::Dir);
    assert_eq!(
        std::fs::read_dir(t.dev.join("devops/rules"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("rules/mine.md")).unwrap(),
        "mine"
    );
}

#[test]
fn a_file_linked_to_a_missing_target_is_copied_there() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::write(t.dirs.claude.join("settings.json"), "{\"mine\":1}").unwrap();
    let p = only(&t, "settings.json", link("claude/settings.json"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert_eq!(
        steps.iter().find(|s| s.path == "settings.json").unwrap().op,
        Op::SeedLink
    );
    apply(&t.dirs, &cat, &steps).unwrap();
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("settings.json")).unwrap(),
        "{\"mine\":1}"
    );
}

#[test]
fn a_full_target_still_wins_and_the_entry_goes_to_the_backup() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::create_dir_all(t.dirs.claude.join("rules")).unwrap();
    std::fs::write(t.dirs.claude.join("rules/mine.md"), "mine").unwrap();
    std::fs::write(t.dev.join("devops/rules/theirs.md"), "t").unwrap();
    let p = only(&t, "rules", link("devops/rules"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert_eq!(
        steps.iter().find(|s| s.path == "rules").unwrap().op,
        Op::BackupLink
    );
}

#[test]
fn a_seed_that_stopped_midway_is_rolled_back() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    let agents = t.dirs.claude.join("agents");
    std::fs::create_dir_all(&agents).unwrap();
    std::fs::write(agents.join("a.md"), "a").unwrap();
    // A dangling link inside stops the copy after the target was started.
    symlink(Path::new("/nowhere/at/all"), &agents.join("z-broken")).unwrap();
    let p = only(&t, "agents", link("claude/agents"));
    let steps = plan(&t.dirs, &cat, &p).unwrap();
    assert!(apply(&t.dirs, &cat, &steps).is_err());
    assert_eq!(inspect(&agents), Found::Dir, "the original never moved");
    rollback(&t.dirs).unwrap();
    assert_eq!(std::fs::read_to_string(agents.join("a.md")).unwrap(), "a");
    assert_eq!(inspect(&t.dev.join("claude/agents")), Found::Missing);
}

#[test]
fn a_copy_from_nowhere_is_refused_before_anything_moves() {
    let t = setup();
    let cat = Catalog::builtin().unwrap();
    std::fs::create_dir_all(t.dirs.claude.join("rules")).unwrap();
    std::fs::write(t.dirs.claude.join("rules/mine.md"), "mine").unwrap();
    let p = only(&t, "rules", copy_spec("nowhere/rules"));
    let err = apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap()).unwrap_err();
    assert!(format!("{err}").contains("nada foi mudado"), "{err}");
    assert_eq!(
        std::fs::read_to_string(t.dirs.claude.join("rules/mine.md")).unwrap(),
        "mine"
    );
    assert!(
        !t.dirs.journal().exists(),
        "no journal for a run that did nothing"
    );
}
