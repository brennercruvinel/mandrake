//! `mandrake`: one binary, one subcommand per job.

mod cmd;
mod install;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "mandrake",
    version,
    about = "Claude Code's harness: trimmed, measured, reproducible"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Interactive setup, or `--profile <file> --yes` to apply a saved one
    Setup {
        #[arg(long)]
        profile: Option<PathBuf>,
        /// Apply without asking
        #[arg(long)]
        yes: bool,
    },
    /// Write the current Claude Code folder as a profile
    Capture {
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Links of the Claude Code folder: plan, apply, rollback
    Link {
        #[command(subcommand)]
        op: LinkOp,
    },
    /// The local proxy
    Proxy {
        #[command(subcommand)]
        op: ProxyOp,
    },
    /// Live split view: untrimmed on the left, trimmed on the right
    Watch {
        #[arg(long)]
        embed: bool,
        /// Print one frame as text, WIDTHxHEIGHT, instead of opening the screen
        #[arg(long)]
        snapshot: Option<String>,
    },
    /// Side-by-side hex of one request before and after the trim
    Hex {
        id: String,
        #[arg(long)]
        snapshot: Option<String>,
        /// With --snapshot: scroll to this changed stretch first
        #[arg(long, default_value_t = 0)]
        hunk: usize,
    },
    /// Token savings over fixed requests
    Bench {
        /// Request bodies to measure; default: the fixtures shipped with Mandrake
        #[arg(long)]
        fixtures: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        /// Draw the report instead of running
        #[arg(long)]
        view: bool,
        /// With --view: print one frame as text, WIDTHxHEIGHT
        #[arg(long)]
        snapshot: Option<String>,
    },
    /// Requests with audit bodies kept, newest first
    Audit {
        #[arg(long, default_value_t = 20)]
        last: usize,
    },
    /// Checks links, settings, plugins and the proxy
    Doctor,
}

#[derive(Subcommand)]
enum LinkOp {
    Plan,
    Apply,
    Rollback,
}

#[derive(Subcommand)]
enum ProxyOp {
    /// Serve in the foreground
    Run {
        #[arg(long)]
        pass: bool,
        #[arg(long)]
        port: Option<u16>,
    },
    Install,
    Uninstall,
    Status,
}

fn print(text: anyhow::Result<String>) -> anyhow::Result<()> {
    println!("{}", text?);
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let dirs = mandrake_home::Dirs::from_env()?;
    match cli.cmd {
        Cmd::Setup { profile, yes } => cmd::setup(&dirs, profile, yes),
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
        Cmd::Watch {
            snapshot: Some(s), ..
        } => print(mandrake_tui::watch_snapshot(&dirs, &s)),
        Cmd::Watch { embed, .. } => mandrake_tui::watch(&dirs, mandrake_tui::WatchOpts { embed }),
        Cmd::Hex {
            id,
            snapshot: Some(s),
            hunk,
        } => print(mandrake_tui::hex_snapshot(&dirs, &id, &s, hunk)),
        Cmd::Hex { id, .. } => mandrake_tui::hex(&dirs, &id),
        Cmd::Bench {
            fixtures,
            out,
            view,
            snapshot,
        } => {
            let out = out.unwrap_or_else(|| dirs.state.join("bench/latest"));
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
        Cmd::Doctor => cmd::doctor(&dirs),
    }
}
