//! The setup wizard as a state machine: keys in, a step and a form out. Nothing here touches the
//! terminal, and nothing is applied: confirming yields a [`Profile`] for the caller.

use super::field::TextField;
use crossterm::event::{KeyCode, KeyEvent};
use mandrake_home::{Action, Dirs, Profile};
use mandrake_layout::{Catalog, Group, Kind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    DevRoot,
    Items,
    Plugins,
    Proxy,
    Souls,
    Summary,
}

pub const STEPS: [Step; 6] = [
    Step::DevRoot,
    Step::Items,
    Step::Plugins,
    Step::Proxy,
    Step::Souls,
    Step::Summary,
];

impl Step {
    pub fn index(self) -> usize {
        STEPS.iter().position(|&s| s == self).unwrap_or(0)
    }
    pub fn title(self) -> &'static str {
        match self {
            Step::DevRoot => "Pasta de desenvolvimento",
            Step::Items => "Itens da pasta do Claude Code",
            Step::Plugins => "Plugins",
            Step::Proxy => "Proxy",
            Step::Souls => "Souls e skill invoke",
            Step::Summary => "Resumo",
        }
    }
}

/// What the Claude Code folder has at an item's path right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presence {
    Missing,
    File,
    Dir,
    Link(String),
}

#[derive(Debug, Clone)]
pub struct ItemChoice {
    pub path: String,
    pub kind: Kind,
    pub group: Group,
    pub sensitive: bool,
    pub about: String,
    pub action: Action,
    pub target: TextField,
    pub present: Presence,
}

/// Rows of the plugins step; the first one is locked on.
pub const PLUGINS: [(&str, &str); 3] = [
    (
        "prompt-trim",
        "corta o que o Claude Code injeta no prompt; obrigatório",
    ),
    ("fancy", "status line e temas do Mandrake"),
    (
        "tui-embed",
        "painel dentro do Claude Code para o mandrake watch --embed",
    ),
];

/// Rows of the proxy step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxyRow {
    Port,
    Safeguards,
    Audit,
    KeepDays,
}
pub const PROXY_ROWS: [ProxyRow; 4] = [
    ProxyRow::Port,
    ProxyRow::Safeguards,
    ProxyRow::Audit,
    ProxyRow::KeepDays,
];

/// Rows of the souls step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoulsRow {
    Dir,
    Invoke,
    MemoryCmd,
}
pub const SOULS_ROWS: [SoulsRow; 3] = [SoulsRow::Dir, SoulsRow::Invoke, SoulsRow::MemoryCmd];

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Continue,
    Quit,
    Done(Box<Profile>),
}

pub const SENSITIVE: &str = "⚠ guarda conversas, prompts ou conteúdo colado: linkar ou copiar leva esse histórico para o destino";

pub struct Wizard {
    pub dirs: Dirs,
    pub current: Profile,
    pub step: Step,
    pub dev_root: TextField,
    pub items: Vec<ItemChoice>,
    pub item: usize,
    /// While a target is being edited: its value before the edit, restored by `Esc`.
    pub editing: Option<String>,
    pub plugin: usize,
    pub fancy: bool,
    pub tui_embed: bool,
    pub proxy_row: usize,
    pub port: TextField,
    pub strip_safeguards: bool,
    pub audit: bool,
    pub keep_days: TextField,
    pub souls_row: usize,
    pub souls_dir: TextField,
    pub invoke: bool,
    pub memory_cmd: TextField,
    pub summary_scroll: usize,
    /// Shown under the form until the next key.
    pub notice: Option<String>,
}

fn group_order(g: Group) -> u8 {
    match g {
        Group::Config => 0,
        Group::Plugins => 1,
        Group::Session => 2,
    }
}

fn presence(dirs: &Dirs, path: &str) -> Presence {
    let p = dirs.claude.join(path);
    match std::fs::symlink_metadata(&p) {
        Err(_) => Presence::Missing,
        Ok(m) if m.file_type().is_symlink() => Presence::Link(
            std::fs::read_link(&p)
                .map(|t| t.display().to_string())
                .unwrap_or_default(),
        ),
        Ok(m) if m.is_dir() => Presence::Dir,
        Ok(_) => Presence::File,
    }
}

