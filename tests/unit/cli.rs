use super::*;

fn representative(service: &str) -> &'static str {
    crate::schema_commands::contract(service)
        .document
        .root
        .methods
        .iter()
        .find(|(_, spec)| {
            spec.http_method == "GET" && !crate::schema_commands::direct_download(spec)
        })
        .map(|(name, _)| name.as_str())
        .unwrap()
}

#[test]
fn read_methods_default_to_all_results_and_only_offer_supported_limits() {
    for service in SERVICES {
        let document = &crate::schema_commands::contract(service.cli).document;
        for (method, spec) in document.root.methods.iter().filter(|(_, spec)| {
            spec.http_method == "GET"
                && !spec.fresh_response
                && !crate::schema_commands::direct_download(spec)
        }) {
            let base = [
                "joom",
                service.cli,
                "reader@example.com",
                method,
                "--file",
                "request.json",
            ];
            let parsed = parse_from(base).unwrap();
            let args = crate::native::arguments(parsed.subcommand().unwrap().1);
            assert!(args.try_get_one::<u32>("max").ok().flatten().is_none());
            assert!(
                args.try_get_one::<u32>("max-pages")
                    .ok()
                    .flatten()
                    .is_none()
            );
            assert_eq!(args.get_one::<u32>("ttl"), Some(&0));
            let json = parse_from(base.into_iter().chain(["--ttl", "60"])).unwrap();
            assert_eq!(
                crate::native::arguments(json.subcommand().unwrap().1).get_one::<u32>("ttl"),
                Some(&60)
            );
            for (flag, supported) in [
                (
                    "--max",
                    crate::pagination::Plan::new(spec, true, Some(2), None).is_ok(),
                ),
                (
                    "--max-pages",
                    crate::pagination::Plan::new(spec, true, None, Some(2)).is_ok(),
                ),
            ] {
                assert_eq!(
                    parse_from(base.into_iter().chain([flag, "2"])).is_ok(),
                    supported,
                    "{} {method} {flag}",
                    service.cli
                );
            }
            for flag in ["--all", "--no-cache"] {
                assert!(parse_from(base.into_iter().chain([flag])).is_err());
            }
        }
    }
}

#[test]
fn every_service_uses_file_paths_and_has_no_version_or_params_options() {
    for service in SERVICES {
        let method = representative(service.cli);
        let args = parse_from([
            "joom",
            service.cli,
            "reader@example.com",
            method,
            "--file",
            "request.json",
        ])
        .unwrap();
        assert_eq!(
            crate::native::arguments(args.subcommand().unwrap().1)
                .get_one::<String>("file")
                .unwrap(),
            "request.json"
        );
        for (option, value) in [("--params", "{}"), ("--api-version", "v2")] {
            assert!(
                parse_from([
                    "joom",
                    service.cli,
                    "reader@example.com",
                    method,
                    option,
                    value
                ])
                .is_err()
            );
        }
        assert!(
            parse_from([
                "joom",
                service.cli,
                "reader@example.com",
                "--file",
                "request.json",
                method
            ])
            .is_err()
        );
        let help = parse_from(["joom", "help", service.cli, method])
            .unwrap_err()
            .to_string();
        assert!(help.contains("--file <PATH>"));
        for removed in ["--api-version", "--params", "@file", "@-"] {
            assert!(!help.contains(removed));
        }
    }
}

#[test]
fn every_command_defaults_to_plain_and_propagates_explicit_json() {
    let mut commands = SERVICES
        .iter()
        .map(|service| {
            vec![
                service.cli,
                "reader@example.com",
                representative(service.cli),
                "--file",
                "request.json",
            ]
        })
        .collect::<Vec<_>>();
    commands.extend([
        vec!["auth", "init"],
        vec!["auth", "enroll"],
        vec!["auth", "test"],
        vec!["auth", "list"],
        vec!["auth", "reader@example.com", "login", "mail,drive"],
        vec!["auth", "reader@example.com", "logout"],
    ]);
    for command in commands {
        let defaults = parse_from(std::iter::once("joom").chain(command.iter().copied())).unwrap();
        assert!(!defaults.get_flag("json"));
        let action = if command[0] == "auth" && command.len() == 2 {
            1
        } else {
            2
        };
        for index in 0..=action {
            let mut args = command.clone();
            args.insert(index, "--json");
            let parsed = parse_from(std::iter::once("joom").chain(args.iter().copied())).unwrap();
            let mut current = &parsed;
            loop {
                assert!(current.get_flag("json"));
                if let Some((_, child)) = current.subcommand() {
                    current = child;
                } else {
                    break;
                }
            }
        }
    }
}

