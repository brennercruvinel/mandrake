//! The user service that keeps `mandrake proxy run` up: launchd on macOS, systemd --user on Linux.
//!
//! Only the text generation is tested; `launchctl` and `systemctl` run only from the real commands,
//! and never when `MANDRAKE_HOME` is set.

use anyhow::{Context, Result, bail};
use mandrake_home::Dirs;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub const LABEL: &str = "dev.mandrake.proxy";
pub const UNIT: &str = "mandrake-proxy.service";
const PLIST: &str = include_str!("../../../cfgs/proxy/launchd.plist");
const SYSTEMD: &str = include_str!("../../../cfgs/proxy/systemd.service");

/// Variables the service inherits from the shell that installed it, so it finds the same folders.
pub const PASS_ENV: [&str; 5] = [
    "MANDRAKE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "CLAUDE_CONFIG_DIR",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Mac,
    Linux,
}

impl Os {
    pub fn current() -> Result<Self> {
        match std::env::consts::OS {
            "macos" => Ok(Os::Mac),
            "linux" => Ok(Os::Linux),
            other => bail!("serviço do proxy: {other} não é suportado (só macOS e Linux)"),
        }
    }
}

/// Whether `MANDRAKE_HOME` isolates this run: then the service file lands in `state/service` and nothing loads.
pub fn isolated() -> bool {
    std::env::var_os("MANDRAKE_HOME").is_some_and(|v| !v.is_empty())
}

pub fn env_from_process() -> Vec<(String, String)> {
    let mut env = Vec::new();
    for k in PASS_ENV {
        if let Some(v) = std::env::var_os(k).filter(|v| !v.is_empty()) {
            env.push((k.to_string(), v.to_string_lossy().into_owned()));
        }
    }
    env
}

pub fn file_name(os: Os) -> String {
    match os {
        Os::Mac => format!("{LABEL}.plist"),
        Os::Linux => UNIT.to_string(),
    }
}

/// Where the service file goes, from `dirs.home` (or `dirs.state` when isolated).
pub fn unit_path(dirs: &Dirs, os: Os, isolated: bool) -> PathBuf {
    if isolated {
        return dirs.state.join("service").join(file_name(os));
    }
    match os {
        Os::Mac => dirs.home.join("Library/LaunchAgents").join(file_name(os)),
        Os::Linux => dirs.home.join(".config/systemd/user").join(file_name(os)),
    }
}

pub fn launchd_plist(exe: &Path, stderr: &Path, env: &[(String, String)]) -> String {
    let env: String = env
        .iter()
        .map(|(k, v)| {
            format!(
                "    <key>{}</key>\n    <string>{}</string>\n",
                xml(k),
                xml(v)
            )
        })
        .collect();
    PLIST
        .replace("{{label}}", LABEL)
        .replace("{{exe}}", &xml(&exe.to_string_lossy()))
        .replace("{{stderr}}", &xml(&stderr.to_string_lossy()))
        .replace("{{env}}", &env)
}

pub fn systemd_unit(exe: &Path, env: &[(String, String)]) -> String {
    let env: String = env
        .iter()
        .map(|(k, v)| format!("Environment={}\n", systemd_quote(&format!("{k}={v}"))))
        .collect();
    SYSTEMD
        .replace("{{exe}}", &systemd_quote(&exe.to_string_lossy()))
        .replace("{{env}}", &env)
}

/// The service text for this OS.
pub fn render(dirs: &Dirs, os: Os, exe: &Path, env: &[(String, String)]) -> String {
    match os {
        Os::Mac => launchd_plist(exe, &dirs.logs().join("proxy.stderr.log"), env),
        Os::Linux => systemd_unit(exe, env),
    }
}

