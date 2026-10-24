//! Draws the wizard, one step at a time.

use super::profile::action_label;
use super::state::{
    ItemChoice, PLUGINS, PROXY_ROWS, Presence, ProxyRow, SOULS_ROWS, STEPS, SoulsRow, Step, Wizard,
};
use crate::theme;
use mandrake_home::Action;
use mandrake_layout::{Group, Kind};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Tabs, Wrap};

pub fn render(f: &mut Frame, w: &Wizard) {
    let [title, tabs, body, notice, help] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(6),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(f.area());
    let n = w.step.index() + 1;
    f.render_widget(
        theme::title(
            "setup",
            &format!("passo {n} de {} · {}", STEPS.len(), w.step.title()),
        ),
        title,
    );
    let names = [
        "1 Pasta",
        "2 Itens",
        "3 Plugins",
        "4 Proxy",
        "5 Souls",
        "6 Resumo",
    ];
    f.render_widget(
        Tabs::new(names)
            .select(w.step.index())
            .style(theme::dim())
            .highlight_style(theme::selected())
            .divider(Span::styled("›", theme::dim())),
        tabs,
    );
    let block = Block::bordered()
        .border_style(theme::frame())
        .title(Span::styled(
            format!(" {} ", w.step.title()),
            theme::accent(),
        ));
    let inner = block.inner(body);
    f.render_widget(block, body);
    match w.step {
        Step::DevRoot => dev_root(f, inner, w),
        Step::Items => items(f, inner, w),
        Step::Plugins => plugins(f, inner, w),
        Step::Proxy => proxy(f, inner, w),
        Step::Souls => souls(f, inner, w),
        Step::Summary => summary(f, inner, w),
    }
    if let Some(msg) = &w.notice {
        f.render_widget(
            Paragraph::new(Span::styled(format!(" {msg}"), theme::warn())),
            notice,
        );
    }
    f.render_widget(Paragraph::new(theme::help(&help_for(w))), help);
}

fn help_for(w: &Wizard) -> Vec<(&'static str, &'static str)> {
    let nav = [
        ("Enter", "próximo"),
        ("Shift+Tab", "voltar"),
        ("Esc", "sair"),
    ];
    let mut keys = match w.step {
        Step::DevRoot => vec![],
        Step::Items if w.editing.is_some() => {
            return vec![("Enter", "conclui o destino"), ("Esc", "desfaz")];
        }
        Step::Items => vec![
            ("↑↓", "item"),
            ("←→ espaço", "ação"),
            ("m l c", "manter/linkar/copiar"),
            ("e", "editar destino"),
        ],
        Step::Plugins => vec![("↑↓", "plugin"), ("espaço", "marca")],
        Step::Proxy | Step::Souls => vec![("↑↓", "campo"), ("espaço", "alterna")],
        Step::Summary => {
            return vec![
                ("Enter", "confirma"),
                ("↑↓", "rolar"),
                ("Shift+Tab", "voltar"),
                ("Esc q", "sair sem salvar"),
            ];
        }
    };
    keys.extend(nav);
    keys
}

fn label(text: &str) -> Span<'static> {
    Span::styled(format!("{text:<20}"), theme::label())
}

/// `› ` on the focused row, so focus never depends on color alone.
fn pointer(focused: bool) -> Span<'static> {
    if focused {
        Span::styled("› ", theme::accent())
    } else {
        Span::raw("  ")
    }
}

fn check(on: bool) -> Span<'static> {
    if on {
        Span::styled(
            "[x] ",
            Style::new().fg(theme::TRIM).add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("[ ] ", theme::dim())
    }
}