#[test]
fn command_definition_and_service_guide_are_valid() {
    command().debug_assert();
    let grammar = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/grammar.md"));
    for service in SERVICES {
        assert!(grammar.contains(&format!("| `{}` | {} |", service.cli, service.title)));
    }
}

#[test]
fn auth_has_only_the_requested_commands_and_explicit_accounts() {
    let cli = command();
    let auth = cli.find_subcommand("auth").unwrap();
    assert_eq!(
        auth.get_subcommands()
            .map(Command::get_name)
            .collect::<Vec<_>>(),
        [
            "init", "enroll", "test", "list", "login", "all", "export", "import", "logout", "help"
        ]
    );
    for action in ["login", "all", "logout"] {
        assert!(parse_from(["joom", "auth", action]).is_err());
        let mut args = vec!["joom", "auth", "alice@example.com", action];
        if action == "login" {
            args.push("mail");
        }
        assert!(parse_from(args).is_ok());
        assert!(
            parse_from(["joom", "auth", "alice@example.com", action, "--help"])
                .unwrap_err()
                .to_string()
                .contains(&format!("auth <ACCOUNT> {action}"))
        );
    }
    for action in ["init", "enroll", "test", "list"] {
        assert!(parse_from(["joom", "auth", action]).is_ok());
        assert!(parse_from(["joom", "auth", "alice@example.com", action]).is_err());
    }
    for action in ["add", "remove", "status", "use"] {
        assert!(parse_from(["joom", "auth", "alice@example.com", action]).is_err());
    }
    assert_eq!(
        parse_from(["joom", "auth", "help"]).unwrap_err().kind(),
        clap::error::ErrorKind::DisplayHelp
    );
}

#[test]
fn credential_transfers_accept_optional_accounts_and_require_files() {
    for action in ["export", "import"] {
        assert!(parse_from(["joom", "auth", action]).is_err());
        assert!(parse_from(["joom", "auth", "me@example.com", action]).is_err());
        assert!(parse_from(["joom", "auth", action, "credentials.joom"]).is_ok());
        assert!(parse_from(["joom", "auth", "me@example.com", action, "credentials.joom"]).is_ok());
        assert!(
            parse_from([
                "joom",
                "auth",
                action,
                "credentials.joom",
                "--passphrase-file",
                "private.txt"
            ])
            .is_ok()
        );
        assert!(
            parse_from([
                "joom",
                "auth",
                action,
                "credentials.joom",
                "--passphrase",
                "secret"
            ])
            .is_err()
        );
        let common = parse_from(["joom", "auth", action, "--help"])
            .unwrap_err()
            .to_string();
        assert!(!common.contains("--passphrase-file"));
        let full = parse_from(["joom", "auth", action, "--help", "--verbose"])
            .unwrap_err()
            .to_string();
        assert!(full.contains("--passphrase-file"));
        assert!(
            full.split("Optional parameters (advanced):")
                .nth(1)
                .unwrap()
                .contains("--passphrase-file")
        );
    }
    assert!(parse_from(["joom", "auth", "init", "--headless"]).is_ok());
    assert!(
        parse_from([
            "joom",
            "auth",
            "import",
            "credentials.joom",
            "--headless",
            "--replace"
        ])
        .is_ok()
    );
    assert!(parse_from(["joom", "auth", "export", "credentials.joom", "--headless"]).is_err());
}

#[test]
fn options_precede_actions_and_accounts_are_context() {
    let parsed = parse_from([
        "joom",
        "mail",
        "alice@example.com",
        "--json",
        "messages.list",
        "--q",
        "is:unread",
    ])
    .unwrap();
    assert!(parsed.get_flag("json"));
    let mail = parsed.subcommand_matches("mail").unwrap();
    assert_eq!(
        mail.get_one::<String>("account").unwrap(),
        "alice@example.com"
    );
    assert_eq!(mail.subcommand_name(), Some("messages.list"));
    for args in [
        vec![
            "joom",
            "mail",
            "alice@example.com",
            "send",
            "a@example.com",
            "--json",
        ],
        vec!["joom", "drive", "alice@example.com", "--max", "5", "list"],
        vec!["joom", "--account", "alice@example.com", "drive", "list"],
    ] {
        assert!(parse_from(args.clone()).is_err(), "{args:?}");
    }
    assert!(parse_from(["joom", "drive", "-", "files.list"]).is_ok());
}

