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

    let j = apply(&t.dirs, &cat, &steps, "s1").unwrap().unwrap();
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

    assert_eq!(rollback(&t.dirs).unwrap(), 4);
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
    apply(&t.dirs, &cat, &plan(&t.dirs, &cat, &p).unwrap(), "s1").unwrap();
    // The CLI's own links inside its folder stay out of the profile.
    std::fs::create_dir_all(t.dirs.claude.join("debug")).unwrap();
    symlink(&t.dirs.claude.join("debug/x.txt"), &t.dirs.claude.join("debug/latest")).unwrap();
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