fn dev_root(f: &mut Frame, area: Rect, w: &Wizard) {
    let resolved = w
        .dev_root
        .some()
        .map(|r| format!("→ {}", w.dirs.expand(&r, None).display()))
        .unwrap_or_else(|| "sem pasta: só destinos absolutos ou com ~/ funcionam".into());
    let lines = vec![
        Line::from(
            "Onde ficam seus projetos. Destinos relativos dos itens são resolvidos a partir dela,",
        ),
        Line::from("então o mesmo perfil serve num Mac e num Linux com pastas diferentes."),
        Line::from(""),
        Line::from(
            [
                vec![pointer(true), label("pasta")],
                w.dev_root.spans(true, "ex.: ~/dev"),
            ]
            .concat(),
        ),
        Line::from(vec![
            Span::raw("  "),
            label(""),
            Span::styled(resolved, theme::dim()),
        ]),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn group_label(g: Group) -> &'static str {
    match g {
        Group::Config => "Configuração",
        Group::Plugins => "Plugins",
        Group::Session => "Sessão e histórico",
    }
}

fn action_style(a: Action) -> Style {
    match a {
        Action::Keep => theme::dim(),
        Action::Link => Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        Action::Copy => Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
    }
}

fn items(f: &mut Frame, area: Rect, w: &Wizard) {
    let [list_area, detail_area] = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area)
    } else {
        Layout::vertical([Constraint::Min(5), Constraint::Length(9)]).areas(area)
    };
    let mut rows = vec![];
    let mut selected_row = 0;
    let mut last_group = None;
    for (i, it) in w.items.iter().enumerate() {
        if last_group != Some(it.group) {
            rows.push(ListItem::new(Line::from(Span::styled(
                group_label(it.group),
                theme::accent(),
            ))));
            last_group = Some(it.group);
        }
        if i == w.item {
            selected_row = rows.len();
        }
        let mut spans = vec![
            Span::styled(
                format!(" {:<8}", action_label(it.action)),
                action_style(it.action),
            ),
            Span::raw(format!("{:<18}", it.path)),
        ];
        if it.action != Action::Keep {
            spans.push(Span::styled(
                format!("→ {}", it.target.value()),
                theme::dim(),
            ));
        }
        if it.sensitive {
            spans.push(Span::styled(" ⚠", theme::warn()));
        }
        rows.push(ListItem::new(Line::from(spans)));
    }
    let list = List::new(rows).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
    let mut state = ListState::default().with_selected(Some(selected_row));
    f.render_stateful_widget(list, list_area, &mut state);
    if let Some(it) = w.selected() {
        detail(f, detail_area, w, it);
    }
}

