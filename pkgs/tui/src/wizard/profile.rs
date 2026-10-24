//! The profile the wizard's answers describe, and how it differs from the current one.

use super::state::{ItemChoice, Wizard};
use mandrake_home::{Action, LinkSpec, Profile};

/// One line of the summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub what: String,
    pub before: String,
    pub after: String,
    /// The entry holds conversations and is being linked or copied.
    pub sensitive: bool,
}

pub fn action_label(a: Action) -> &'static str {
    match a {
        Action::Keep => "manter",
        Action::Link => "linkar",
        Action::Copy => "copiar",
    }
}

fn on_off(b: bool) -> String {
    if b {
        "ligado".into()
    } else {
        "desligado".into()
    }
}

fn or_dash(v: Option<&str>, dash: &str) -> String {
    v.map(str::to_string).unwrap_or_else(|| dash.to_string())
}

fn spec_label(spec: Option<&LinkSpec>) -> String {
    match spec {
        None
        | Some(LinkSpec {
            action: Action::Keep,
            ..
        }) => "manter".into(),
        Some(LinkSpec { action, target }) => {
            format!(
                "{} {}",
                action_label(*action),
                target.as_deref().unwrap_or("?")
            )
        }
    }
}

impl Wizard {
    /// Where an item's target lands on this machine, for display.
    pub fn resolved(&self, item: &ItemChoice) -> Option<String> {
        let t = item.target.some()?;
        let root = self.dev_root.some().map(|r| self.dirs.expand(&r, None));
        let relative = !t.starts_with('/') && !t.starts_with('~');
        if relative && root.is_none() {
            return None;
        }
        Some(self.dirs.expand(&t, root.as_deref()).display().to_string())
    }

    /// The profile the answers describe, starting from the current one so that whatever the wizard
    /// does not ask about (links outside the catalog, upstream, aliases) carries over untouched.
    pub fn build(&self) -> Result<Profile, String> {
        self.check(super::state::Step::Proxy)?;
        self.check(super::state::Step::Items)?;
        let mut p = self.current.clone();
        p.dev_root = self.dev_root.some();
        for i in &self.items {
            match i.action {
                Action::Keep if self.current.links.contains_key(&i.path) => {
                    p.links.insert(
                        i.path.clone(),
                        LinkSpec {
                            action: Action::Keep,
                            target: None,
                        },
                    );
                }
                Action::Keep => {
                    p.links.remove(&i.path);
                }
                action => {
                    p.links.insert(
                        i.path.clone(),
                        LinkSpec {
                            action,
                            target: i.target.some(),
                        },
                    );
                }
            }
        }
        p.plugins.fancy = self.fancy;
        p.plugins.tui_embed = self.tui_embed;
        p.proxy.port = self
            .port
            .value()
            .parse()
            .map_err(|_| "porta inválida".to_string())?;
        p.proxy.strip_fields.retain(|f| f != "safeguards");
        if self.strip_safeguards {
            p.proxy.strip_fields.push("safeguards".into());
        }
        p.proxy.audit = self.audit;
        if let Ok(days) = self.keep_days.value().parse() {
            p.proxy.audit_keep_days = days;
        }
        p.souls.dir = self.souls_dir.some();
        p.skills.invoke = self.invoke;
        p.skills.memory_cmd = self.memory_cmd.some();
        Ok(p)
    }

    /// What confirming would change, in the order the wizard asked.
    pub fn changes(&self) -> Vec<Change> {
        let Ok(new) = self.build() else { return vec![] };
        let old = &self.current;
        let mut out = vec![];
        let mut push = |what: &str, before: String, after: String, sensitive: bool| {
            if before != after {
                out.push(Change {
                    what: what.into(),
                    before,
                    after,
                    sensitive,
                });
            }
        };
        push(
            "pasta de desenvolvimento",
            or_dash(old.dev_root.as_deref(), "(nenhuma)"),
            or_dash(new.dev_root.as_deref(), "(nenhuma)"),
            false,
        );
        for i in &self.items {
            let after = new.links.get(&i.path);
            let linked = after.is_some_and(|s| s.action != Action::Keep);
            push(
                &i.path,
                spec_label(old.links.get(&i.path)),
                spec_label(after),
                i.sensitive && linked,
            );
        }
        push(
            "plugin fancy",
            on_off(old.plugins.fancy),
            on_off(new.plugins.fancy),
            false,
        );
        push(
            "plugin tui-embed",
            on_off(old.plugins.tui_embed),
            on_off(new.plugins.tui_embed),
            false,
        );
        push(
            "porta do proxy",
            old.proxy.port.to_string(),
            new.proxy.port.to_string(),
            false,
        );
        push(
            "campos removidos",
            old.proxy.strip_fields.join(", "),
            new.proxy.strip_fields.join(", "),
            false,
        );
        push(
            "auditoria",
            on_off(old.proxy.audit),
            on_off(new.proxy.audit),
            false,
        );
        push(
            "retenção da auditoria",
            format!("{} dias", old.proxy.audit_keep_days),
            format!("{} dias", new.proxy.audit_keep_days),
            false,
        );
        let shipped = "(as que vêm com o Mandrake)";
        push(
            "pasta de souls",
            or_dash(old.souls.dir.as_deref(), shipped),
            or_dash(new.souls.dir.as_deref(), shipped),
            false,
        );
        push(
            "skill invoke",
            on_off(old.skills.invoke),
            on_off(new.skills.invoke),
            false,
        );
        push(
            "comando de memória",
            or_dash(old.skills.memory_cmd.as_deref(), "(nenhum)"),
            or_dash(new.skills.memory_cmd.as_deref(), "(nenhum)"),
            false,
        );
        out
    }
}
