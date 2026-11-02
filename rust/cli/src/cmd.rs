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

/// How `setup` gets its answers.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Asking {
    /// The full-screen wizard.
    Tui,
    /// One question per line on stdin.
    Cli,
    /// None: the starting profile as it is.
    No,
}

pub fn setup(dirs: &Dirs, from: Option<PathBuf>, asking: Asking) -> Result<()> {
    let catalog = Catalog::builtin()?;
    let start = starting_point(dirs, from.as_ref())?;
    let chosen = match asking {
        Asking::No => Some(start),
        Asking::Tui => mandrake_tui::setup_wizard(dirs, &catalog, &start)?,
        Asking::Cli => crate::ask::run(dirs, &catalog, &start, std::io::stdin().lock())?,
    };
    let Some(profile) = chosen else {
        println!("setup: nada mudou");
        return Ok(());
    };
    println!("\nInstalando");
    let mut errors: Vec<String> = vec![];
    let mut step = |name: &str, r: Result<Vec<String>>| match r {
        Ok(lines) if lines.is_empty() => println!("  ok   {name}: sem mudança"),
        Ok(lines) => {
            for l in lines {
                println!("  ok   {name}: {l}");
            }
        }
        Err(e) => {
            println!("  ERRO {name}: {e:#}");
            errors.push(format!("{name}: {e:#}"));
        }
    };
    step(
        "perfil",
        profile
            .save(&dirs.profile())
            .map(|_| vec![dirs.profile().display().to_string()]),
    );
    let root = match install::source(dirs).and_then(|src| install::data(dirs, &src)) {
        Ok(root) => {
            step("dados", Ok(vec![root.display().to_string()]));
            root
        }
        Err(e) => {
            step("dados", Err(e));
            report(&errors);
            bail!("sem os dados do Mandrake não há o que instalar");
        }
    };
    step("core", install::core(dirs, &profile, &root));
    step("links", links_step(dirs, &catalog, &profile));
    step("settings", install::settings_keys(dirs, &profile, &root));
    step("shell", install::shell(dirs, &profile, &root));
    step("plugins", install::plugins(dirs, &profile, &root));
    if profile.plugins.tui_embed {
        for app in &crate::apps::APPS {
            step(
                &format!("app {}", app.name),
                crate::apps::install(dirs, app),
            );
        }
    }
    step("skill", install::skill(dirs, &profile, &root));
    step("clis", install::harnesses(dirs, &profile));
    let exe = std::env::current_exe()?;
    step(
        "proxy",
        mandrake_proxy::install_service(dirs, &profile, &exe).and_then(|_| {
            let port = profile.proxy.port;
            if std::env::var_os("MANDRAKE_HOME").is_some() {
                return Ok(vec![format!(
                    "serviço escrito para 127.0.0.1:{port} (MANDRAKE_HOME: não carregado)"
                )]);
            }
            // Report the proxy up only once it answers, not when the service was merely loaded.
            for _ in 0..50 {
                if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
                    return Ok(vec![format!("no ar em 127.0.0.1:{port}")]);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
            anyhow::bail!(
                "serviço carregado, mas 127.0.0.1:{port} não respondeu em 10 s (veja {})",
                dirs.logs().display()
            )
        }),
    );
    report(&errors);
    if !errors.is_empty() {
        bail!("{} erro(s) na instalação", errors.len());
    }
    println!("próximo passo: abra um shell novo e rode `mandrake doctor`");
    Ok(())
}

fn links_step(dirs: &Dirs, catalog: &Catalog, profile: &Profile) -> Result<Vec<String>> {
    std::fs::create_dir_all(&dirs.claude)?;
    let steps = links::plan(dirs, catalog, profile)?;
    let mut out: Vec<String> = steps
        .iter()
        .filter(|s| s.op.changes())
        .map(|s| s.say(dirs))
        .collect();
    if let Some(j) = links::apply(dirs, catalog, &steps)? {
        out.push(format!("journal {}", j.display()));
    }
    Ok(out)
}

fn report(errors: &[String]) {
    if errors.is_empty() {
        println!("\nErros: nenhum");
    } else {
        println!("\nErros ({}):", errors.len());
        for e in errors {
            println!("  - {e}");
        }
    }
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
            println!("{} links capturados em {}", p.links.len(), f.display());
        }
        None => print!("{text}"),
    }
    Ok(())
}

fn print_steps(dirs: &Dirs, steps: &[links::Step]) {
    let changing: Vec<_> = steps.iter().filter(|s| s.op.changes()).collect();
    if changing.is_empty() {
        return println!("links: nada muda");
    }
    for s in changing {
        println!("links: {}", s.say(dirs));
    }
}

pub fn link_plan(dirs: &Dirs) -> Result<()> {
    let p = Profile::load(&dirs.profile())
        .context("ainda não há perfil: rode mandrake setup, ou mandrake capture --out")?;
    print_steps(dirs, &links::plan(dirs, &Catalog::builtin()?, &p)?);
    Ok(())
}

pub fn link_apply(dirs: &Dirs) -> Result<()> {
    let catalog = Catalog::builtin()?;
    let p = Profile::load(&dirs.profile())
        .context("ainda não há perfil: rode mandrake setup, ou mandrake capture --out")?;
    let steps = links::plan(dirs, &catalog, &p)?;
    print_steps(dirs, &steps);
    if let Some(j) = links::apply(dirs, &catalog, &steps)? {
        println!("journal {}", j.display());
    }
    Ok(())
}

