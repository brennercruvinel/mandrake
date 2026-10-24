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
        bail!("MANDRAKE_SRC={} is not a Mandrake checkout", s.display());
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
    bail!("no Mandrake source found: run from the checkout, or set MANDRAKE_SRC")
}

/// Copies `data/` and `cfgs/` into the data folder, replacing the previous copy; returns the root.
pub fn data(dirs: &Dirs, src: &Path) -> Result<PathBuf> {
    let root = dirs.data.join("src");
    if root == src {
        return Ok(root);
    }
    for part in ["data", "cfgs"] {
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
        &dirs.backups().join(mandrake_home::stamp()),
    )?;
    Ok(changed
        .into_iter()
        .map(|(k, old, new)| {
            format!(
                "settings env.{k}: {} -> {new}",
                old.unwrap_or_else(|| "unset".into())
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
    ];
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
    let rc = shell::rc_file(&dirs.home, std::env::var("SHELL").ok().as_deref());
    let mut out = vec![];
    if shell::install(&rc, &script)? {
        out.push(format!("shell: block added to {}", rc.display()));
    }
    Ok(out)
}

/// fancy and tui-embed through the `mandrake` marketplace, skipped when a plugin of that name is
/// already on from any marketplace (two fancy would draw every reply twice).
pub fn plugins(dirs: &Dirs, profile: &Profile, root: &Path) -> Result<Vec<String>> {
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
            out.push(format!("plugin {name}: already on as {on}, skipped"));
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
            "plugin {name}: installed from the mandrake marketplace"
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
            "plugin fancy: tab tint built".into()
        } else {
            "plugin fancy: no Go, tab tint left out".into()
        });
    }
    Ok(out)
}

fn claude(args: &[&str]) -> Result<()> {
    let st = Command::new("claude")
        .args(args)
        .status()
        .context("running claude (is Claude Code installed?)")?;
    if !st.success() {
        bail!("claude {} failed", args.join(" "));
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
        return Ok(vec!["skill invoke: one is already there, kept".into()]);
    }
    copy_tree(&root.join("data/skills/invoke"), &at)?;
    Ok(vec![format!("skill invoke: installed in {}", at.display())])
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