/// Writes the service file and, unless isolated, loads it. Returns what it did, for the user.
pub fn install(
    dirs: &Dirs,
    os: Os,
    exe: &Path,
    isolated: bool,
    env: &[(String, String)],
) -> Result<String> {
    let path = unit_path(dirs, os, isolated);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).with_context(|| format!("criando {}", dir.display()))?;
    }
    std::fs::create_dir_all(dirs.logs())?;
    std::fs::write(&path, render(dirs, os, exe, env))
        .with_context(|| format!("escrevendo {}", path.display()))?;
    if isolated {
        return Ok(format!(
            "escrito em {} (MANDRAKE_HOME definido: não carregado)",
            path.display()
        ));
    }
    match os {
        Os::Mac => {
            let domain = gui_domain()?;
            // A previous copy may be loaded; bootstrap fails on a label that is already there.
            let _ = quiet("launchctl", &["bootout", &format!("{domain}/{LABEL}")]);
            run(
                "launchctl",
                &["bootstrap", &domain, &path.to_string_lossy()],
            )?;
        }
        Os::Linux => {
            run("systemctl", &["--user", "daemon-reload"])?;
            run("systemctl", &["--user", "enable", UNIT])?;
            run("systemctl", &["--user", "restart", UNIT])?;
        }
    }
    Ok(format!("escrito e carregado: {}", path.display()))
}

pub fn uninstall(dirs: &Dirs, os: Os, isolated: bool) -> Result<String> {
    let path = unit_path(dirs, os, isolated);
    if !isolated {
        match os {
            Os::Mac => {
                let _ = quiet(
                    "launchctl",
                    &["bootout", &format!("{}/{LABEL}", gui_domain()?)],
                );
            }
            Os::Linux => {
                let _ = quiet("systemctl", &["--user", "disable", "--now", UNIT]);
            }
        }
    }
    let existed = match std::fs::remove_file(&path) {
        Ok(()) => true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e).with_context(|| format!("removendo {}", path.display())),
    };
    if !isolated && os == Os::Linux {
        let _ = quiet("systemctl", &["--user", "daemon-reload"]);
    }
    Ok(if existed {
        format!("removido: {}", path.display())
    } else {
        format!("{} não existia", path.display())
    })
}

/// One line: whether the service is installed and loaded, and whether the port takes a TCP connect.
pub fn status(dirs: &Dirs, os: Os, isolated: bool, port: u16) -> String {
    health(dirs, os, isolated, port).1
}

/// The status line, and whether all is well: the service loaded and the port answering.
pub fn health(dirs: &Dirs, os: Os, isolated: bool, port: u16) -> (bool, String) {
    let path = unit_path(dirs, os, isolated);
    let (loaded, service) = if !path.exists() {
        (false, "serviço não instalado".to_string())
    } else if isolated {
        (
            false,
            format!(
                "serviço escrito em {} (MANDRAKE_HOME definido: não carregado)",
                path.display()
            ),
        )
    } else {
        let (kind, up) = match os {
            Os::Mac => (
                "launchd",
                gui_domain().is_ok_and(|d| quiet("launchctl", &["print", &format!("{d}/{LABEL}")])),
            ),
            Os::Linux => (
                "systemd --user",
                quiet("systemctl", &["--user", "is-active", "--quiet", UNIT]),
            ),
        };
        let name = if os == Os::Mac { LABEL } else { UNIT };
        let state = if up {
            "carregado"
        } else {
            "instalado, não carregado"
        };
        (up, format!("{kind} {name} {state}"))
    };
    let answering = answers(port);
    let port_state = if answering {
        "respondendo"
    } else {
        "não responde"
    };
    // Isolated, nothing is ever loaded: a proxy answering on the port (`proxy run`) is the healthy state.
    (
        (loaded || isolated) && answering,
        format!("proxy: {service}; 127.0.0.1:{port} {port_state}"),
    )
}

pub fn answers(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::from(([127, 0, 0, 1], port)),
        Duration::from_millis(500),
    )
    .is_ok()
}

fn gui_domain() -> Result<String> {
    let out = Command::new("id")
        .arg("-u")
        .output()
        .context("rodando id -u")?;
    let uid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || uid.is_empty() {
        bail!("não deu para ler o id do usuário");
    }
    Ok(format!("gui/{uid}"))
}

