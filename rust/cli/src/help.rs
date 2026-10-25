//! `--help` in Portuguese and in two groups: what someone new runs first, then the rest.

use clap::{Arg, ArgAction, Command};

/// The commands someone new needs, in the order they run them.
pub const START: [&str; 4] = ["setup", "doctor", "compare", "watch"];
/// The rest: measuring, auditing, links and the proxy by hand.
pub const ADVANCED: [&str; 6] = ["hex", "audit", "bench", "link", "proxy", "capture"];

const ROOT: &str = "\
{about-with-newline}
Uso: {usage}

Para começar:
{start}
Avançado:
{advanced}
Opções:
{options}

Ajuda de um comando: mandrake <comando> --help
";

const SUB: &str = "\
{about-with-newline}
Uso: {usage}
{subcommands_block}{positionals_block}
Opções:
{options}
";

/// The clap command with Portuguese labels, the `-h` and `-V` flags in Portuguese, and the top
/// level's commands split into the two groups.
pub fn localize(cmd: Command) -> Command {
    let line = |c: &Command| {
        format!(
            "  {:<9}{}\n",
            c.get_name(),
            c.get_about().map(|a| a.to_string()).unwrap_or_default()
        )
    };
    let group = |names: &[&str]| -> String {
        names
            .iter()
            .filter_map(|n| cmd.find_subcommand(n))
            .map(line)
            .collect()
    };
    let template = ROOT
        .replace("{start}", &group(&START))
        .replace("{advanced}", &group(&ADVANCED));
    let mut cmd = sub(cmd.disable_help_subcommand(true))
        .help_template(template)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help("Mostra a versão"),
        );
    cmd.build();
    usage_in_portuguese(&mut cmd);
    cmd
}

/// clap writes `[OPTIONS]` in every usage line; once the tree is built each line has its full
/// name (`mandrake proxy run`), and the placeholder can be swapped.
fn usage_in_portuguese(cmd: &mut Command) {
    let usage = cmd.render_usage().to_string();
    let usage = usage
        .trim_start_matches("Usage: ")
        .replace("[OPTIONS]", "[OPÇÕES]");
    *cmd = std::mem::take(cmd).override_usage(usage);
    for c in cmd.get_subcommands_mut() {
        usage_in_portuguese(c);
    }
}

/// clap's own errors are in English; the ones a person meets most often get a Portuguese line.
pub fn error_line(e: &clap::Error) -> Option<String> {
    use clap::error::{ContextKind, ContextValue, ErrorKind};
    let said = |k| match e.get(k) {
        Some(ContextValue::String(s)) => s.clone(),
        Some(ContextValue::Strings(v)) => v.join(", "),
        _ => String::new(),
    };
    Some(match e.kind() {
        ErrorKind::InvalidSubcommand => format!(
            "comando desconhecido: {}. Os comandos estão em mandrake --help",
            said(ContextKind::InvalidSubcommand)
        ),
        ErrorKind::UnknownArgument => format!(
            "opção desconhecida: {}. As opções estão no --help do comando",
            said(ContextKind::InvalidArg)
        ),
        ErrorKind::MissingRequiredArgument => {
            format!("falta um argumento: {}", said(ContextKind::InvalidArg))
        }
        ErrorKind::InvalidValue | ErrorKind::ValueValidation => format!(
            "valor inválido para {}: {}",
            said(ContextKind::InvalidArg),
            said(ContextKind::InvalidValue)
        ),
        ErrorKind::MissingSubcommand | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
            return None;
        }
        _ => return None,
    })
}

/// Every level below the top: the same labels, all the way down.
fn sub(cmd: Command) -> Command {
    let has_subs = cmd.get_subcommands().next().is_some();
    let template = SUB
        .replace(
            "{subcommands_block}",
            if has_subs {
                "\nComandos:\n{subcommands}\n"
            } else {
                ""
            },
        )
        .replace("{positionals_block}", "{positionals}");
    cmd.help_template(template)
        .subcommand_value_name("COMANDO")
        .subcommand_help_heading("Comandos")
        .disable_help_flag(true)
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .action(ArgAction::Help)
                .help("Mostra esta ajuda")
                .global(true),
        )
        .mut_subcommands(sub)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    fn help(args: &[&str]) -> String {
        let err = localize(crate::Cli::command())
            .try_get_matches_from(args)
            .unwrap_err();
        err.to_string()
    }

    #[test]
    fn every_command_is_in_exactly_one_group() {
        let cmd = crate::Cli::command();
        let mut names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        let mut grouped: Vec<&str> = START.iter().chain(&ADVANCED).copied().collect();
        names.sort();
        grouped.sort();
        assert_eq!(names, grouped);
    }

    #[test]
    fn the_help_reads_in_two_groups_in_portuguese() {
        let text = help(&["mandrake", "--help"]);
        let start = text.find("Para começar:").expect(&text);
        let advanced = text.find("Avançado:").expect(&text);
        assert!(start < text.find("  setup").unwrap() && text.find("  watch").unwrap() < advanced);
        assert!(advanced < text.find("  capture").unwrap());
        for english in [
            "Usage:",
            "Commands:",
            "Options:",
            "Print help",
            "Print version",
        ] {
            assert!(!text.contains(english), "{english} in\n{text}");
        }
        assert!(text.contains("Mostra a versão"), "{text}");
    }

    #[test]
    fn no_line_breaks_mid_word_at_100_columns() {
        for args in [
            &["mandrake", "--help"][..],
            &["mandrake", "proxy", "--help"],
        ] {
            let text = help(args);
            for l in text.lines() {
                assert!(l.chars().count() <= 100, "over 100 columns: {l:?}");
            }
        }
        let sub = help(&["mandrake", "link", "--help"]);
        assert!(sub.contains("Uso: mandrake link"), "{sub}");
        assert!(sub.contains("Comandos:"), "{sub}");
    }
}
