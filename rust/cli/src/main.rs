//! `mandrake`: one binary, one subcommand per job.

mod ab;
mod apps;
mod ask;
mod cmd;
mod compare;
mod doctor;
mod help;
mod install;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

const ABOUT: &str =
    "Mandrake corta o que o Claude Code coloca em volta do seu prompt, mede quanto isso economiza
e refaz a sua pasta ~/.claude em qualquer máquina a partir de um perfil.

Sem argumentos, mostra o estado desta máquina. Comece por `mandrake setup`, confira com
`mandrake doctor` e veja o corte com `mandrake compare`, que não gasta nada.";

#[derive(Parser)]
#[command(name = "mandrake", version, about = "Corta, mede e reproduz o harness do Claude Code", long_about = ABOUT)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Instala ou ajusta, item por item (--cli em linhas; --profile f --yes sem perguntar)
    Setup {
        /// Começa deste perfil em vez do salvo
        #[arg(long)]
        profile: Option<PathBuf>,
        /// Aplica sem perguntar
        #[arg(long)]
        yes: bool,
        /// Perguntas uma por linha, em vez da tela cheia
        #[arg(long)]
        cli: bool,
    },
    /// Confere tudo: links, settings, plugins, shell, proxy e Claude Code
    Doctor,
    /// Mede de verdade pelo claude -p, sem e com Mandrake (gasta 2 requisições por prompt)
    Bench {
        /// Mede só o corte do proxy sobre corpos de requisição desta pasta (sem rodar o Claude Code)
        #[arg(long)]
        fixtures: Option<PathBuf>,
        #[arg(long, hide = true)]
        out: Option<PathBuf>,
        /// Mostra o último relatório em vez de medir
        #[arg(long)]
        view: bool,
        /// Com --view: imprime a tela como texto, LARGURAxALTURA
        #[arg(long)]
        snapshot: Option<String>,
    },
    /// O que o Claude Code envia sem e com o Mandrake, lado a lado (não gasta nada)
    Compare {
        /// O prompt enviado nos dois lados
        #[arg(default_value = compare::PROMPT)]
        prompt: String,
        /// Imprime a tabela em vez de abrir a tela
        #[arg(long)]
        print: bool,
        /// Imprime a tela como texto, LARGURAxALTURA
        #[arg(long, hide = true)]
        snapshot: Option<String>,
    },
    /// Tela ao vivo das requisições: sem Mandrake à esquerda, com Mandrake à direita
    Watch {
        /// Para rodar dentro de um painel do tui-embed
        #[arg(long)]
        embed: bool,
        /// Imprime a tela como texto, LARGURAxALTURA
        #[arg(long)]
        snapshot: Option<String>,
    },
    /// Uma requisição antes e depois do corte, lado a lado em hex (precisa de auditoria ligada)
    Hex {
        /// O id, de `mandrake audit`; sem ele, a requisição mais recente
        id: Option<String>,
        #[arg(long)]
        snapshot: Option<String>,
        #[arg(long, default_value_t = 0)]
        hunk: usize,
    },
    /// Lista as requisições cujo corpo foi guardado (com [proxy] audit = true no perfil)
    Audit {
        #[arg(long, default_value_t = 20)]
        last: usize,
    },
    /// Escreve a pasta ~/.claude atual como perfil, para levar a outra máquina
    Capture {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Links da pasta ~/.claude: plan mostra, apply aplica, rollback desfaz
    Link {
        #[command(subcommand)]
        op: LinkOp,
    },
    /// O proxy local: run, install, uninstall, status
    Proxy {
        #[command(subcommand)]
        op: ProxyOp,
    },
}

#[derive(Subcommand)]
enum LinkOp {
    /// Mostra o que mudaria, sem tocar em nada
    Plan,
    /// Aplica; o que estiver no caminho vai para o backup
    Apply,
    /// Desfaz o último apply
    Rollback,
}

#[derive(Subcommand)]
enum ProxyOp {
    /// Roda em primeiro plano
    Run {
        /// Só mede, não corta
        #[arg(long)]
        pass: bool,
        /// A porta (padrão: a do perfil)
        #[arg(long, value_name = "PORTA")]
        port: Option<u16>,
    },
    /// Instala como serviço do usuário (launchd ou systemd)
    Install,
    /// Remove o serviço
    Uninstall,
    /// Diz se está no ar
    Status,
}