pub fn link_rollback(dirs: &Dirs) -> Result<()> {
    let r = links::rollback(dirs)?;
    match r.steps {
        1 => println!("desfeito: 1 passo"),
        n => println!("desfeitos: {n} passos"),
    }
    for s in &r.aside {
        println!(
            "{}: o que estava lá desde a instalação foi guardado em {}",
            dirs.tilde(&s.target),
            dirs.tilde(&s.aside)
        );
    }
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
            "Nenhuma requisição guardada. A auditoria guarda o corpo inteiro (conversa e código),"
        );
        println!(
            "por isso vem desligada. Para ver antes e depois de um corte real, rode `mandrake bench`:"
        );
        println!(
            "ele guarda as requisições da medição e diz o MANDRAKE_HOME para o `mandrake hex`."
        );
        println!(
            "Para guardar tudo: [proxy] audit = true em {}",
            dirs.profile().display()
        );
    }
    let (kept, gone): (Vec<_>, Vec<_>) = events
        .into_iter()
        .partition(|e| mandrake_tui::has_bodies(dirs, &e.id));
    if !kept.is_empty() {
        println!(
            "{:<24}  {:<16}  {:<3}  {:>6}  {:>21}  economia",
            "id", "quando", "lado", "status", "bytes antes → depois"
        );
    }
    for e in kept.iter().take(last) {
        let saved = match e.saved_tokens() {
            Some(s) if s > 0 => format!("{s} tokens"),
            Some(s) => format!("{s} tokens (cresceu)"),
            None => "-".into(),
        };
        println!(
            "{:<24}  {:<16}  {:<3}  {:>6}  {:>21}  {saved}",
            e.id,
            e.ts.get(..16).unwrap_or(&e.ts).replace('T', " "),
            if e.mode == mandrake_meter::Mode::Trim {
                "com"
            } else {
                "sem"
            },
            e.status,
            format!("{} → {}", e.bytes_before, e.bytes_after),
        );
    }
    if !kept.is_empty() {
        println!("\n`mandrake hex` abre a mais recente; `mandrake hex <id>`, uma delas.");
    }
    if !gone.is_empty() {
        println!(
            "{} requisição(ões) com o corpo já apagado (audit_keep_days) ficaram fora da lista.",
            gone.len()
        );
    }
    Ok(())
}

/// `mandrake` with no arguments: what it does and what is on, in plain words.
pub fn status(dirs: &Dirs) -> Result<()> {
    println!(
        "Mandrake: corta o que o Claude Code coloca em volta do seu prompt e mede a economia.\n"
    );
    let Ok(p) = Profile::load(&dirs.profile()) else {
        println!("Ainda não instalado nesta máquina. Rode: mandrake setup");
        return Ok(());
    };
    let trim = install::prompt_trim(&dirs.data.join("src"));
    let doc = settings::read(&dirs.claude.join("settings.json")).unwrap_or_default();
    let plugin_on = settings::has_plugin_dir(&doc, &trim.to_string_lossy());
    let proxy_up = std::net::TcpStream::connect(("127.0.0.1", p.proxy.port)).is_ok();
    let mark = |b: bool| if b { "ligado" } else { "DESLIGADO" };
    println!("Como funciona, em duas camadas:");
    println!(
        "  1. prompt-trim (dentro do Claude Code) ........ {}",
        mark(plugin_on)
    );
    println!(
        "     tira o prompt padrão, os lembretes, data, pasta e as descrições longas das tools,"
    );
    println!("     e põe no lugar o core (data/prompt/cores/coding.md) nas sessões sem soul");
    println!(
        "  2. proxy local 127.0.0.1:{} (entre o Claude Code e a API) ... {}",
        p.proxy.port,
        mark(proxy_up)
    );
    println!(
        "     tira as tags <system-reminder> e os campos {}; mede tokens antes e depois",
        p.proxy.strip_fields.join(", ")
    );
    println!("     sessões abertas por um shell com o Mandrake usam o proxy quando ele responde;");
    println!("     se ele estiver fora, vão direto para a API, sem quebrar\n");
    let events = mandrake_meter::read(&dirs.events()).unwrap_or_default();
    let last = events.iter().rev().find(|e| e.tokens_after.is_some());
    println!(
        "Pelo proxy passaram {} requisições.{}",
        events.len(),
        last.map(|e| format!(" Última: {} tokens enviados.", e.tokens_after.unwrap_or(0)))
            .unwrap_or_default()
    );
    let bench = dirs.state.join("bench/latest.json");
    match std::fs::read(&bench)
        .ok()
        .and_then(|b| serde_json::from_slice::<mandrake_meter::BenchReport>(&b).ok())
    {
        Some(r) if r.kind == "ab" => {
            let (b, a): (u64, u64) = r.rows.iter().fold((0, 0), |(b, a), x| {
                (
                    b + x.tokens_before.unwrap_or(0),
                    a + x.tokens_after.unwrap_or(0),
                )
            });
            let pct = if b > 0 {
                100.0 * (b - a.min(b)) as f64 / b as f64
            } else {
                0.0
            };
            println!(
                "Último bench ({}): {} tokens sem Mandrake, {} com, {:.1}% a menos.",
                &r.ts[..10.min(r.ts.len())],
                b,
                a,
                pct
            );
        }
        _ => println!(
            "Para ver o corte: `mandrake compare` mostra o que o Claude Code envia sem e com o Mandrake,\n\
             sem gastar nada. `mandrake bench` mede de verdade, com 2 requisições por prompt."
        ),
    }
    println!(
        "\nPara começar: {}\nAvançado: {}\n(mandrake <comando> --help)",
        crate::help::START.join(" · "),
        crate::help::ADVANCED.join(" · ")
    );
    Ok(())
}