fn detail(f: &mut Frame, area: Rect, w: &Wizard, it: &ItemChoice) {
    let kind = match it.kind {
        Kind::File => "arquivo",
        Kind::Dir => "pasta",
    };
    let now = match &it.present {
        Presence::Missing => "não existe".to_string(),
        Presence::File => "arquivo".to_string(),
        Presence::Dir => "pasta".to_string(),
        Presence::Link(t) => format!("link → {t}"),
    };
    let editing = w.editing.is_some();
    let resolved = match (it.action, w.resolved(it)) {
        (Action::Keep, _) => "nada muda nesse item".to_string(),
        (_, Some(r)) => r,
        (_, None) => "falta a pasta de desenvolvimento para resolver".into(),
    };
    let mut lines = vec![
        Line::from(Span::styled(
            format!("{} ({kind})", it.path),
            Style::new().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(it.about.clone(), theme::dim())),
        Line::from(""),
        Line::from(vec![label("agora"), Span::raw(now)]),
        Line::from(vec![
            label("ação"),
            Span::styled(action_label(it.action), action_style(it.action)),
        ]),
        Line::from([vec![label("destino")], it.target.spans(editing, "")].concat()),
        Line::from(vec![
            label("resolve para"),
            Span::styled(resolved, theme::dim()),
        ]),
    ];
    if let Some(warn) = w.warning() {
        lines.push(Line::from(""));
        let style = if it.action == Action::Keep {
            theme::warn()
        } else {
            theme::error()
        };
        lines.push(Line::from(Span::styled(warn, style)));
    }
    let block = Block::bordered().border_style(theme::frame());
    let inner = block.inner(area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn plugins(f: &mut Frame, area: Rect, w: &Wizard) {
    let on = [true, w.fancy, w.tui_embed];
    let mut lines = vec![
        Line::from("O prompt-trim sempre instala; os outros são opcionais."),
        Line::from(""),
    ];
    for (i, (name, about)) in PLUGINS.iter().enumerate() {
        let mut spans = vec![
            pointer(w.plugin == i),
            check(on[i]),
            Span::raw(format!("{name:<14}")),
        ];
        if i == 0 {
            spans.push(Span::styled("travado · ", theme::warn()));
        }
        spans.push(Span::styled(*about, theme::dim()));
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn proxy(f: &mut Frame, area: Rect, w: &Wizard) {
    let at = PROXY_ROWS[w.proxy_row];
    let mut lines = vec![];
    for row in PROXY_ROWS {
        let focus = row == at;
        let line = match row {
            ProxyRow::Port => Line::from(
                [
                    vec![pointer(focus), label("porta")],
                    w.port.spans(focus, ""),
                ]
                .concat(),
            ),
            ProxyRow::Safeguards => Line::from(vec![
                pointer(focus),
                label("remover safeguards"),
                check(w.strip_safeguards),
                Span::styled("desligado por padrão", theme::dim()),
            ]),
            ProxyRow::Audit => Line::from(vec![
                pointer(focus),
                label("auditoria"),
                check(w.audit),
                Span::styled(
                    "guarda os corpos antes e depois do corte em state/audit, modo 600",
                    theme::dim(),
                ),
            ]),
            ProxyRow::KeepDays => {
                let mut l = [
                    vec![pointer(focus), label("retenção (dias)")],
                    w.keep_days.spans(focus, ""),
                ]
                .concat();
                if !w.audit {
                    l.push(Span::styled(
                        "  só vale com a auditoria ligada",
                        theme::dim(),
                    ));
                }
                Line::from(l)
            }
        };
        lines.push(line);
        if row == ProxyRow::Safeguards {
            lines.push(Line::from(vec![
                Span::raw("  "),
                label(""),
                Span::styled(
                    "o campo safeguards alimenta o classificador de segurança da API; o padrão é mantê-lo",
                    theme::warn(),
                ),
            ]));
        }
    }
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn souls(f: &mut Frame, area: Rect, w: &Wizard) {
    let at = SOULS_ROWS[w.souls_row];
    let mut lines = vec![
        Line::from(
            "Souls são as personalidades que o claude invoke <nome> abre; a skill invoke traz a memória.",
        ),
        Line::from(""),
    ];
    for row in SOULS_ROWS {
        let focus = row == at;
        lines.push(match row {
            SoulsRow::Dir => Line::from(
                [
                    vec![pointer(focus), label("pasta de souls")],
                    w.souls_dir
                        .spans(focus, "vazio: as souls que vêm com o Mandrake"),
                ]
                .concat(),
            ),
            SoulsRow::Invoke => Line::from(vec![
                pointer(focus),
                label("skill invoke"),
                check(w.invoke),
                Span::styled("abre a sessão com o que a memória registrou", theme::dim()),
            ]),
            SoulsRow::MemoryCmd => Line::from(
                [
                    vec![pointer(focus), label("comando de memória")],
                    w.memory_cmd.spans(
                        focus,
                        "ex.: mem.sh find; sem ele o invoke avisa e abre mesmo assim",
                    ),
                ]
                .concat(),
            ),
        });
    }
    f.render_widget(Paragraph::new(lines), area);
}

fn summary(f: &mut Frame, area: Rect, w: &Wizard) {
    if let Err(e) = w.build() {
        f.render_widget(
            Paragraph::new(Span::styled(
                format!("Ainda não dá para confirmar: {e}"),
                theme::error(),
            )),
            area,
        );
        return;
    }
    let changes = w.changes();
    let head = if changes.is_empty() {
        "Nenhuma mudança em relação ao perfil atual.".to_string()
    } else {
        format!(
            "{} mudança(s). Nada é aplicado aqui: Enter devolve o perfil para o setup aplicar.",
            changes.len()
        )
    };
    let mut lines = vec![
        Line::from(head),
        Line::from(vec![
            check(true),
            Span::styled("prompt-trim instala sempre", theme::dim()),
        ]),
        Line::from(""),
    ];
    for c in changes.iter().skip(w.summary_scroll) {
        let mut spans = vec![
            Span::styled(format!("{:<26}", c.what), theme::label()),
            Span::styled(c.before.clone(), theme::dim()),
            Span::raw(" → "),
            Span::styled(c.after.clone(), Style::new().add_modifier(Modifier::BOLD)),
        ];
        if c.sensitive {
            spans.push(Span::styled("  ⚠ guarda conversas", theme::warn()));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
}
