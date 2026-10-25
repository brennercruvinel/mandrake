//! `mandrake doctor`: one line per check, in Portuguese, and every failing line ends with the
//! command that fixes it, so a lost link or a missing shell block is a copy and paste away.

use anyhow::Result;
use mandrake_home::{Action, Dirs, Profile};
use mandrake_layout::{Catalog, links, settings};

use crate::install;

/// The command that reapplies the saved profile: links, settings, shell block, plugins, data.
const SETUP: &str = "mandrake setup --yes";

#[derive(Debug, PartialEq, Eq)]
pub enum Mark {
    Ok,
    Fail,
    Note,
}

#[derive(Debug)]
pub struct Check {
    pub mark: Mark,
    pub what: String,
    /// For a failing check: the command that fixes it.
    pub fix: Option<String>,
}

impl Check {
    fn ok(ok: bool, what: String, fix: &str) -> Self {
        Self::with(ok, what, Some(fix.to_string()))
    }

    fn with(ok: bool, what: String, fix: Option<String>) -> Self {
        let mark = if ok { Mark::Ok } else { Mark::Fail };
        Self {
            fix: fix.filter(|_| !ok),
            mark,
            what,
        }
    }

    pub fn line(&self) -> String {
        match (&self.mark, &self.fix) {
            (Mark::Ok, _) => format!("ok   {}", self.what),
            (Mark::Note, _) => format!("nota {}", self.what),
            (Mark::Fail, Some(fix)) => format!("FAIL {} · conserte com: {fix}", self.what),
            (Mark::Fail, None) => format!("FAIL {}", self.what),
        }
    }
}

/// Prints every check; any failure makes the exit code 1.
pub fn run(dirs: &Dirs) -> Result<()> {
    let checks = checks(dirs)?;
    for c in &checks {
        println!("{}", c.line());
    }
    if checks.iter().any(|c| c.mark == Mark::Fail) {
        std::process::exit(1);
    }
    Ok(())
}

pub fn checks(dirs: &Dirs) -> Result<Vec<Check>> {
    let mut out = vec![];
    let Ok(p) = Profile::load(&dirs.profile()) else {
        out.push(Check::ok(
            false,
            format!("perfil {}: não existe", dirs.profile().display()),
            "mandrake setup",
        ));
        return Ok(out);
    };
    out.push(Check::ok(
        true,
        format!("perfil {}", dirs.profile().display()),
        "",
    ));
    links_checks(dirs, &p, &mut out)?;
    out.extend(env_checks(dirs, |k| std::env::var(k).ok()));

    // The installed copy: what env.sh, the launcher and prompt-trim point at, not the checkout.
    let root = dirs.data.join("src");
    let doc = settings::read(&dirs.claude.join("settings.json"))?;
    for (k, v) in settings::wanted(p.proxy.port, &install::prompt_trim(&root)) {
        let have = doc["env"][k].as_str().unwrap_or("vazio");
        let ok = if k == settings::PLUGIN_DIRS {
            settings::has_plugin_dir(&doc, &v)
        } else {
            have == v
        };
        out.push(Check::ok(ok, format!("settings env.{k} = {have}"), SETUP));
    }
    let base = doc["env"][settings::BASE_URL].as_str().unwrap_or("");
    out.push(Check::ok(
        !settings::is_ours(base, p.proxy.port),
        format!(
            "settings sem ANTHROPIC_BASE_URL do Mandrake (com ele, proxy fora derruba todas as sessões){}",
            if base.is_empty() {
                String::new()
            } else {
                format!(": {base}")
            }
        ),
        &format!(
            "tire env.{} de {}",
            settings::BASE_URL,
            dirs.claude.join("settings.json").display()
        ),
    ));
    if !base.is_empty() && !settings::is_ours(base, p.proxy.port) {
        out.push(Check {
            mark: Mark::Note,
            what: format!(
                "settings manda as sessões para outro endereço ({base}): o proxy do Mandrake fica de fora, o prompt-trim continua; para passar pelos dois, ponha upstream = \"{base}\" em [proxy] no perfil"
            ),
            fix: None,
        });
    }
    out.push(Check::ok(
        install::prompt_trim(&root)
            .join("hooks/register.js")
            .is_file(),
        "plugin prompt-trim instalado".into(),
        SETUP,
    ));
    for (name, want) in [
        ("fancy", p.plugins.fancy),
        ("tui-embed", p.plugins.tui_embed),
    ] {
        if want {
            let on = settings::plugin_enabled(&doc, name);
            out.push(Check::ok(
                on.is_some(),
                format!(
                    "plugin {name}: {}",
                    on.unwrap_or_else(|| "não ligado".into())
                ),
                SETUP,
            ));
        }
    }
    if p.plugins.tui_embed {
        for (ok, what, fix) in crate::apps::check(dirs) {
            out.push(Check::with(ok, what, fix));
        }
    }
    let rc = mandrake_layout::shell::rc_file(&dirs.home, std::env::var("SHELL").ok().as_deref());
    let has_block = std::fs::read_to_string(&rc)
        .unwrap_or_default()
        .contains("# >>> mandrake >>>");
    out.push(Check::ok(
        has_block,
        format!("bloco do shell em {}", rc.display()),
        &format!("{SETUP}, depois abra um shell novo"),
    ));
    let souls = install::souls_dir(dirs, &p, &root);
    out.push(Check::ok(
        souls.is_dir(),
        format!("souls {}", souls.display()),
        SETUP,
    ));
    // Under MANDRAKE_HOME the service is written but never loaded, so only `proxy run` can fix it.
    let proxy_fix = if mandrake_proxy::isolated() {
        "mandrake proxy run (com MANDRAKE_HOME o serviço não é carregado)"
    } else {
        "mandrake proxy install"
    };
    out.push(match mandrake_proxy::health(dirs, &p) {
        Ok((ok, line)) => Check::ok(ok, line, proxy_fix),
        Err(e) => Check::ok(false, format!("proxy: {e:#}"), proxy_fix),
    });
    let v = std::process::Command::new("claude")
        .arg("--version")
        .output();
    out.push(Check::ok(
        v.as_ref().is_ok_and(|o| o.status.success()),
        format!(
            "claude {}",
            v.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                .unwrap_or_else(|_| "não encontrado no PATH".into())
        ),
        "instale o Claude Code: https://docs.claude.com/claude-code",
    ));
    Ok(out)
}

