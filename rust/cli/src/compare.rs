//! `mandrake compare`: what Claude Code sends for one prompt, without Mandrake and with it, side by
//! side. Both runs go through `claude -p` with the user's own Claude Code, through the two
//! measuring proxies, to a stand-in API on loopback that answers locally: no request reaches the
//! API, nothing is spent, and the bodies kept are the real ones.

use crate::ab;
use anyhow::{Context, Result, bail};
use mandrake_home::{Dirs, Profile};
use mandrake_meter::{Event, Mode};
use std::path::Path;

pub const PROMPT: &str = "Responda só: ok";

/// How the result is shown.
pub enum Shown {
    /// The side-by-side screen, or the table when stdout is not a terminal.
    Screen,
    Table,
    /// One frame as text, `WIDTHxHEIGHT`, for tests and recordings.
    Snapshot(String),
}

pub fn run(
    dirs: &Dirs,
    profile: &Profile,
    plugin_dir: &Path,
    prompt: &str,
    shown: Shown,
) -> Result<()> {
    if !plugin_dir.join("hooks/register.js").is_file() {
        bail!(
            "prompt-trim não instalado em {}: rode mandrake setup",
            plugin_dir.display()
        );
    }
    let core = std::fs::read_to_string(plugin_dir.join("hooks/core.js")).unwrap_or_default();
    if core.contains("CORE = ''") || core.contains("CORE = \"\"") {
        eprintln!(
            "aviso: o core está vazio em {}; o lado com Mandrake sai sem ele (rode mandrake setup)",
            plugin_dir.display()
        );
    }
    let run = dirs
        .state
        .join("compare")
        .join(format!("run-{}", dirs.fresh_stamp()));
    let api = ab::free_port()?;
    std::thread::spawn(move || mandrake_proxy::serve_capture(api));
    if !ab::wait_port(api) {
        bail!("a API local de captura não subiu na porta {api}");
    }
    let mut p = profile.clone();
    p.proxy.upstream = format!("http://127.0.0.1:{api}");
    p.proxy.count_tokens = false;
    p.proxy.audit = true;
    p.proxy.audit_keep_days = 1;
    p.save(&run.join("config/mandrake.toml"))?;
    let (kids, pass_port, trim_port) = ab::proxies(&run)?;
    let events = run.join("state/events.jsonl");
    eprintln!(
        "compare: \"{prompt}\" pelo seu Claude Code, sem e com o Mandrake; nada vai para a API"
    );
    let side = |port: u16, dir: &str, mode: Mode| -> Result<Vec<u8>> {
        let seen = mandrake_meter::read(&events)?.len();
        ab::claude(&run, port, dir, prompt)?;
        let evs = ab::new_events(&events, seen);
        let main = main_request(&evs, mode).with_context(|| {
            format!(
                "o proxy não registrou a requisição (veja {})",
                run.display()
            )
        })?;
        let path = run
            .join("state/audit")
            .join(format!("{}.after.json", main.id));
        std::fs::read(&path).with_context(|| format!("lendo {}", path.display()))
    };
    let without = side(pass_port, "", Mode::Pass)?;
    let with = side(trim_port, &plugin_dir.to_string_lossy(), Mode::Trim)?;
    drop(kids);
    eprintln!("corpos guardados em {}", run.join("state/audit").display());
    match shown {
        Shown::Screen => mandrake_tui::compare(&without, &with, prompt, false),
        Shown::Table => mandrake_tui::compare(&without, &with, prompt, true),
        Shown::Snapshot(size) => {
            for part in 0..3 {
                println!(
                    "{}",
                    mandrake_tui::compare_snapshot(&without, &with, prompt, &size, part)?
                );
            }
            Ok(())
        }
    }
}

/// The conversation request of one run: the largest audited `/v1/messages` call of that mode, since
/// Claude Code may also send small side requests (titles, quota checks).
fn main_request(evs: &[Event], mode: Mode) -> Option<&Event> {
    evs.iter()
        .filter(|e| e.mode == mode && e.audited && e.path.ends_with("/v1/messages"))
        .max_by_key(|e| e.bytes_before)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(id: &str, mode: Mode, path: &str, bytes: u64) -> Event {
        Event {
            id: id.into(),
            ts: String::new(),
            mode,
            path: path.into(),
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
    fn the_main_request_is_the_largest_conversation_call() {
        let evs = [
            ev("a", Mode::Pass, "/v1/messages", 900),
            ev("b", Mode::Pass, "/v1/messages", 90_000),
            ev("c", Mode::Pass, "/v1/messages/count_tokens", 200_000),
            ev("d", Mode::Trim, "/v1/messages", 500_000),
        ];
        assert_eq!(
            main_request(&evs, Mode::Pass).map(|e| e.id.as_str()),
            Some("b")
        );
        assert_eq!(
            main_request(&evs[..1], Mode::Trim).map(|e| e.id.as_str()),
            None
        );
    }
}
