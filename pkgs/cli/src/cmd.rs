//! Subcommands that are glue between crates. Owner: Tiamati.

use anyhow::{Context, Result, bail};
use mandrake_home::{Dirs, Profile};
use mandrake_layout::{Catalog, links, settings};
use std::path::PathBuf;

use crate::install;

/// The saved profile, or the defaults when there is none yet.
pub fn profile(dirs: &Dirs) -> Result<Profile> {
    let f = dirs.profile();
    if f.exists() {
        Profile::load(&f)
    } else {
        Ok(Profile::default())
    }
}

/// What the wizard starts from: the given file, the saved profile, else what is on the machine now.
fn starting_point(dirs: &Dirs, from: Option<&PathBuf>) -> Result<Profile> {
    if let Some(f) = from {
        return Profile::load(f);
    }
    if dirs.profile().exists() {
        return Profile::load(&dirs.profile());
    }
    if dirs.claude.is_dir() {
        return links::capture(dirs);
    }
    Ok(Profile::default())
}

pub fn setup(dirs: &Dirs, from: Option<PathBuf>, yes: bool) -> Result<()> {
    let catalog = Catalog::builtin()?;
    let start = starting_point(dirs, from.as_ref())?;
    let profile = if yes {
        start
    } else {
        match mandrake_tui::setup_wizard(dirs, &catalog, &start)? {
            Some(p) => p,
            None => {
                println!("setup: nothing changed");
                return Ok(());
            }
        }
    };
    profile.save(&dirs.profile())?;
    println!("profile: {}", dirs.profile().display());

    let src = install::source(dirs)?;
    let root = install::data(dirs, &src)?;
    println!("data: {}", root.display());

    std::fs::create_dir_all(&dirs.claude)?;
    let steps = links::plan(dirs, &catalog, &profile)?;
    print_steps(&steps);
    if let Some(j) = links::apply(dirs, &catalog, &steps, &mandrake_home::stamp())? {
        println!("links: applied, journal {}", j.display());
    }
    let mut notes = install::settings_keys(dirs, &profile, &root)?;
    notes.extend(install::shell(dirs, &profile, &root)?);
    notes.extend(install::plugins(dirs, &profile, &root)?);
    notes.extend(install::skill(dirs, &profile, &root)?);
    for n in notes {
        println!("{n}");
    }
    let exe = std::env::current_exe()?;
    match mandrake_proxy::install_service(dirs, &profile, &exe) {
        Ok(()) => println!(
            "proxy: service installed on 127.0.0.1:{}",
            profile.proxy.port
        ),
        Err(e) => println!("proxy: service not installed: {e:#}"),
    }
    println!("next: open a new shell, then `mandrake doctor`");
    Ok(())
}