fn run(cmd: &str, args: &[&str]) -> Result<()> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("running {cmd}"))?;
    if !out.status.success() {
        bail!(
            "{cmd} {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

fn quiet(cmd: &str, args: &[&str]) -> bool {
    Command::new(cmd)
        .args(args)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// A double-quoted systemd word: `\` and `"` escaped, `%` and `$` doubled so they stay literal.
fn systemd_quote(s: &str) -> String {
    let inner = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "$$");
    format!("\"{inner}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(root: &Path) -> Dirs {
        let root = root.to_path_buf();
        Dirs::resolve(move |k| match k {
            "HOME" => Some(root.join("home")),
            "MANDRAKE_HOME" => Some(root.join("mh")),
            _ => None,
        })
        .unwrap()
    }

    #[test]
    fn plist_runs_proxy_and_escapes() {
        let env = vec![("XDG_STATE_HOME".to_string(), "/x/a&b".to_string())];
        let p = launchd_plist(
            Path::new("/Apps/My <Tools>/mandrake"),
            Path::new("/s/proxy.stderr.log"),
            &env,
        );
        assert!(p.contains("<string>dev.mandrake.proxy</string>"));
        assert!(p.contains("<string>/Apps/My &lt;Tools&gt;/mandrake</string>\n    <string>proxy</string>\n    <string>run</string>"));
        assert!(p.contains("<key>RunAtLoad</key>\n  <true/>"));
        assert!(p.contains("<key>KeepAlive</key>\n  <true/>"));
        assert!(p.contains("<key>XDG_STATE_HOME</key>\n    <string>/x/a&amp;b</string>"));
        assert!(p.contains("<string>/s/proxy.stderr.log</string>"));
        assert!(!p.contains("{{"));
    }

    #[test]
    fn unit_runs_proxy_and_quotes() {
        let env = vec![("MANDRAKE_HOME".to_string(), "/t/100%".to_string())];
        let u = systemd_unit(Path::new("/opt/my tools/mandrake"), &env);
        assert!(u.contains("ExecStart=\"/opt/my tools/mandrake\" proxy run\n"));
        assert!(u.contains("Environment=\"MANDRAKE_HOME=/t/100%%\"\n"));
        assert!(u.contains("Restart=always"));
        assert!(u.contains("WantedBy=default.target"));
        assert!(!u.contains("{{"));
        assert!(!systemd_unit(Path::new("/m"), &[]).contains("Environment="));
    }

    #[test]
    fn paths_hang_from_home_or_state() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        assert_eq!(
            unit_path(&d, Os::Mac, false),
            tmp.path()
                .join("home/Library/LaunchAgents/dev.mandrake.proxy.plist")
        );
        assert_eq!(
            unit_path(&d, Os::Linux, false),
            tmp.path()
                .join("home/.config/systemd/user/mandrake-proxy.service")
        );
        assert_eq!(
            unit_path(&d, Os::Linux, true),
            tmp.path().join("mh/state/service/mandrake-proxy.service")
        );
    }

    #[test]
    fn isolated_install_only_writes_into_state() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        for os in [Os::Mac, Os::Linux] {
            let msg = install(&d, os, Path::new("/bin/mandrake"), true, &[]).unwrap();
            assert!(msg.contains("não carregado"), "{msg}");
            let path = unit_path(&d, os, true);
            assert!(
                std::fs::read_to_string(&path)
                    .unwrap()
                    .contains("/bin/mandrake")
            );
            assert!(status(&d, os, true, 1).contains("MANDRAKE_HOME definido"));
            assert!(
                !health(&d, os, true, 1).0,
                "an isolated service is never loaded"
            );
            let up = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = up.local_addr().unwrap().port();
            assert!(
                health(&d, os, true, port).0,
                "isolated, a proxy answering the port is healthy"
            );
            assert!(uninstall(&d, os, true).unwrap().starts_with("removido"));
            assert!(
                status(&d, os, true, 1)
                    .starts_with("proxy: serviço não instalado; 127.0.0.1:1 não responde")
            );
        }
        assert!(!tmp.path().join("home").exists());
    }
}
