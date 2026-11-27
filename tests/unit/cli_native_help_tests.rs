use super::*;

#[test]
fn every_native_command_and_its_options_are_documented_and_have_specific_help() {
    let doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/grammar.md"));
    let common = ["file", "fields", "ttl", "max", "max-pages"];
    let mut count = 0;
    for service in SERVICES {
        for cmd in crate::native::commands(service.cli) {
            let prefix = format!("joom {} ACCOUNT {}", service.cli, cmd.get_name());
            let forms = doc
                .lines()
                .filter(|line| {
                    *line == prefix
                        || line
                            .strip_prefix(&prefix)
                            .is_some_and(|tail| tail.starts_with(' '))
                })
                .collect::<Vec<_>>();
            assert_eq!(forms.len(), 1, "{prefix}");
            assert_eq!(
                forms[0],
                documentation_form(service.cli, &cmd),
                "{prefix}: command grammar differs from the parser"
            );
            let help =
                parse_from(["joom", "help", service.cli, cmd.get_name(), "--verbose"]).unwrap_err();
            assert_eq!(help.kind(), clap::error::ErrorKind::DisplayHelp);
            let help = help.to_string();
            assert!(
                help.contains(&format!(
                    "joom [OPTIONS] {} <ACCOUNT> {}",
                    service.cli,
                    cmd.get_name()
                )),
                "{help}"
            );
            for arg in cmd.get_arguments() {
                if let Some(long) = arg.get_long() {
                    assert!(
                        help.contains(&format!("--{long}")),
                        "{prefix}: missing help for --{long}"
                    );
                    if !common.contains(&long) {
                        assert!(
                            forms[0].contains(&format!("--{long}")),
                            "{prefix}: undocumented --{long}"
                        );
                    }
                } else if let Some(labels) = arg.get_value_names() {
                    for label in labels {
                        assert!(
                            forms[0].contains(label.as_str()),
                            "{prefix}: undocumented {label}"
                        );
                    }
                }
            }
            for option in [
                "body",
                "upload",
                "output",
                "max",
                "max-pages",
                "ttl",
                "parent",
                "to",
                "cc",
                "bcc",
                "calendar",
                "list",
            ] {
                let advertised = help.contains(&format!("--{option} "));
                let accepted = cmd
                    .get_arguments()
                    .any(|arg| arg.get_long() == Some(option));
                assert_eq!(advertised, accepted, "{prefix}: --{option}");
            }
            count += 1;
        }
    }
    assert_eq!(count, 6);
}

fn documentation_form(service: &str, command: &Command) -> String {
    let mut form = format!("joom {service} ACCOUNT {}", command.get_name());
    let mut positionals = command
        .get_arguments()
        .filter(|arg| arg.get_long().is_none())
        .collect::<Vec<_>>();
    positionals.sort_by_key(|arg| arg.get_index());
    let label = |arg: &Arg| {
        arg.get_value_names()
            .map(|names| {
                names
                    .iter()
                    .map(|name| name.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_else(|| arg.get_id().as_str().to_ascii_uppercase())
    };
    for arg in positionals {
        form.push(' ');
        form.push_str(&label(arg));
    }
    for arg in command.get_arguments() {
        let Some(long) = arg.get_long() else {
            continue;
        };
        if ["file", "fields", "ttl", "max", "max-pages"].contains(&long) {
            continue;
        }
        let mut option = format!("--{long}");
        if arg.get_action().takes_values() {
            option.push(' ');
            option.push_str(&label(arg));
            if arg
                .get_num_args()
                .is_some_and(|range| range.max_values() > 1)
            {
                option.push_str("...");
            }
        }
        form.push(' ');
        if arg.is_required_set() {
            form.push_str(&option);
        } else {
            form.push_str(&format!("[{option}]"));
        }
    }
    form
}