pub fn capture(dirs: &Dirs, out: Option<PathBuf>) -> Result<()> {
    let mut p = links::capture(dirs)?;
    // Keep what the saved profile says about everything that is not a link.
    if dirs.profile().exists() {
        let saved = Profile::load(&dirs.profile())?;
        p = Profile {
            dev_root: p.dev_root,
            links: p.links,
            ..saved
        };
    }
    let doc = settings::read(&dirs.claude.join("settings.json"))?;
    if let Some(port) = doc["env"][settings::BASE_URL]
        .as_str()
        .and_then(|u| u.rsplit(':').next())
        .and_then(|p| p.parse().ok())
    {
        p.proxy.port = port;
    }
    p.plugins.fancy |= settings::plugin_enabled(&doc, "fancy").is_some();
    p.plugins.tui_embed |= settings::plugin_enabled(&doc, "tui-embed").is_some();
    let text = toml::to_string_pretty(&p)?;
    match out {
        Some(f) => {
            p.save(&f)?;
            println!("captured {} links into {}", p.links.len(), f.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}

fn print_steps(steps: &[links::Step]) {
    let changing: Vec<_> = steps.iter().filter(|s| s.op.changes()).collect();
    if changing.is_empty() {
        return println!("links: nothing to change");
    }
    for s in changing {
        let to = s
            .target
            .as_ref()
            .map(|t| t.display().to_string())
            .unwrap_or_default();
        println!(
            "links: {:<11} {} -> {}",
            format!("{:?}", s.op).to_lowercase(),
            s.path,
            to
        );
    }
}

pub fn link_plan(dirs: &Dirs) -> Result<()> {
    let p = Profile::load(&dirs.profile())
        .context("no profile yet: run mandrake setup or mandrake capture --out")?;
    print_steps(&links::plan(dirs, &Catalog::builtin()?, &p)?);
    Ok(())
}

pub fn link_apply(dirs: &Dirs) -> Result<()> {
    let catalog = Catalog::builtin()?;
    let p = Profile::load(&dirs.profile())
        .context("no profile yet: run mandrake setup or mandrake capture --out")?;
    let steps = links::plan(dirs, &catalog, &p)?;
    print_steps(&steps);
    if let Some(j) = links::apply(dirs, &catalog, &steps, &mandrake_home::stamp())? {
        println!("journal {}", j.display());
    }
    Ok(())
}

pub fn link_rollback(dirs: &Dirs) -> Result<()> {
    let n = links::rollback(dirs)?;
    println!("rolled back {n} steps");
    Ok(())
}

pub fn audit(dirs: &Dirs, last: usize) -> Result<()> {
    let mut events: Vec<_> = mandrake_meter::read(&dirs.events())?
        .into_iter()
        .filter(|e| e.audited)
        .collect();
    events.reverse();
    if events.is_empty() {
        println!(
            "no audited requests: set [proxy] audit = true in {}",
            dirs.profile().display()
        );
    }
    for e in events.into_iter().take(last) {
        let saved = e
            .saved_tokens()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "{}\t{}\t{:?}\t{}\t{} -> {} bytes\tsaved {} tokens",
            e.id, e.ts, e.mode, e.status, e.bytes_before, e.bytes_after, saved
        );
    }
    Ok(())
}

/// Each check prints one line; any failure makes the exit code 1.
pub fn doctor(dirs: &Dirs) -> Result<()> {
    let mut bad = 0;
    let mut check = |ok: bool, what: String| {
        println!("{} {what}", if ok { "ok  " } else { "FAIL" });
        bad += usize::from(!ok);
    };
    let Ok(p) = Profile::load(&dirs.profile()) else {
        check(false, format!("profile {}", dirs.profile().display()));
        bail!("no profile: run mandrake setup");
    };
    check(true, format!("profile {}", dirs.profile().display()));

    let steps = links::plan(dirs, &Catalog::builtin()?, &p)?;
    let off: Vec<_> = steps
        .iter()
        .filter(|s| s.op.changes())
        .map(|s| s.path.as_str())
        .collect();
    let linked = steps
        .iter()
        .filter(|s| s.action != mandrake_home::Action::Keep)
        .count();
    check(
        off.is_empty(),
        format!(
            "links: {} as the profile says{}",
            linked - off.len().min(linked),
            if off.is_empty() {
                String::new()
            } else {
                format!(", differ: {}", off.join(", "))
            }
        ),
    );
    let dangling: Vec<_> = steps
        .iter()
        .filter(|s| {
            s.action == mandrake_home::Action::Link
                && s.target.as_ref().is_some_and(|t| !t.exists())
        })
        .map(|s| s.path.as_str())
        .collect();
    if !dangling.is_empty() {
        println!("note link targets not created yet: {}", dangling.join(", "));
    }

    let root = install::source(dirs).unwrap_or_else(|_| dirs.data.join("src"));
    let doc = settings::read(&dirs.claude.join("settings.json"))?;
    for (k, v) in settings::wanted(p.proxy.port, &install::prompt_trim(&dirs.data.join("src"))) {
        let have = doc["env"][k].as_str().unwrap_or("unset");
        check(have == v, format!("settings env.{k} = {have}"));
    }
    check(
        install::prompt_trim(&dirs.data.join("src"))
            .join("hooks/register.js")
            .is_file(),
        "plugin prompt-trim installed".into(),
    );
    for (name, want) in [
        ("fancy", p.plugins.fancy),
        ("tui-embed", p.plugins.tui_embed),
    ] {
        if want {
            let on = settings::plugin_enabled(&doc, name);
            check(
                on.is_some(),
                format!(
                    "plugin {name}: {}",
                    on.unwrap_or_else(|| "not enabled".into())
                ),
            );
        }
    }
    let rc = mandrake_layout::shell::rc_file(&dirs.home, std::env::var("SHELL").ok().as_deref());
    let has_block = std::fs::read_to_string(&rc)
        .unwrap_or_default()
        .contains("# >>> mandrake >>>");
    check(has_block, format!("shell block in {}", rc.display()));
    let souls = install::souls_dir(dirs, &p, &root);
    check(souls.is_dir(), format!("souls {}", souls.display()));
    match mandrake_proxy::status(dirs, &p) {
        Ok(s) => check(
            !s.contains("down") && !s.contains("not "),
            format!("proxy: {s}"),
        ),
        Err(e) => check(false, format!("proxy: {e:#}")),
    }
    let v = std::process::Command::new("claude")
        .arg("--version")
        .output();
    check(
        v.as_ref().is_ok_and(|o| o.status.success()),
        format!(
            "claude {}",
            v.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .unwrap_or_else(|e| e.to_string())
        ),
    );
    if bad > 0 {
        std::process::exit(1);
    }
    Ok(())
}