impl Wizard {
    pub fn new(dirs: &Dirs, catalog: &Catalog, current: &Profile) -> Self {
        let mut items: Vec<ItemChoice> = catalog
            .items
            .iter()
            .map(|i| {
                let spec = current.links.get(&i.path);
                let target = spec
                    .and_then(|s| s.target.clone())
                    .unwrap_or_else(|| format!("claude/{}", i.path));
                ItemChoice {
                    path: i.path.clone(),
                    kind: i.kind,
                    group: i.group,
                    sensitive: i.sensitive,
                    about: i.about.clone(),
                    action: spec.map(|s| s.action).unwrap_or(Action::Keep),
                    target: TextField::new(target),
                    present: presence(dirs, &i.path),
                }
            })
            .collect();
        items.sort_by_key(|i| group_order(i.group));
        let p = &current.proxy;
        Self {
            dirs: dirs.clone(),
            current: current.clone(),
            step: Step::DevRoot,
            dev_root: TextField::new(current.dev_root.clone().unwrap_or_default()),
            items,
            item: 0,
            editing: None,
            plugin: 0,
            fancy: current.plugins.fancy,
            tui_embed: current.plugins.tui_embed,
            proxy_row: 0,
            port: TextField::digits(p.port.to_string()),
            strip_safeguards: p.strip_fields.iter().any(|f| f == "safeguards"),
            audit: p.audit,
            keep_days: TextField::digits(p.audit_keep_days.to_string()),
            souls_row: 0,
            souls_dir: TextField::new(current.souls.dir.clone().unwrap_or_default()),
            invoke: current.skills.invoke,
            memory_cmd: TextField::new(current.skills.memory_cmd.clone().unwrap_or_default()),
            summary_scroll: 0,
            notice: None,
        }
    }

    pub fn selected(&self) -> Option<&ItemChoice> {
        self.items.get(self.item)
    }

