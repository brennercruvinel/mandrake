//! Local proxy between Claude Code and the Anthropic API. Owner: agent A (plan items P1 to P10).
//!
//! The CLI calls only the functions below; everything else is private to this crate.

use anyhow::{Result, bail};
use mandrake_home::{Dirs, Profile};
use std::path::Path;

/// `trim` cuts per `cfgs/proxy/trim.toml` and the profile; `pass` forwards untouched and still measures.
pub use mandrake_meter::Mode;

/// Serves until killed. Blocks; builds its own runtime.
pub fn serve(dirs: &Dirs, profile: &Profile, mode: Mode, port: Option<u16>) -> Result<()> {
    let _ = (dirs, profile, mode, port);
    bail!("proxy: not built yet (plan P1)")
}

/// Writes and loads the user service (launchd on macOS, systemd --user on Linux) that runs `exe proxy run`.
pub fn install_service(dirs: &Dirs, profile: &Profile, exe: &Path) -> Result<()> {
    let _ = (dirs, profile, exe);
    bail!("proxy: service not built yet (plan P8)")
}

pub fn uninstall_service(dirs: &Dirs) -> Result<()> {
    let _ = dirs;
    bail!("proxy: service not built yet (plan P8)")
}

/// One line for `doctor` and `proxy status`: whether the service is loaded and the port answers.
pub fn status(dirs: &Dirs, profile: &Profile) -> Result<String> {
    let _ = (dirs, profile);
    bail!("proxy: status not built yet (plan P8)")
}

/// Runs the fixtures in `fixtures` through the trim and `count_tokens`, writes `<out>.json` and `<out>.md`.
pub fn bench(dirs: &Dirs, profile: &Profile, fixtures: &Path, out: &Path) -> Result<()> {
    let _ = (dirs, profile, fixtures, out);
    bail!("bench: not built yet (plan P9)")
}
