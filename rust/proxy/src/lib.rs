//! Local proxy between Claude Code and the Anthropic API. Owner: agent A (plan items P1 to P10).
//!
//! The CLI calls only the functions below; everything else is private to this crate.

mod audit;
mod bench;
mod clock;
mod count;
mod log;
mod serve;
mod service;
mod trim;
mod usage;

#[cfg(test)]
mod stream_tests;

use anyhow::Result;
use mandrake_home::{Dirs, Profile};
use std::path::Path;

/// Writes a bench report as `<out>.json` and `<out>.md`.
pub use bench::write as write_bench;
/// `trim` cuts per `cfgs/proxy/trim.toml` and the profile; `pass` forwards untouched and still measures.
pub use mandrake_meter::Mode;

/// Serves until killed. Blocks; builds its own runtime.
pub fn serve(dirs: &Dirs, profile: &Profile, mode: Mode, port: Option<u16>) -> Result<()> {
    let state = serve::State::new(serve::Config::new(dirs, profile, mode))?;
    match audit::sweep(&dirs.audit(), profile.proxy.audit_keep_days) {
        Ok(0) => {}
        Ok(n) => state
            .log
            .line(&format!("{} audit sweep removed={n}", clock::now())),
        Err(e) => state
            .log
            .line(&format!("{} audit sweep error={e:#}", clock::now())),
    }
    serve::serve(state, port.unwrap_or(profile.proxy.port))
}

/// Writes and loads the user service (launchd on macOS, systemd --user on Linux) that runs `exe proxy run`.
pub fn install_service(dirs: &Dirs, profile: &Profile, exe: &Path) -> Result<()> {
    let _ = profile; // the service reads the profile itself when it starts
    let msg = service::install(
        dirs,
        service::Os::current()?,
        exe,
        service::isolated(),
        &service::env_from_process(),
    )?;
    println!("{msg}");
    Ok(())
}

pub fn uninstall_service(dirs: &Dirs) -> Result<()> {
    println!(
        "{}",
        service::uninstall(dirs, service::Os::current()?, service::isolated())?
    );
    Ok(())
}

/// One line for `doctor` and `proxy status`: whether the service is loaded and the port answers.
pub fn status(dirs: &Dirs, profile: &Profile) -> Result<String> {
    Ok(service::status(
        dirs,
        service::Os::current()?,
        service::isolated(),
        profile.proxy.port,
    ))
}

/// Runs the fixtures in `fixtures` through the trim and `count_tokens`, writes `<out>.json` and `<out>.md`.
pub fn bench(dirs: &Dirs, profile: &Profile, fixtures: &Path, out: &Path) -> Result<()> {
    let _ = dirs;
    let api_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let bench_auth = std::env::var("MANDRAKE_BENCH_AUTH").ok();
    let auth = bench::credential(api_key.as_deref(), bench_auth.as_deref())?;
    let files = bench::fixtures(fixtures)?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let report = rt.block_on(bench::measure(
        &files,
        &profile.proxy.strip_fields,
        &profile.proxy.upstream,
        auth.as_ref(),
    ))?;
    let (json, md) = bench::write(&report, out)?;
    if !report.counted {
        eprintln!(
            "bench: no credential (ANTHROPIC_API_KEY or MANDRAKE_BENCH_AUTH), measured bytes only"
        );
    }
    println!("wrote {} and {}", json.display(), md.display());
    Ok(())
}
