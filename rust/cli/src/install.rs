//! What `setup` puts on the machine besides the links: Mandrake's data, the settings keys, the shell
//! block, the optional plugins and skill. Each step says what it changed.

use anyhow::{Context, Result, bail};
use mandrake_home::{Dirs, Profile};
use mandrake_layout::{settings, shell};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The source tree: `MANDRAKE_SRC`, else the checkout the binary was built in, else the installed copy.
pub fn source(dirs: &Dirs) -> Result<PathBuf> {
    let is_src =
        |p: &Path| p.join("cfgs/setup/items.toml").is_file() && p.join("data/plugins").is_dir();
    if let Some(s) = std::env::var_os("MANDRAKE_SRC").map(PathBuf::from) {
        if is_src(&s) {
            return Ok(s);
        }
        bail!("MANDRAKE_SRC={} não é uma cópia do Mandrake", s.display());
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(p) = exe.ancestors().find(|p| is_src(p))
    {
        return Ok(p.to_path_buf());
    }
    let installed = dirs.data.join("src");
    if is_src(&installed) {
        return Ok(installed);
    }
    bail!("não achei os arquivos do Mandrake: rode de dentro da cópia, ou defina MANDRAKE_SRC")
}

/// Copies `data/`, `cfgs/` and the bench fixtures into the data folder, replacing the previous copy; returns the root.
pub fn data(dirs: &Dirs, src: &Path) -> Result<PathBuf> {
    let root = dirs.data.join("src");
    if root == src {
        return Ok(root);
    }
    for part in ["data", "cfgs", "tool/tests/fixtures"] {
        let to = root.join(part);
        if to.exists() {
            std::fs::remove_dir_all(&to).with_context(|| format!("clearing {}", to.display()))?;
        }
        copy_tree(&src.join(part), &to)?;
    }
    Ok(root)
}

pub fn prompt_trim(root: &Path) -> PathBuf {
    root.join("data/plugins/prompt-trim")
}

/// The two env keys in the Claude Code `settings.json`.
pub fn settings_keys(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
    let keys = settings::wanted(profile.proxy.port, &prompt_trim(root));
    let changed = settings::apply(
        &dirs.claude.join("settings.json"),
        &keys,
        profile.proxy.port,
        &dirs.backups().join(dirs.fresh_stamp()),
    )?;
    Ok(changed
        .into_iter()
        .map(|(k, old, new)| {
            format!(
                "settings env.{k}: {} → {new}",
                old.unwrap_or_else(|| "vazio".into())
            )
        })
        .collect())
}

/// The souls folder the shell uses: the profile's, else the ones shipped.
pub fn souls_dir(dirs: &Dirs, profile: &Profile, root: &Path) -> PathBuf {
    match &profile.souls.dir {
        Some(d) => dirs.expand(d, profile.dev_root(dirs).as_deref()),
        None => root.join("data/prompt/souls"),
    }
}

/// The core prompt the launcher puts before a soul, if any.
pub fn core_file(dirs: &Dirs, profile: &Profile, root: &Path) -> Option<PathBuf> {
    match profile.souls.core.as_deref() {
        Some("none") => None,
        Some(c) => Some(dirs.expand(c, profile.dev_root(dirs).as_deref())),
        None => Some(root.join("data/prompt/cores/coding.md")),
    }
}

/// The core text into the installed prompt-trim (`hooks/core.js`), for sessions without a soul.
pub fn core(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
    let text = match core_file(dirs, profile, root) {
        Some(f) => {
            std::fs::read_to_string(&f).with_context(|| format!("lendo o core {}", f.display()))?
        }
        None => String::new(),
    };
    let js = format!(
        "// Written by `mandrake setup` from the profile's core; edit that file and run setup again.\nexport const CORE = {}\n",
        serde_json::to_string(&text)?
    );
    std::fs::write(prompt_trim(root).join("hooks/core.js"), js)?;
    Ok(vec![if text.is_empty() {
        "nenhum core: sessões sem soul seguem sem system prompt".into()
    } else {
        format!(
            "{} caracteres nas sessões sem soul{}",
            text.len(),
            if profile.souls.core_only_plain {
                ""
            } else {
                " e antes de cada soul"
            }
        )
    }])
}

/// `shell/env.sh`, the shell file, and the block in the rc.
pub fn shell(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
    let sh = dirs.data.join("shell");
    std::fs::create_dir_all(&sh)?;
    let mut vars = vec![
        (
            "MANDRAKE_SOULS",
            souls_dir(dirs, profile, root)
                .to_string_lossy()
                .into_owned(),
        ),
        (
            "MANDRAKE_INVOKE",
            if profile.skills.invoke {
                "1".into()
            } else {
                "0".into()
            },
        ),
        ("MANDRAKE_PORT", profile.proxy.port.to_string()),
        (
            "MANDRAKE_CORE_SOULS",
            if profile.souls.core_only_plain {
                "0".into()
            } else {
                "1".into()
            },
        ),
        (
            "MANDRAKE_CORE",
            core_file(dirs, profile, root)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        ),
    ];
    // Where install.sh and tui-embed put binaries; macOS leaves it off PATH, so the shell file adds it.
    // The prefix goes in too: a later `mandrake setup` from a new shell then keeps a custom
    // MANDRAKE_PREFIX instead of falling back to ~/.local and losing the binary from PATH.
    let prefix = crate::apps::prefix(dirs);
    vars.push(("MANDRAKE_PREFIX", prefix.to_string_lossy().into_owned()));
    vars.push((
        "MANDRAKE_BIN_DIR",
        prefix.join("bin").to_string_lossy().into_owned(),
    ));
    if let Some(m) = &profile.skills.memory_cmd {
        vars.push((
            "MANDRAKE_MEMORY_CMD",
            dirs.expand(m, profile.dev_root(dirs).as_deref())
                .to_string_lossy()
                .into_owned(),
        ));
    }
    std::fs::write(sh.join("env.sh"), shell::env_file(&vars))?;
    let script = sh.join("mandrake.sh");
    std::fs::copy(root.join("cfgs/shell/mandrake.sh"), &script)?;
    // The functions in mandrake.sh call it; fs::copy keeps its executable bit.
    std::fs::copy(root.join("cfgs/shell/soul"), sh.join("soul"))?;
    let rc = shell::rc_file(&dirs.home, std::env::var("SHELL").ok().as_deref());
    let mut out = vec![];
    if shell::install(&rc, &script)? {
        out.push(format!("shell: bloco acrescentado em {}", rc.display()));
    }
    Ok(out)
}

/// /invoke and the other skills only a person starts, for each CLI found besides Claude Code. `soul`
/// writes them, so they follow the same rules as the launcher, and they stay thin: each one calls
/// `soul skill`, which reads the skill at run time.
pub fn harnesses(dirs: &Dirs, profile: &Profile) -> Result<Vec<String>> {
    if !profile.skills.invoke {
        return Ok(vec![]);
    }
    let out = Command::new(dirs.data.join("shell/soul"))
        .arg("install")
        .env("HOME", &dirs.home)
        .env("CLAUDE_CONFIG_DIR", &dirs.claude)
        .output()
        .context("rodando soul install")?;
    if !out.status.success() {
        bail!(
            "soul install falhou: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// fancy and tui-embed through the `mandrake` marketplace, skipped when a plugin of that name is
/// already on from any marketplace (two fancy would draw every reply twice). Without Claude Code
/// there is nothing to install them into, and the rest of the setup still applies.
pub fn plugins(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
    if crate::apps::on_path("claude", std::env::var_os("PATH").as_ref()).is_none() {
        return Ok(vec!["plugins: sem Claude Code, ficaram de fora".into()]);
    }
    let doc = settings::read(&dirs.claude.join("settings.json"))?;
    let want: Vec<&str> = [
        ("fancy", profile.plugins.fancy),
        ("tui-embed", profile.plugins.tui_embed),
    ]
    .into_iter()
    .filter_map(|(n, w)| w.then_some(n))
    .collect();
    let mut out = vec![];
    let mut added = false;
    for name in want {
        if let Some(on) = settings::plugin_enabled(&doc, name) {
            out.push(format!("plugin {name}: já ligado como {on}, mantido"));
            continue;
        }
        if !added {
            claude(&[
                "plugin",
                "marketplace",
                "add",
                &root.join("data/plugins").to_string_lossy(),
            ])?;
            added = true;
        }
        claude(&[
            "plugin",
            "install",
            &format!("{name}@mandrake"),
            "--scope",
            "user",
        ])?;
        out.push(format!(
            "plugin {name}: instalado do marketplace do mandrake"
        ));
    }
    if profile.plugins.fancy
        && root.join("data/plugins/fancy/tint").is_dir()
        && !root.join("data/plugins/fancy/bin/fancy-tint").exists()
    {
        let ok = Command::new("go")
            .args(["build", "-o", "../bin/", "./cmd/fancy-tint"])
            .current_dir(root.join("data/plugins/fancy/tint"))
            .status()
            .is_ok_and(|s| s.success());
        out.push(if ok {
            "plugin fancy: cor da aba compilada".into()
        } else {
            "plugin fancy: sem Go, a cor da aba ficou de fora".into()
        });
    }
    Ok(out)
}

fn claude(args: &[&str]) -> Result<()> {
    let st = Command::new("claude")
        .args(args)
        .status()
        .context("rodando claude (o Claude Code está instalado?)")?;
    if !st.success() {
        bail!("claude {} falhou", args.join(" "));
    }
    Ok(())
}

/// The `invoke` skill, unless the skills folder already has one.
pub fn skill(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
    if !profile.skills.invoke {
        return Ok(vec![]);
    }
    let at = dirs.claude.join("skills/invoke");
    if at.join("SKILL.md").exists() {
        return Ok(vec!["skill invoke: já existe uma, mantida".into()]);
    }
    copy_tree(&root.join("data/skills/invoke"), &at)?;
    Ok(vec![format!("skill invoke: instalada em {}", at.display())])
}

pub fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    let m = std::fs::metadata(from).with_context(|| format!("reading {}", from.display()))?;
    if m.is_file() {
        if let Some(d) = to.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::copy(from, to).with_context(|| format!("copying {}", from.display()))?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let name = e.file_name();
        if name == ".DS_Store" || name == "node_modules" {
            continue;
        }
        copy_tree(&e.path(), &to.join(name))?;
    }
    Ok(())
}
