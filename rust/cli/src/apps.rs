//! The terminal apps tui-embed opens in a pane, abtop and gitlogue, built with their `--embed`
//! mode. Upstream releases do not have that mode, so setup installs them from forks with cargo,
//! into the prefix Mandrake itself goes to, and doctor checks that the ones on PATH answer it.

use anyhow::{Result, bail};
use mandrake_home::Dirs;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct App {
    pub name: &'static str,
    pub git: &'static str,
}

/// Where each `--embed` build comes from: the fork, on the branch [`BRANCH`].
pub const APPS: [App; 2] = [
    App {
        name: "abtop",
        git: "https://github.com/tiamatio/abtop",
    },
    App {
        name: "gitlogue",
        git: "https://github.com/tiamatio/gitlogue",
    },
];
pub const BRANCH: &str = "embed";

/// `MANDRAKE_PREFIX`, else `~/.local`, the same root `install.sh` gives `cargo install`.
pub fn prefix(dirs: &Dirs) -> PathBuf {
    std::env::var_os("MANDRAKE_PREFIX")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs.home.join(".local"))
}

/// Where the binary `name` is: on `path` (what tui-embed spawns from), else in `prefix_bin`.
/// The flag says whether it was found on `path`.
fn find(name: &str, path: Option<&OsString>, prefix_bin: &Path) -> Option<(PathBuf, bool)> {
    on_path(name, path).map(|p| (p, true)).or_else(|| {
        Some(prefix_bin.join(name))
            .filter(|p| p.is_file())
            .map(|p| (p, false))
    })
}

/// The first `name` on `path`, the one a spawn by name runs.
fn on_path(name: &str, path: Option<&OsString>) -> Option<PathBuf> {
    path.into_iter()
        .flat_map(std::env::split_paths)
        .filter(|d| d.is_absolute())
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

/// Whether `bin` has the `--embed` mode. Asked with no socket, a build with it says the flag needs
/// one and exits; an upstream build does not know the flag. Killed after 5 s, which counts as no.
pub fn answers_embed(bin: &Path) -> bool {
    let Ok(mut child) = Command::new(bin)
        .arg("--embed")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    else {
        return false;
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < Duration::from_secs(5) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
    let mut text = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut text);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut text);
    }
    text.contains("--embed") && !text.contains("unexpected argument")
}

/// The doctor line for one app: found where tui-embed looks, and with `--embed`.
fn check_one(app: &App, path: Option<&OsString>, prefix_bin: &Path) -> (bool, String) {
    let fix = format!(
        "cargo install --git {} --branch {BRANCH} --locked --root {}",
        app.git,
        prefix_bin.parent().unwrap_or(prefix_bin).display()
    );
    match find(app.name, path, prefix_bin) {
        None => (
            false,
            format!("{}: not on PATH; install it: {fix}", app.name),
        ),
        Some((p, _)) if !answers_embed(&p) => (
            false,
            format!(
                "{}: {} has no --embed mode; install the fork: {fix}",
                app.name,
                p.display()
            ),
        ),
        Some((p, false)) => (
            false,
            format!(
                "{}: {} has --embed but its folder is not on PATH, so tui-embed will not find it",
                app.name,
                p.display()
            ),
        ),
        Some((p, true)) => (
            true,
            format!("{}: {} answers --embed", app.name, p.display()),
        ),
    }
}

/// One doctor line per app.
pub fn check(dirs: &Dirs) -> Vec<(bool, String)> {
    let path = std::env::var_os("PATH");
    let bin = prefix(dirs).join("bin");
    APPS.iter()
        .map(|a| check_one(a, path.as_ref(), &bin))
        .collect()
}