    /// The warning for the item under the cursor, when it holds conversations.
    pub fn warning(&self) -> Option<&'static str> {
        self.selected().filter(|i| i.sensitive).map(|_| SENSITIVE)
    }

    /// The focused text field of the current step, if the cursor is on one.
    fn focused_field(&mut self) -> Option<&mut TextField> {
        match self.step {
            Step::DevRoot => Some(&mut self.dev_root),
            Step::Items if self.editing.is_some() => {
                self.items.get_mut(self.item).map(|i| &mut i.target)
            }
            Step::Proxy => match PROXY_ROWS[self.proxy_row] {
                ProxyRow::Port => Some(&mut self.port),
                ProxyRow::KeepDays => Some(&mut self.keep_days),
                _ => None,
            },
            Step::Souls => match SOULS_ROWS[self.souls_row] {
                SoulsRow::Dir => Some(&mut self.souls_dir),
                SoulsRow::MemoryCmd => Some(&mut self.memory_cmd),
                SoulsRow::Invoke => None,
            },
            _ => None,
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) -> Outcome {
        self.notice = None;
        if self.editing.is_some() {
            return self.on_edit_key(k);
        }
        match k.code {
            KeyCode::Esc => return Outcome::Quit,
            KeyCode::Tab => return self.forward(),
            KeyCode::Enter if self.step != Step::Summary => return self.forward(),
            KeyCode::BackTab => {
                self.back();
                return Outcome::Continue;
            }
            _ => {}
        }
        // A text field takes letters, so `q` only quits where the cursor is not on one.
        if let Some(f) = self.focused_field()
            && f.on_key(&k)
        {
            return Outcome::Continue;
        }
        if k.code == KeyCode::Char('q') {
            return Outcome::Quit;
        }
        match self.step {
            Step::DevRoot => {}
            Step::Items => self.items_key(k.code),
            Step::Plugins => self.plugins_key(k.code),
            Step::Proxy => self.proxy_key(k.code),
            Step::Souls => self.souls_key(k.code),
            Step::Summary => return self.summary_key(k.code),
        }
        Outcome::Continue
    }

    fn on_edit_key(&mut self, k: KeyEvent) -> Outcome {
        match k.code {
            KeyCode::Enter | KeyCode::Tab => self.editing = None,
            KeyCode::Esc => {
                if let (Some(before), Some(item)) =
                    (self.editing.take(), self.items.get_mut(self.item))
                {
                    item.target.set(before);
                }
            }
            _ => {
                if let Some(f) = self.focused_field() {
                    f.on_key(&k);
                }
            }
        }
        Outcome::Continue
    }

    fn forward(&mut self) -> Outcome {
        if let Err(e) = self.check(self.step) {
            self.notice = Some(e);
            return Outcome::Continue;
        }
        if self.step == Step::Summary {
            return self.confirm();
        }
        self.step = STEPS[self.step.index() + 1];
        self.summary_scroll = 0;
        Outcome::Continue
    }

    fn back(&mut self) {
        if let Some(i) = self.step.index().checked_sub(1) {
            self.step = STEPS[i];
        }
    }

    /// Confirms the summary: the profile, or why it cannot be built yet.
    pub fn confirm(&mut self) -> Outcome {
        match self.build() {
            Ok(p) => Outcome::Done(Box::new(p)),
            Err(e) => {
                self.notice = Some(e);
                Outcome::Continue
            }
        }
    }

    /// Why a step cannot be left yet.
    pub fn check(&self, step: Step) -> Result<(), String> {
        match step {
            Step::Items => {
                let relative = |t: &str| !t.starts_with('/') && !t.starts_with('~');
                for i in self.items.iter().filter(|i| i.action != Action::Keep) {
                    match i.target.some() {
                        None => return Err(format!("{}: falta o destino", i.path)),
                        Some(t) if relative(&t) && self.dev_root.some().is_none() => {
                            return Err(format!(
                                "{}: destino relativo sem pasta de desenvolvimento (volte com Shift+Tab)",
                                i.path
                            ));
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            Step::Proxy => {
                match self.port.value().parse::<u16>() {
                    Ok(p) if p > 0 => {}
                    _ => return Err("porta inválida: use um número de 1 a 65535".into()),
                }
                match self.keep_days.value().parse::<u32>() {
                    Ok(d) if d > 0 => Ok(()),
                    _ if !self.audit => Ok(()),
                    _ => Err("retenção inválida: use pelo menos 1 dia".into()),
                }
            }
            Step::Summary => STEPS[..5].iter().try_for_each(|&s| self.check(s)),
            _ => Ok(()),
        }
    }

    fn items_key(&mut self, code: KeyCode) {
        let last = self.items.len().saturating_sub(1);
        let set = |w: &mut Self, a: Action| {
            if let Some(i) = w.items.get_mut(w.item) {
                i.action = a;
            }
        };
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.item = self.item.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => self.item = (self.item + 1).min(last),
            KeyCode::PageUp => self.item = self.item.saturating_sub(10),
            KeyCode::PageDown => self.item = (self.item + 10).min(last),
            KeyCode::Home => self.item = 0,
            KeyCode::End => self.item = last,
            KeyCode::Right | KeyCode::Char(' ') => self.cycle(1),
            KeyCode::Left => self.cycle(-1),
            KeyCode::Char('m') => set(self, Action::Keep),
            KeyCode::Char('l') => set(self, Action::Link),
            KeyCode::Char('c') => set(self, Action::Copy),
            KeyCode::Char('e') => {
                self.editing = self.selected().map(|i| i.target.value().to_string())
            }
            _ => {}
        }
    }

    fn cycle(&mut self, delta: i32) {
        const ORDER: [Action; 3] = [Action::Keep, Action::Link, Action::Copy];
        if let Some(i) = self.items.get_mut(self.item) {
            let at = ORDER.iter().position(|&a| a == i.action).unwrap_or(0) as i32;
            i.action = ORDER[(at + delta).rem_euclid(3) as usize];
        }
    }

    fn plugins_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up | KeyCode::Char('k') => self.plugin = self.plugin.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                self.plugin = (self.plugin + 1).min(PLUGINS.len() - 1)
            }
            KeyCode::Char(' ') => match self.plugin {
                0 => {
                    self.notice = Some("prompt-trim é obrigatório: é ele que corta o prompt".into())
                }
                1 => self.fancy = !self.fancy,
                _ => self.tui_embed = !self.tui_embed,
            },
            _ => {}
        }
    }

    fn proxy_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.proxy_row = self.proxy_row.saturating_sub(1),
            KeyCode::Down => self.proxy_row = (self.proxy_row + 1).min(PROXY_ROWS.len() - 1),
            KeyCode::Char(' ') => match PROXY_ROWS[self.proxy_row] {
                ProxyRow::Safeguards => self.strip_safeguards = !self.strip_safeguards,
                ProxyRow::Audit => self.audit = !self.audit,
                _ => {}
            },
            _ => {}
        }
    }

    fn souls_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Up => self.souls_row = self.souls_row.saturating_sub(1),
            KeyCode::Down => self.souls_row = (self.souls_row + 1).min(SOULS_ROWS.len() - 1),
            KeyCode::Char(' ') if SOULS_ROWS[self.souls_row] == SoulsRow::Invoke => {
                self.invoke = !self.invoke
            }
            _ => {}
        }
    }

    fn summary_key(&mut self, code: KeyCode) -> Outcome {
        match code {
            KeyCode::Enter => return self.forward(),
            KeyCode::Up | KeyCode::Char('k') => {
                self.summary_scroll = self.summary_scroll.saturating_sub(1)
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.summary_scroll =
                    (self.summary_scroll + 1).min(self.changes().len().saturating_sub(1))
            }
            _ => {}
        }
        Outcome::Continue
    }
}