#[test]
fn login_requires_a_nonempty_supported_positional_service_list() {
    assert!(parse_from(["joom", "auth", "me@example.com", "login"]).is_err());
    for services in ["", ",", "mail,", "mail,,drive", "admin"] {
        assert!(
            parse_from(["joom", "auth", "me@example.com", "login", services]).is_err(),
            "{services}"
        );
    }
    for services in [
        "mail",
        "mail,drive",
        "mail",
        "mail,drive",
        "all",
        "all,mail",
    ] {
        let parsed = parse_from(["joom", "auth", "me@example.com", "login", services]).unwrap();
        assert_eq!(
            parsed
                .subcommand_matches("auth")
                .unwrap()
                .subcommand_matches("login")
                .unwrap()
                .get_one::<String>("services")
                .unwrap(),
            services
        );
    }
    assert!(
        parse_from([
            "joom",
            "auth",
            "me@example.com",
            "login",
            "--services",
            "mail,drive"
        ])
        .is_err()
    );
}

#[test]
fn personal_endpoint_is_an_explicit_sign_in_option() {
    for command in [vec!["login", "all"], vec!["all"]] {
        let mut args = vec!["joom", "auth", "person@gmail.com"];
        args.extend(command.iter().copied());
        let parsed = parse_from(args.clone()).unwrap();
        let auth = parsed.subcommand_matches("auth").unwrap();
        assert!(!auth.subcommand().unwrap().1.get_flag("personal"));
        args.push("--personal");
        let parsed = parse_from(args).unwrap();
        let auth = parsed.subcommand_matches("auth").unwrap();
        assert!(auth.subcommand().unwrap().1.get_flag("personal"));
    }
}

#[test]
fn removed_options_services_and_oauth_service_names_are_rejected() {
    for option in [
        "--all",
        "--no-cache",
        "--api-version",
        "--params",
        "--plain",
        "--allow-write",
        "--anonymous",
        "--quota-project",
        "--readonly",
        "--dry-run",
        "--no-input",
    ] {
        assert!(
            parse_from([
                "joom",
                option,
                "mail",
                "alice@example.com",
                "send",
                "to@example.com"
            ])
            .is_err(),
            "{option}"
        );
    }
    assert_eq!(
        SERVICES.iter().map(|s| s.cli).collect::<Vec<_>>(),
        ["mail", "drive", "calendar", "sheets", "tasks", "contacts"]
    );
    for service in [
        "keep",
        "api",
        "chat",
        "meet",
        "classroom",
        "admin",
        "appscript",
        "groups",
        "analytics",
        "analyticsadmin",
        "searchconsole",
        "adsense",
        "storage",
        "pubsub",
        "cloud",
    ] {
        assert!(
            parse_from(["joom", service, "alice@example.com", "list"]).is_err(),
            "{service}"
        );
        assert!(crate::services::scopes(service).is_err(), "{service}");
    }
}

#[test]
fn authentication_guide_commands_parse() {
    let mut shell = false;
    let mut checked = 0;
    for line in include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/authentication.md"
    ))
    .lines()
    {
        if line == "```sh" {
            shell = true;
            continue;
        }
        if line == "```" {
            shell = false;
            continue;
        }
        if shell && line.starts_with("./bin/joom ") {
            parse_from(line.split_whitespace())
                .unwrap_or_else(|e| panic!("invalid documented command: {line}\n{e}"));
            checked += 1;
        }
    }
    assert!(checked > 0);
}

#[test]
fn documented_help_paths_do_not_require_accounts_or_credentials() {
    let mut paths = vec![
        vec!["joom", "--help"],
        vec!["joom", "-h"],
        vec!["joom", "help", "auth"],
    ];
    for action in [
        "init", "enroll", "test", "list", "login", "all", "export", "import", "logout",
    ] {
        paths.push(vec!["joom", "help", "auth", action]);
        paths.push(vec!["joom", "auth", "help", action]);
    }
    for action in ["login", "logout"] {
        paths.push(vec!["joom", "auth", "reader@example.com", action, "--help"]);
    }
    for service in SERVICES {
        paths.push(vec!["joom", "help", service.cli]);
        paths.push(vec!["joom", service.cli, "--help"]);
        paths.push(vec![
            "joom",
            service.cli,
            "reader@example.com",
            representative(service.cli),
            "--help",
        ]);
    }
    for path in paths {
        assert_eq!(
            parse_from(path.clone()).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayHelp,
            "{path:?}"
        );
    }
    for version in ["--version", "-V"] {
        assert_eq!(
            parse_from(["joom", version]).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayVersion
        );
    }
}