fn print(text: anyhow::Result<String>) -> anyhow::Result<()> {
    println!("{}", text?);
    Ok(())
}

/// Errors in one line, in Portuguese, with what caused them after a colon.
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let matches = match help::localize(Cli::command()).try_get_matches() {
        Ok(m) => m,
        Err(e) => match help::error_line(&e) {
            Some(line) => anyhow::bail!(line),
            None => e.exit(),
        },
    };
    let cli = Cli::from_arg_matches(&matches)?;
    let dirs = mandrake_home::Dirs::from_env()?;
    let Some(command) = cli.cmd else {
        return cmd::status(&dirs);
    };
    match command {
        Cmd::Setup { profile, yes, cli } => {
            let asking = if yes {
                cmd::Asking::No
            } else if cli {
                cmd::Asking::Cli
            } else {
                cmd::Asking::Tui
            };
            cmd::setup(&dirs, profile, asking)
        }
        Cmd::Capture { out } => cmd::capture(&dirs, out),
        Cmd::Link { op: LinkOp::Plan } => cmd::link_plan(&dirs),
        Cmd::Link { op: LinkOp::Apply } => cmd::link_apply(&dirs),
        Cmd::Link {
            op: LinkOp::Rollback,
        } => cmd::link_rollback(&dirs),
        Cmd::Proxy { op } => {
            let p = cmd::profile(&dirs)?;
            match op {
                ProxyOp::Run { pass, port } => {
                    let mode = if pass {
                        mandrake_meter::Mode::Pass
                    } else {
                        mandrake_meter::Mode::Trim
                    };
                    mandrake_proxy::serve(&dirs, &p, mode, port)
                }
                ProxyOp::Install => {
                    mandrake_proxy::install_service(&dirs, &p, &std::env::current_exe()?)
                }
                ProxyOp::Uninstall => mandrake_proxy::uninstall_service(&dirs),
                ProxyOp::Status => {
                    println!("{}", mandrake_proxy::status(&dirs, &p)?);
                    Ok(())
                }
            }
        }
        Cmd::Compare {
            prompt,
            print,
            snapshot,
        } => {
            let p = cmd::profile(&dirs)?;
            let trim = install::prompt_trim(&dirs.data.join("src"));
            let shown = match snapshot {
                Some(s) => compare::Shown::Snapshot(s),
                None if print => compare::Shown::Table,
                None => compare::Shown::Screen,
            };
            compare::run(&dirs, &p, &trim, &prompt, shown)
        }
        Cmd::Watch {
            snapshot: Some(s), ..
        } => print(mandrake_tui::watch_snapshot(&dirs, &s)),
        Cmd::Watch { embed, .. } => mandrake_tui::watch(&dirs, mandrake_tui::WatchOpts { embed }),
        Cmd::Hex {
            id,
            snapshot: Some(s),
            hunk,
        } => print(mandrake_tui::hex_snapshot(&dirs, id.as_deref(), &s, hunk)),
        Cmd::Hex { id, .. } => mandrake_tui::hex(&dirs, id.as_deref()),
        Cmd::Bench {
            fixtures,
            out,
            view,
            snapshot,
        } => {
            let out = out.unwrap_or_else(|| dirs.state.join("bench/latest"));
            if !view && fixtures.is_none() {
                let p = cmd::profile(&dirs)?;
                let trim = install::prompt_trim(&dirs.data.join("src"));
                return ab::run(&dirs, &p, &trim, &out).map(|_| ());
            }
            if let (true, Some(s)) = (view, &snapshot) {
                print(mandrake_tui::bench_snapshot(&out.with_extension("json"), s))
            } else if view {
                mandrake_tui::bench_view(&out.with_extension("json"))
            } else {
                let fixtures = match fixtures {
                    Some(f) => f,
                    None => install::source(&dirs)?.join("tool/tests/fixtures"),
                };
                mandrake_proxy::bench(&dirs, &cmd::profile(&dirs)?, &fixtures, &out)
            }
        }
        Cmd::Audit { last } => cmd::audit(&dirs, last),
        Cmd::Doctor => doctor::run(&dirs),
    }
}