/// A `MANDRAKE_*` in this shell that differs from what setup installed: a shell from before a
/// reinstall, or one that inherited an old value (#33). A new shell takes the installed one.
fn env_checks(dirs: &Dirs, get: impl Fn(&str) -> Option<String>) -> Vec<Check> {
    let Ok(text) = std::fs::read_to_string(dirs.data.join("shell/env.sh")) else {
        return vec![];
    };
    let stale: Vec<String> = mandrake_layout::shell::env_vars(&text)
        .into_iter()
        .filter_map(|(k, v)| get(&k).filter(|have| *have != v).map(|have| (k, have, v)))
        .map(|(k, have, v)| format!("{k} é {have}, o instalado é {v}"))
        .collect();
    vec![Check::ok(
        stale.is_empty(),
        if stale.is_empty() {
            "ambiente: os MANDRAKE_* deste shell são os instalados".into()
        } else {
            format!("ambiente: neste shell {}", stale.join("; "))
        },
        "abra um shell novo (exec $SHELL): ele carrega os valores instalados",
    )]
}

fn links_checks(dirs: &Dirs, p: &Profile, out: &mut Vec<Check>) -> Result<()> {
    let steps = links::plan(dirs, &Catalog::builtin()?, p)?;
    let off: Vec<_> = steps
        .iter()
        .filter(|s| s.op.changes())
        .map(|s| s.path.as_str())
        .collect();
    let linked = steps.iter().filter(|s| s.action != Action::Keep).count();
    out.push(Check::ok(
        off.is_empty(),
        format!(
            "links: {} como o perfil diz{}",
            linked - off.len().min(linked),
            if off.is_empty() {
                String::new()
            } else {
                format!("; diferem: {}", off.join(", "))
            }
        ),
        "mandrake link apply",
    ));
    let dangling: Vec<_> = steps
        .iter()
        .filter(|s| s.action == Action::Link && s.target.as_ref().is_some_and(|t| !t.exists()))
        .map(|s| s.path.as_str())
        .collect();
    if !dangling.is_empty() {
        out.push(Check {
            mark: Mark::Note,
            what: format!(
                "destinos de link ainda não criados: {}",
                dangling.join(", ")
            ),
            fix: None,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn dirs(root: &Path) -> Dirs {
        let home = root.join("home");
        Dirs::resolve(|k| match k {
            "HOME" => Some(home.clone()),
            "MANDRAKE_HOME" => Some(root.join("mh")),
            _ => None::<PathBuf>,
        })
        .unwrap()
    }

    #[test]
    fn every_fail_names_its_fix_and_the_lines_speak_one_language() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        let lone = checks(&d).unwrap();
        assert_eq!(lone.len(), 1);
        assert!(
            lone[0].line().ends_with("conserte com: mandrake setup"),
            "{}",
            lone[0].line()
        );

        let mut p = Profile::default();
        p.plugins.fancy = true;
        p.proxy.port = 1;
        std::fs::create_dir_all(&d.config).unwrap();
        p.save(&d.profile()).unwrap();
        let all = checks(&d).unwrap();
        let fails: Vec<_> = all.iter().filter(|c| c.mark == Mark::Fail).collect();
        assert!(fails.len() >= 4, "nothing is installed here: {all:?}");
        for c in &fails {
            assert!(c.line().contains(" · conserte com: "), "{}", c.line());
        }
        let text: String = all.iter().map(|c| c.line() + "\n").collect();
        for english in [
            "installed",
            "not enabled",
            "as the profile says",
            "answering",
            "shell block",
        ] {
            assert!(!text.contains(english), "{english} in:\n{text}");
        }
        assert!(text.contains("bloco do shell em"), "{text}");
        assert!(
            text.contains("FAIL proxy: serviço não instalado; 127.0.0.1:1 não responde"),
            "{text}"
        );
    }

    #[test]
    fn a_folder_linked_to_a_missing_target_is_fixed_by_link_apply() {
        // Since #27 `link apply` fills a missing target first, so the fix is the command itself.
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        std::fs::create_dir_all(d.claude.join("agents")).unwrap();
        std::fs::write(d.claude.join("agents/reviewer.md"), "r").unwrap();
        let mut p = Profile::default();
        p.links.insert(
            "agents".into(),
            mandrake_home::LinkSpec {
                action: Action::Link,
                target: Some(tmp.path().join("dev/agents").display().to_string()),
            },
        );
        let mut out = vec![];
        links_checks(&d, &p, &mut out).unwrap();
        assert_eq!(
            out[0].fix.as_deref(),
            Some("mandrake link apply"),
            "{:?}",
            out[0].fix
        );
        let steps = links::plan(&d, &Catalog::builtin().unwrap(), &p).unwrap();
        links::apply(&d, &Catalog::builtin().unwrap(), &steps).unwrap();
        assert!(d.claude.join("agents/reviewer.md").exists());
        let mut out = vec![];
        links_checks(&d, &p, &mut out).unwrap();
        assert_eq!(out[0].mark, Mark::Ok, "{:?}", out[0].what);
    }

    #[test]
    fn a_stale_mandrake_variable_in_the_shell_is_named() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        std::fs::create_dir_all(d.data.join("shell")).unwrap();
        std::fs::write(
            d.data.join("shell/env.sh"),
            mandrake_layout::shell::env_file(&[
                ("MANDRAKE_SOULS", "/new/souls".into()),
                ("MANDRAKE_PORT", "8787".into()),
            ]),
        )
        .unwrap();
        let c = &env_checks(&d, |k| match k {
            "MANDRAKE_SOULS" => Some("/old".into()),
            "MANDRAKE_PORT" => Some("8787".into()),
            _ => None,
        })[0];
        assert_eq!(c.mark, Mark::Fail);
        assert_eq!(
            c.what,
            "ambiente: neste shell MANDRAKE_SOULS é /old, o instalado é /new/souls"
        );
        assert!(c.fix.as_deref().unwrap().starts_with("abra um shell novo"));
        let c = &env_checks(&d, |_| None)[0];
        assert_eq!(
            c.mark,
            Mark::Ok,
            "a shell that never loaded env.sh is not stale"
        );
    }

    #[test]
    fn souls_are_checked_in_the_installed_copy() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        std::fs::create_dir_all(&d.config).unwrap();
        Profile::default().save(&d.profile()).unwrap();
        let souls = |d: &Dirs| {
            checks(d)
                .unwrap()
                .into_iter()
                .find(|c| c.what.starts_with("souls "))
                .unwrap()
        };
        let installed = d.data.join("src/data/prompt/souls");
        let c = souls(&d);
        assert_eq!(c.what, format!("souls {}", installed.display()));
        assert_eq!(c.mark, Mark::Fail);
        std::fs::create_dir_all(&installed).unwrap();
        assert_eq!(souls(&d).mark, Mark::Ok);
    }
}
