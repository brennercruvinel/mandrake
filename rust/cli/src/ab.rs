//! `mandrake bench`: the real A/B. Each prompt runs through `claude -p` twice, with the user's own
//! login: once without Mandrake (no prompt-trim, a proxy in pass mode that only measures) and once
//! with it (the installed prompt-trim, a proxy that cuts). Tokens are what `count_tokens` reports
//! for each request as sent. Two short real requests per prompt.

use anyhow::{Context, Result, bail};
use mandrake_home::{Dirs, Profile};
use mandrake_meter::{BenchReport, BenchRow, Event, Mode};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Short prompts that each make one request: what every session pays before any work.
pub const PROMPTS: [(&str, &str); 3] = [
    ("resposta-curta", "Responda só: ok"),
    ("pergunta", "Em uma frase: o que é um symlink?"),
    (
        "codigo",
        "Escreva uma função Rust que soma um slice de i32. Só o código.",
    ),
];

/// How long a bench run keeps its bodies, for `mandrake hex` on them.
const BENCH_KEEP_DAYS: u32 = 7;

pub(crate) struct Proxies(Vec<Child>);
impl Drop for Proxies {
    fn drop(&mut self) {
        for c in &mut self.0 {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

pub(crate) fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

pub(crate) fn wait_port(port: u16) -> bool {
    let end = Instant::now() + Duration::from_secs(10);
    while Instant::now() < end {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

/// Runs `claude -p` with a settings overlay, with a ceiling; returns its last line.
pub(crate) fn claude(cwd: &Path, port: u16, plugin_dir: &str, prompt: &str) -> Result<String> {
    let settings = serde_json::json!({ "env": {
        "ANTHROPIC_BASE_URL": format!("http://127.0.0.1:{port}"),
        "CLAUDE_CODE_PLUGIN_DIRS": plugin_dir,
    }});
    let mut child = Command::new("claude")
        .args(["-p", "--settings", &settings.to_string(), "--", prompt])
        .current_dir(cwd)
        .env("MANDRAKE_PROXY", "0")
        .env_remove("ANTHROPIC_BASE_URL")
        // Inside a soul session prompt-trim would skip the core, and gh would answer as the soul.
        .env_remove("MANDRAKE_SOUL")
        .env_remove("GH_CONFIG_DIR")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("running claude (is Claude Code installed and logged in?)")?;
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if Instant::now() > end {
            let _ = child.kill();
            bail!("claude -p passou de 120 s");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    let out = child.wait_with_output()?;
    let text = String::from_utf8_lossy(&out.stdout);
    if !out.status.success() {
        bail!(
            "claude -p falhou: {}",
            String::from_utf8_lossy(&out.stderr)
                .lines()
                .last()
                .unwrap_or("")
        );
    }
    Ok(text.trim().lines().last().unwrap_or("").to_string())
}

/// The events written since `seen`, once the log has been quiet for a while: the proxy writes an
/// event only when its stream ends, so a small side request (title, quota) can land before the
/// conversation, and `claude -p` can exit before either is written.
pub(crate) fn new_events(path: &Path, seen: usize) -> Vec<Event> {
    settle(
        path,
        seen,
        Duration::from_millis(1500),
        Duration::from_secs(40),
    )
}

fn settle(path: &Path, seen: usize, quiet: Duration, ceiling: Duration) -> Vec<Event> {
    let end = Instant::now() + ceiling;
    let (mut count, mut since) = (seen, Instant::now());
    loop {
        let all = mandrake_meter::read(path).unwrap_or_default();
        if all.len() != count {
            (count, since) = (all.len(), Instant::now());
        }
        if (count > seen && since.elapsed() >= quiet) || Instant::now() > end {
            return all.into_iter().skip(seen).collect();
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Removes the `run-*` folders in `parent` older than `keep_days`. Each run keeps whole request
/// bodies under its own MANDRAKE_HOME, which no later run reopens, so its own sweep never comes.
pub(crate) fn sweep_runs(parent: &Path, keep_days: u32) -> usize {
    let Ok(entries) = std::fs::read_dir(parent) else {
        return 0;
    };
    let cutoff = std::time::SystemTime::now() - Duration::from_secs(u64::from(keep_days) * 86_400);
    entries
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("run-"))
        .filter(|e| {
            std::fs::symlink_metadata(e.path())
                .is_ok_and(|m| m.is_dir() && m.modified().is_ok_and(|t| t < cutoff))
        })
        .filter(|e| std::fs::remove_dir_all(e.path()).is_ok())
        .count()
}

fn sum(evs: &[Event], f: impl Fn(&Event) -> Option<u64>) -> Option<u64> {
    evs.iter()
        .map(&f)
        .try_fold(0u64, |a, v| v.map(|v| a + v))
        .filter(|_| !evs.is_empty())
}

/// Starts the two measuring proxies of a run folder, `pass` and `trim`, each on a free port; they
/// read the profile saved in `run/config` and stop when the returned guard drops.
pub(crate) fn proxies(run: &Path) -> Result<(Proxies, u16, u16)> {
    let exe = std::env::current_exe()?;
    let (pass_port, trim_port) = (free_port()?, free_port()?);
    let mut kids = Proxies(vec![]);
    for (port, pass) in [(pass_port, true), (trim_port, false)] {
        let mut c = Command::new(&exe);
        c.args(["proxy", "run", "--port", &port.to_string()])
            .env("MANDRAKE_HOME", run);
        if pass {
            c.arg("--pass");
        }
        let log = std::fs::File::create(run.join(format!("proxy-{port}.log")))?;
        kids.0.push(c.stdout(log.try_clone()?).stderr(log).spawn()?);
        if !wait_port(port) {
            bail!(
                "o proxy de medição não subiu na porta {port} (veja {})",
                run.display()
            );
        }
    }
    Ok((kids, pass_port, trim_port))
}

pub fn run(dirs: &Dirs, profile: &Profile, plugin_dir: &Path, out: &Path) -> Result<BenchReport> {
    if !plugin_dir.join("hooks/register.js").is_file() {
        bail!(
            "prompt-trim não instalado em {}: rode mandrake setup",
            plugin_dir.display()
        );
    }
    let runs = dirs.state.join("bench");
    sweep_runs(&runs, BENCH_KEEP_DAYS);
    let run = runs.join(format!("run-{}", mandrake_home::stamp()));
    let mut p = profile.clone();
    p.proxy.count_tokens = true;
    p.proxy.audit = true;
    p.proxy.audit_keep_days = BENCH_KEEP_DAYS;
    p.save(&run.join("config/mandrake.toml"))?;
    let (kids, pass_port, trim_port) = proxies(&run)?;
    let events = run.join("state/events.jsonl");
    let mut rows = vec![];
    println!(
        "bench: {} prompts, cada um sem e com Mandrake, pela sua sessão do Claude Code",
        PROMPTS.len()
    );
    for (name, prompt) in PROMPTS {
        let side = |port: u16, dir: &str, mode: Mode| -> Result<Vec<Event>> {
            let seen = mandrake_meter::read(&events)?.len();
            let reply = claude(&run, port, dir, prompt)?;
            let evs: Vec<Event> = new_events(&events, seen)
                .into_iter()
                .filter(|e| e.mode == mode)
                .collect();
            let tok = sum(&evs, |e| e.tokens_after).map_or("-".into(), |t| t.to_string());
            println!(
                "  {name:<15} {:<4} {tok:>7} tokens  resposta: {}",
                if mode == Mode::Pass { "sem" } else { "com" },
                reply.chars().take(50).collect::<String>()
            );
            Ok(evs)
        };
        let without = side(pass_port, "", Mode::Pass)?;
        let with = side(trim_port, &plugin_dir.to_string_lossy(), Mode::Trim)?;
        if without.is_empty() || with.is_empty() {
            bail!(
                "{name}: o proxy não registrou a requisição (veja {})",
                run.display()
            );
        }
        rows.push(BenchRow {
            name: name.into(),
            bytes_before: without.iter().map(|e| e.bytes_after).sum(),
            bytes_after: with.iter().map(|e| e.bytes_after).sum(),
            tokens_before: sum(&without, |e| e.tokens_after),
            tokens_after: sum(&with, |e| e.tokens_after),
            cuts: with.last().map(|e| e.cuts.clone()).unwrap_or_default(),
        });
    }
    drop(kids);
    let report = BenchReport {
        ts: mandrake_home::rfc3339(),
        counted: rows
            .iter()
            .all(|r| r.tokens_before.is_some() && r.tokens_after.is_some()),
        rows,
        kind: "ab".into(),
    };
    let (json, md) = mandrake_proxy::write_bench(&report, out)?;
    println!("relatório: {} e {}", json.display(), md.display());
    println!(
        "requisições guardadas em {} (mandrake hex <id> com MANDRAKE_HOME={})",
        run.display(),
        run.display()
    );
    println!("ver: mandrake bench --view");
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, bytes: u64) -> Event {
        Event {
            id: id.into(),
            ts: String::new(),
            mode: Mode::Pass,
            path: "/v1/messages".into(),
            model: None,
            status: 200,
            ms: 0,
            bytes_before: bytes,
            bytes_after: bytes,
            tokens_before: None,
            tokens_after: None,
            usage: None,
            cuts: Default::default(),
            audited: true,
            error: None,
        }
    }

    #[test]
    fn a_side_request_first_does_not_end_the_wait() {
        let tmp = tempfile::tempdir().unwrap();
        let log = tmp.path().join("events.jsonl");
        mandrake_meter::append(&log, &ev("old", 1)).unwrap();
        mandrake_meter::append(&log, &ev("title", 900)).unwrap();
        let writer = {
            let log = log.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(300));
                mandrake_meter::append(&log, &ev("conversation", 90_000)).unwrap();
            })
        };
        let evs = settle(&log, 1, Duration::from_millis(800), Duration::from_secs(10));
        writer.join().unwrap();
        let ids: Vec<_> = evs.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(ids, ["title", "conversation"]);
    }

    #[test]
    fn old_runs_are_swept_and_fresh_ones_kept() {
        let tmp = tempfile::tempdir().unwrap();
        let (old, fresh, other) = (
            tmp.path().join("run-old"),
            tmp.path().join("run-new"),
            tmp.path().join("keep-me"),
        );
        for d in [&old, &fresh, &other] {
            std::fs::create_dir_all(d.join("state/audit")).unwrap();
        }
        let two_days = std::time::SystemTime::now() - Duration::from_secs(2 * 86_400);
        for d in [&old, &other] {
            std::fs::File::open(d)
                .unwrap()
                .set_modified(two_days)
                .unwrap();
        }
        assert_eq!(sweep_runs(tmp.path(), 1), 1);
        assert!(!old.exists() && fresh.exists() && other.exists());
        assert_eq!(sweep_runs(&tmp.path().join("none"), 1), 0);
    }
}
