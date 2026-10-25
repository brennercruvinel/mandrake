//! `mandrake setup --cli`: the setup as plain questions, one per line, for any terminal and for
//! scripts. Enter takes the value in brackets. Prints every answer back, so a log shows each item.
//! Input that ends before the last answer stops the setup: only an answer given applies anything.

use anyhow::{Result, bail};
use mandrake_home::{Action, Dirs, LinkSpec, Profile};
use mandrake_layout::{Catalog, links};
use std::io::{BufRead, IsTerminal, Write};

struct Ask<'a> {
    input: Box<dyn BufRead + 'a>,
}

impl Ask<'_> {
    fn line(&mut self, q: &str, default: &str) -> Result<String> {
        print!("{q} [{default}]: ");
        let _ = std::io::stdout().flush();
        let mut s = String::new();
        if self.input.read_line(&mut s)? == 0 {
            println!();
            bail!("a entrada terminou antes das respostas; nada foi aplicado");
        }
        let s = s.trim();
        let v = if s.is_empty() {
            default.to_string()
        } else {
            s.to_string()
        };
        // Piped answers are not echoed by a terminal: print them, so the log shows every item.
        if !std::io::stdin().is_terminal() {
            println!("{v}");
        }
        Ok(v)
    }

    fn yes(&mut self, q: &str, default: bool) -> Result<bool> {
        loop {
            let a = self
                .line(q, if default { "s" } else { "n" })?
                .to_lowercase();
            match a.as_str() {
                "s" | "sim" | "y" | "yes" => return Ok(true),
                "n" | "nao" | "não" | "no" => return Ok(false),
                _ => println!("  responda s ou n"),
            }
        }
    }
}

fn found_text(f: &links::Found) -> String {
    match f {
        links::Found::Missing => "não existe".into(),
        links::Found::Link(t) => format!("link para {}", t.display()),
        links::Found::File => "arquivo".into(),
        links::Found::Dir => "pasta".into(),
    }
}

