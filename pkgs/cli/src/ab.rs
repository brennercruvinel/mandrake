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

struct Proxies(Vec<Child>);
impl Drop for Proxies {
    fn drop(&mut self) {
        for c in &mut self.0 {
            let _ = c.kill();
            let _ = c.wait();
        }
    }
}

fn free_port() -> Result<u16> {
    Ok(TcpListener::bind("127.0.0.1:0")?.local_addr()?.port())
}

fn wait_port(port: u16) -> bool {
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
fn claude(cwd: &Path, port: u16, plugin_dir: &str, prompt: &str) -> Result<String> {
    let settings = serde_json::json!({ "env": {
        "ANTHROPIC_BASE_URL": format!("http://127.0.0.1:{port}"),
        "CLAUDE_CODE_PLUGIN_DIRS": plugin_dir,
    }});
    let mut child = Command::new("claude")
        .args(["-p", "--settings", &settings.to_string(), "--", prompt])
        .current_dir(cwd)
        .env("MANDRAKE_PROXY", "0")
        .env_remove("ANTHROPIC_BASE_URL")
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

/// The events written since `seen`, waiting for their token counts to land.
fn new_events(path: &Path, seen: usize) -> Vec<Event> {
    let end = Instant::now() + Duration::from_secs(40);
    loop {
        let all = mandrake_meter::read(path).unwrap_or_default();
        if all.len() > seen || Instant::now() > end {
            return all.into_iter().skip(seen).collect();
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

fn sum(evs: &[Event], f: impl Fn(&Event) -> Option<u64>) -> Option<u64> {
    evs.iter()
        .map(&f)
        .try_fold(0u64, |a, v| v.map(|v| a + v))
        .filter(|_| !evs.is_empty())
}

pub fn run(dirs: &Dirs, profile: &Profile, plugin_dir: &Path, out: &Path) -> Result<BenchReport> {
    if !plugin_dir.join("hooks/register.js").is_file() {
        bail!(
            "prompt-trim não instalado em {}: rode mandrake setup",
            plugin_dir.display()
        );
    }
    let run = dirs
        .state
        .join("bench")
        .join(format!("run-{}", mandrake_home::stamp()));
    let mut p = profile.clone();
    p.proxy.count_tokens = true;
    p.proxy.audit = true;
    p.proxy.audit_keep_days = 7;
    p.save(&run.join("config/mandrake.toml"))?;
    let exe = std::env::current_exe()?;
    let (pass_port, trim_port) = (free_port()?, free_port()?);
    let mut kids = Proxies(vec![]);
    for (port, pass) in [(pass_port, true), (trim_port, false)] {
        let mut c = Command::new(&exe);
        c.args(["proxy", "run", "--port", &port.to_string()])
            .env("MANDRAKE_HOME", &run);
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