/// Installs `app` with cargo unless a build with `--embed` is already where tui-embed looks.
pub fn install(dirs: &Dirs, app: &App) -> Result<Vec<String>> {
    let root = prefix(dirs);
    let bin = root.join("bin");
    let path = std::env::var_os("PATH");
    let off_path = |p: &Path| {
        format!(
            "{} não é o primeiro {} do PATH: ponha a pasta no PATH, ou abtop_path/gitlogue_path no /config",
            p.display(),
            app.name
        )
    };
    if let Some((p, on)) = find(app.name, path.as_ref(), &bin)
        && answers_embed(&p)
    {
        let mut lines = vec![format!("{} já tem --embed, mantido", p.display())];
        if !on {
            lines.push(off_path(&p));
        }
        return Ok(lines);
    }
    if !Command::new("cargo")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
    {
        bail!(
            "não instalado: cargo não encontrado (instale Rust em rustup.rs e rode `mandrake setup` de novo)"
        );
    }
    println!(
        "  ..   app {}: compilando de {} ({BRANCH}), pode levar alguns minutos",
        app.name, app.git
    );
    let out = Command::new("cargo")
        .args(["install", "--quiet", "--locked", "--git", app.git])
        .args(["--branch", BRANCH, "--root"])
        .arg(&root)
        .arg(app.name)
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        bail!(
            "não instalado: cargo install falhou: {}",
            cargo_error(&String::from_utf8_lossy(&out.stderr))
        );
    }
    let built = bin.join(app.name);
    if !answers_embed(&built) {
        bail!("instalado em {}, mas não responde --embed", built.display());
    }
    let mut lines = vec![format!("--embed instalado em {}", built.display())];
    if on_path(app.name, path.as_ref()).is_none_or(|p| p != built) {
        lines.push(off_path(&built));
    }
    Ok(lines)
}

/// The line of cargo's output that says why it failed: the first `error`, else the last line.
fn cargo_error(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    let i = lines.iter().position(|l| l.starts_with("error"));
    let mut why = match i {
        Some(i) => lines[i..]
            .iter()
            .take(2)
            .copied()
            .collect::<Vec<_>>()
            .join(" "),
        None => lines.last().copied().unwrap_or("sem saída").to_owned(),
    };
    if why.len() > 240 {
        let cut = (0..=240)
            .rev()
            .find(|&c| why.is_char_boundary(c))
            .unwrap_or(0);
        why.truncate(cut);
        why.push('…');
    }
    why
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mandrake-apps-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn script(dir: &Path, name: &str, body: &str) {
        let p = dir.join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn doctor_fails_with_a_clear_line_when_a_binary_is_missing() {
        let d = temp("missing");
        let (ok, line) = check_one(&APPS[1], Some(&d.clone().into()), &d.join("prefix/bin"));
        assert!(!ok);
        assert!(line.starts_with("gitlogue: not on PATH; install it: cargo install --git https://github.com/tiamatio/gitlogue --branch embed"), "{line}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn doctor_tells_an_embed_build_from_an_upstream_one() {
        let d = temp("builds");
        let path: OsString = d.clone().into();
        script(
            &d,
            "abtop",
            "echo 'Error: --embed requires a socket path' >&2",
        );
        let (ok, line) = check_one(&APPS[0], Some(&path), &d.join("prefix/bin"));
        assert!(ok, "{line}");
        assert!(line.ends_with("answers --embed"), "{line}");

        script(
            &d,
            "gitlogue",
            "echo \"error: unexpected argument '--embed' found\" >&2; exit 2",
        );
        let (ok, line) = check_one(&APPS[1], Some(&path), &d.join("prefix/bin"));
        assert!(!ok);
        assert!(line.contains("has no --embed mode"), "{line}");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn doctor_flags_a_build_off_path() {
        let d = temp("offpath");
        let bin = d.join("prefix/bin");
        std::fs::create_dir_all(&bin).unwrap();
        script(&bin, "abtop", "echo '--embed requires a socket path' >&2");
        let (ok, line) = check_one(&APPS[0], Some(&d.join("empty").into()), &bin);
        assert!(!ok);
        assert!(
            line.contains("not on PATH, so tui-embed will not find it"),
            "{line}"
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn cargo_error_picks_the_reason() {
        let e = "    Updating git repository `x`\nerror: failed to clone into: /tmp/x\n\nCaused by:\n  network down\n";
        assert_eq!(
            cargo_error(e),
            "error: failed to clone into: /tmp/x Caused by:"
        );
        assert_eq!(cargo_error(""), "sem saída");
    }
}