/// Asks everything, starting from `start`; returns the profile to apply, or None if not confirmed.
pub fn run(
    dirs: &Dirs,
    catalog: &Catalog,
    start: &Profile,
    input: impl BufRead,
) -> Result<Option<Profile>> {
    let mut a = Ask {
        input: Box::new(input),
    };
    let mut p = start.clone();
    println!("Mandrake setup. Enter aceita o valor entre colchetes.\n");
    println!("1. Pasta de desenvolvimento: a raiz dos destinos relativos.");
    let dev = a.line("   pasta", p.dev_root.as_deref().unwrap_or("~/dev"))?;
    p.dev_root = Some(dev);

    let mut keys: Vec<(String, String, bool)> = catalog
        .items
        .iter()
        .map(|i| (i.path.clone(), i.about.clone(), i.sensitive))
        .collect();
    keys.extend(
        start
            .links
            .keys()
            .filter(|k| catalog.get(k).is_none())
            .map(|k| (k.clone(), "fora do catálogo".to_string(), false)),
    );
    println!(
        "\n2. Itens de {} ({} itens): manter (m), linkar (l) ou copiar (c).",
        dirs.claude.display(),
        keys.len()
    );
    for (n, (key, about, sensitive)) in keys.iter().enumerate() {
        let found = links::inspect(&dirs.claude.join(key));
        let cur = p.links.get(key).cloned().unwrap_or(LinkSpec {
            action: Action::Keep,
            target: None,
        });
        println!("\n   [{}/{}] {key}: {about}", n + 1, keys.len());
        println!("         hoje: {}", found_text(&found));
        if *sensitive {
            println!("         atenção: guarda conversas ou conteúdo colado");
        }
        let def = match cur.action {
            Action::Keep => "m",
            Action::Link => "l",
            Action::Copy => "c",
        };
        let action = loop {
            match a.line("         ação", def)?.to_lowercase().as_str() {
                "m" | "manter" => break Action::Keep,
                "l" | "linkar" => break Action::Link,
                "c" | "copiar" => break Action::Copy,
                _ => println!("         responda m, l ou c"),
            }
        };
        if action == Action::Keep {
            p.links.remove(key);
            continue;
        }
        let target = a.line(
            "         destino",
            cur.target.as_deref().unwrap_or(&format!("claude/{key}")),
        )?;
        p.links.insert(
            key.clone(),
            LinkSpec {
                action,
                target: Some(target),
            },
        );
    }

    println!("\n3. Plugins. prompt-trim é obrigatório e vai sempre.");
    p.plugins.fancy = a.yes(
        "   fancy (markdown com tema, badges, barra de status)",
        p.plugins.fancy,
    )?;
    p.plugins.tui_embed = a.yes(
        "   tui-embed (apps de terminal em painéis)",
        p.plugins.tui_embed,
    )?;

    println!("\n4. Proxy local.");
    p.proxy.port = loop {
        match a.line("   porta", &p.proxy.port.to_string())?.parse() {
            Ok(v) => break v,
            Err(_) => println!("   um número de porta"),
        }
    };
    let had = p.proxy.strip_fields.iter().any(|f| f == "safeguards");
    println!(
        "   safeguards alimenta o classificador de segurança da API; tirá-lo é escolha consciente."
    );
    let strip = a.yes("   remover safeguards", had)?;
    p.proxy.strip_fields.retain(|f| f != "safeguards");
    if strip {
        p.proxy.strip_fields.push("safeguards".into());
    }
    p.proxy.audit = a.yes(
        "   auditoria (guarda os corpos antes e depois, local, 600)",
        p.proxy.audit,
    )?;

    println!("\n5. Prompts.");
    let core = a.line(
        "   core (padrao, none ou um caminho)",
        p.souls.core.as_deref().unwrap_or("padrao"),
    )?;
    p.souls.core = if core == "padrao" { None } else { Some(core) };
    let souls = a.line(
        "   pasta de souls (padrao = os do Mandrake)",
        p.souls.dir.as_deref().unwrap_or("padrao"),
    )?;
    p.souls.dir = if souls == "padrao" { None } else { Some(souls) };
    p.skills.invoke = a.yes(
        "   skill invoke (abre a sessão com a memória)",
        p.skills.invoke,
    )?;
    if p.skills.invoke {
        let m = a.line(
            "   comando de memória (vazio = nenhum)",
            p.skills.memory_cmd.as_deref().unwrap_or("nenhum"),
        )?;
        p.skills.memory_cmd = if m == "nenhum" || m.is_empty() {
            None
        } else {
            Some(m)
        };
    }

    println!("\n6. Resumo");
    println!("   pasta {}", p.dev_root.as_deref().unwrap_or("-"));
    let steps = links::plan(dirs, catalog, &p)?;
    let changing: Vec<_> = steps.iter().filter(|s| s.op.changes()).collect();
    println!(
        "   {} no perfil; {}",
        match p.links.len() {
            1 => "1 link".to_string(),
            n => format!("{n} links"),
        },
        match changing.len() {
            0 => "nada muda agora".to_string(),
            1 => "1 muda agora:".to_string(),
            n => format!("{n} mudam agora:"),
        }
    );
    for s in changing {
        println!("     {}", s.say(dirs));
    }
    println!(
        "   plugins: prompt-trim{}{}",
        if p.plugins.fancy { ", fancy" } else { "" },
        if p.plugins.tui_embed {
            ", tui-embed"
        } else {
            ""
        }
    );
    println!(
        "   proxy 127.0.0.1:{} removendo {}, auditoria {}",
        p.proxy.port,
        p.proxy.strip_fields.join(", "),
        if p.proxy.audit { "ligada" } else { "desligada" }
    );
    println!(
        "   core {}, souls {}",
        p.souls.core.as_deref().unwrap_or("padrão"),
        p.souls.dir.as_deref().unwrap_or("do Mandrake")
    );
    if a.yes("\n   aplicar", true)? {
        Ok(Some(p))
    } else {
        Ok(None)
    }
}
