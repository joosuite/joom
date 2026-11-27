use crate::services::SERVICES;
use clap::{Arg, ArgAction, ArgMatches, Command, value_parser};
use std::ffi::OsString;

fn switch(name: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .help(help)
        .action(ArgAction::SetTrue)
}

#[cfg(test)]
pub fn command() -> Command {
    build(None, false, true)
}

fn build(selected: Option<&str>, verbose: bool, style_help: bool) -> Command {
    let auth = Command::new("auth")
        .about("Manage Microsoft sign-in and client credentials")
        .long_about("Microsoft client credentials and account tokens are encrypted locally. Use the OS keychain on a desktop, or a private key file with --headless on a server.")
        .after_help("Setup:\n  joom auth init\n  joom auth enroll\n  joom auth test\n  joom auth you@example.com login all\n\nUse all for every supported service, or a comma-separated list such as mail,drive.\nRegister a mobile and desktop application in Microsoft Entra admin center.\n\nGuide: docs/authentication.md")
        .override_usage("joom auth [OPTIONS] init|enroll|test|list\n       joom auth [OPTIONS] <ACCOUNT> login <SERVICE_LIST>\n       joom auth [OPTIONS] <ACCOUNT> all\n       joom auth [OPTIONS] [ACCOUNT] export <FILE>\n       joom auth [OPTIONS] [ACCOUNT] import <FILE> [--headless] [--replace]\n       joom auth [OPTIONS] <ACCOUNT> logout\n       joom auth help [COMMAND]")
        .subcommand_required(true)
        .subcommand_precedence_over_arg(true)
        .arg(Arg::new("account").value_name("ACCOUNT").help("Microsoft account email"))
        .subcommand(Command::new("init")
            .about("Create or reuse the credential vault")
            .long_about("Create a 256-bit master key in the OS keychain, or reuse the existing vault. Use --headless for a private local key file on a server without a desktop keychain. Existing encrypted credentials require their original key.")
            .arg(switch("headless", "Use a private local key file without an OS keychain"))
            .after_help("Guide: docs/authentication.md"))
        .subcommand(
            Command::new("enroll")
                .about("Save Microsoft client credentials")
                .long_about("Save your Microsoft Application (client) ID for native desktop sign-in with PKCE. Replaces previously enrolled client credentials.")
                .after_help("Microsoft Entra admin center > App registrations > New registration > Authentication > Mobile and desktop applications\n\n--stdin accepts a JSON object with a client_id field.\n\nGuide: docs/authentication.md")
                .arg(switch(
                    "stdin",
                    "Read client_id as JSON from stdin",
                )),
        )
        .subcommand(
            Command::new("test")
                .about("Test Microsoft OAuth connectivity with the enrolled client")
                .long_about("Check connectivity to Microsoft's OAuth endpoint. This does not verify the app registration, supported account types, or browser sign-in.")
                .after_help("Example:\n  joom auth test\n\nGuide: docs/authentication.md"),
        )
        .subcommand(
            Command::new("list")
                .override_usage("joom [OPTIONS] auth list")
                .about("List registered Microsoft accounts")
                .long_about("Show registered account emails in the selected vault, one per line. Use --json for a JSON array. Listing does not unlock the credential vault or check authorization with Microsoft.")
                .after_help("Examples:\n  joom auth list\n  joom --json auth list\n\nGuide: docs/authentication.md"),
        )
        .subcommand(
            Command::new("login")
                .override_usage("joom [OPTIONS] auth <ACCOUNT> login <SERVICE_LIST> [--personal]")
                .about("Sign in to a Microsoft account")
                .long_about("Sign in through your default browser and authorize the requested Microsoft services. Tokens are encrypted locally. The signed-in account must match the requested email. Sign-in expires after three minutes.")
                .after_help("Examples:\n  joom auth you@example.com login all\n  joom auth you@example.com login mail,drive\n\nGuide: docs/authentication.md")
                .arg(switch("personal", "Use Microsoft's personal-account endpoint, including accounts with external email addresses"))
                .arg(
                    Arg::new("services")
                        .value_name("SERVICE_LIST")
                        .required(true)
                        .value_parser(|value: &str| {
                            crate::services::scopes(value).map(|_| value.to_owned()).map_err(|e| e.to_string())
                        })
                        .help("Use all for every supported service, or a comma-separated list such as mail,drive"),
                ),
        )
        .subcommand(
            Command::new("all")
                .override_usage("joom [OPTIONS] auth <ACCOUNT> all [--personal]")
                .about("Sign in and authorize all supported services")
                .long_about("Open Microsoft sign-in and request every configured scope for all supported services. Equivalent to login all.")
                .after_help("Example:\n  joom auth you@example.com all\n\nGuide: docs/authentication.md")
                .arg(switch("personal", "Use Microsoft's personal-account endpoint, including accounts with external email addresses")),
        )
        .subcommand(
            Command::new("export")
                .override_usage("joom [OPTIONS] auth [ACCOUNT] export <FILE> [COMMAND_OPTIONS]")
                .about("Export sign-in credentials for another machine")
                .long_about("Save the OAuth client and refresh tokens in a passphrase-encrypted transfer file. Include ACCOUNT to export one account, or omit it to export all signed-in accounts. Prompts for a passphrase of at least 12 characters. Existing files are never overwritten. No Microsoft requests are made.")
                .arg(Arg::new("transfer-file").value_name("FILE").required(true).help("New encrypted transfer file to create"))
                .arg(Arg::new("passphrase-file").long("passphrase-file").value_name("PATH").help("Read the passphrase from a private UTF-8 file instead of prompting").hide(true))
                .after_help("Example:\n  joom auth you@example.com export credentials.joom\n  scp credentials.joom user@vps:~/\n\nGuide: docs/authentication.md"),
        )
        .subcommand(
            Command::new("import")
                .override_usage("joom [OPTIONS] auth [ACCOUNT] import <FILE> [COMMAND_OPTIONS]")
                .about("Import sign-in credentials without a browser")
                .long_about("Unlock an encrypted transfer file with its passphrase and save its OAuth client and refresh tokens in this machine's vault. Include ACCOUNT to import only that account. No browser or Microsoft request is needed; access tokens refresh automatically when you next use a service. Existing credentials are preserved unless --replace is supplied.")
                .arg(Arg::new("transfer-file").value_name("FILE").required(true).help("Encrypted transfer file to import"))
                .arg(switch("headless", "Create a vault with a private local key file; no desktop keychain required"))
                .arg(switch("replace", "Replace credentials for imported accounts; other accounts are preserved"))
                .arg(Arg::new("passphrase-file").long("passphrase-file").value_name("PATH").help("Read the passphrase from a private UTF-8 file instead of prompting").hide(true))
                .after_help("Example on the server:\n  joom auth import credentials.joom --headless\n\nThe headless vault stores its master key in master.key with owner-only permissions.\nUse --home with a new directory to keep it separate from an existing keychain vault.\n\nGuide: docs/authentication.md"),
        )
        .subcommand(
            Command::new("logout")
                .override_usage("joom [OPTIONS] auth <ACCOUNT> logout")
                .about("Delete the account's local tokens")
                .after_help("Microsoft permissions remain active until revoked in your Microsoft account settings.\n\nGuide: docs/authentication.md"),
        );

    let mut cmd = Command::new("joom").version(env!("CARGO_PKG_VERSION"))
        .about("Microsoft services from the terminal")
        .subcommand_required(true).arg_required_else_help(true)
        .after_help("Examples:\n  joom drive you@example.com files.list --max 10\n  joom mail you@example.com messages.list --q is:unread\n  joom sheets you@example.com values.get --spreadsheet-id SPREADSHEET_ID --range 'Sheet1!A1:D10'\n\nCommands: docs/grammar.md\nParameter files: docs/file-schemas.md\nSign-in setup: docs/authentication.md")
        .arg(Arg::new("home").long("home").help("Store state in PATH/joom").value_name("PATH").help_heading("Joom options").global(true))
        .arg(Arg::new("timeout").long("timeout").value_name("SECONDS").default_value("30").value_parser(value_parser!(u64).range(1..=600)).help("Microsoft request timeout in seconds").help_heading("Joom options").global(true))
        .arg(switch("json", "Output JSON instead of human-readable text").help_heading("Joom options").global(true))
        .arg(switch("verbose", "Include advanced parameters in help").global(true))
        .subcommand(auth);
    for service in SERVICES {
        let mut service_cmd = Command::new(service.cli).about(service.title);
        if selected.is_none() || selected == Some(service.cli) {
            service_cmd=service_cmd
                .override_usage(format!("joom [OPTIONS] {} <ACCOUNT> <COMMAND> [COMMAND_OPTIONS]",service.cli))
                .after_help("Commands: docs/grammar.md\nMicrosoft support: docs/microsoft.md\nRequest files: docs/file-schemas.md")
                .arg(Arg::new("account").value_name("ACCOUNT").required(true).index(1).help("Microsoft account email"))
                .subcommand_precedence_over_arg(true).subcommand_required(true);
            for native in crate::native::commands(service.cli) {
                service_cmd = service_cmd.subcommand(native);
            }
            for operation in crate::schema_commands::commands(service.cli) {
                service_cmd = service_cmd.subcommand(operation);
            }
        }
        cmd = cmd.subcommand(service_cmd);
    }
    if style_help {
        cmd.build();
        crate::help::configure(cmd, None, verbose)
    } else {
        cmd
    }
}

fn select_service(raw: &[OsString]) -> Option<(usize, String)> {
    let mut index = 1;
    while index < raw.len() {
        let word = raw[index].to_str()?;
        match word {
            "--home" | "--timeout" => index += 2,
            "--json" | "--verbose" => index += 1,
            "help" => {
                return raw
                    .get(index + 1)
                    .and_then(|word| word.to_str())
                    .filter(|name| SERVICES.iter().any(|service| service.cli == *name))
                    .map(|service| (index + 1, service.to_owned()));
            }
            _ if word.starts_with("--home=") || word.starts_with("--timeout=") => index += 1,
            _ => {
                return SERVICES
                    .iter()
                    .find(|service| service.cli == word)
                    .map(|service| (index, service.cli.to_owned()));
            }
        }
    }
    None
}
/// Parse context before the action and reject options after it.
pub fn parse_from<I, T>(args: I) -> Result<ArgMatches, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let mut raw = args.into_iter().map(Into::into).collect::<Vec<OsString>>();
    let selected = select_service(&raw);
    let verbose = raw
        .iter()
        .take_while(|arg| arg.as_os_str() != "--")
        .any(|arg| arg.as_os_str() == "--verbose");
    let style_help = verbose
        || raw
            .iter()
            .any(|arg| matches!(arg.to_str(), Some("--help" | "-h" | "help")));
    if verbose {
        let end = raw
            .iter()
            .position(|arg| arg.as_os_str() == "--")
            .unwrap_or(raw.len());
        raw = raw
            .into_iter()
            .enumerate()
            .filter_map(|(index, arg)| {
                (index >= end || arg.as_os_str() != "--verbose").then_some(arg)
            })
            .collect();
    }
    let mut definition = build(
        Some(
            selected
                .as_ref()
                .map(|(_, service)| service.as_str())
                .unwrap_or(""),
        ),
        verbose,
        style_help,
    );
    let matches = definition.try_get_matches_from_mut(&raw).map_err(|error| {
        if error.kind() == clap::error::ErrorKind::DisplayHelp {
            Command::new("joom")
                .override_help(crate::help::order_sections(&error.to_string()))
                .try_get_matches_from(["joom", "--help"])
                .unwrap_err()
        } else {
            error
        }
    })?;
    if verbose {
        return Err(clap::Error::raw(
            clap::error::ErrorKind::ArgumentConflict,
            "--verbose is a help modifier; use --help --verbose\n",
        )
        .with_cmd(&definition));
    }
    let (namespace, context) = matches.subcommand().expect("required namespace");
    let account = context.get_one::<String>("account");
    let action = context.subcommand_name().expect("required command");
    let error = |kind, message: String| clap::Error::raw(kind, message).with_cmd(&definition);
    if namespace == "auth" {
        if matches!(action, "login" | "all" | "logout") && account.is_none() {
            return Err(error(
                clap::error::ErrorKind::MissingRequiredArgument,
                format!(
                    "use joom auth <account> {action}{}\n",
                    if action == "login" {
                        " <service-list>"
                    } else {
                        ""
                    }
                ),
            ));
        }
        if matches!(action, "init" | "enroll" | "test" | "list") && account.is_some() {
            return Err(error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("auth {action} applies to the shared vault; omit the account\n"),
            ));
        }
    }
    let context_definition = definition.find_subcommand(namespace).unwrap();
    let action_definition = context_definition.find_subcommand(action);
    let definitions = definition
        .get_arguments()
        .chain(context_definition.get_arguments())
        .chain(
            action_definition
                .into_iter()
                .flat_map(Command::get_arguments),
        );
    let mut long_options = std::collections::BTreeMap::new();
    let mut short_options = std::collections::BTreeMap::new();
    for arg in definitions {
        let before_action = arg.is_global_set();
        let value = arg.get_action().takes_values() && !arg.is_require_equals_set();
        if let Some(long) = arg.get_long() {
            long_options.insert(long, (value, before_action));
        }
        if let Some(short) = arg.get_short() {
            short_options.insert(short, (value, before_action));
        }
    }
    let mut positional = Vec::new();
    let mut options = Vec::new();
    let mut skip_value = false;
    let mut literal = false;
    for (index, arg) in raw.iter().enumerate().skip(1) {
        let arg = arg.to_string_lossy();
        if skip_value {
            skip_value = false;
            continue;
        }
        if !literal && arg == "--" {
            literal = true;
            continue;
        }
        if !literal && let Some(option) = arg.strip_prefix("--") {
            let name = option.split('=').next().unwrap();
            let &(value, before_action) = long_options.get(name).expect("option validated by clap");
            options.push((index, name.to_owned(), before_action));
            skip_value = value && !option.contains('=');
        } else if !literal && arg.starts_with('-') && arg.len() > 1 {
            let short = arg.chars().nth(1).unwrap();
            let &(value, before_action) = short_options
                .get(&short)
                .expect("short option validated by clap");
            options.push((index, short.to_string(), before_action));
            skip_value = value && arg.len() == 2;
        } else {
            positional.push(index);
        }
    }
    let verb = positional[if account.is_some() { 2 } else { 1 }];
    for (index, option, before_action) in options {
        if before_action && index > verb {
            return Err(error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("joom option --{option} must precede the action {action}\n"),
            ));
        }
        if !before_action && index < verb {
            return Err(error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("command option --{option} must follow the action {action}\n"),
            ));
        }
    }
    Ok(matches)
}

#[cfg(test)]
#[path = "../tests/unit/cli_native_help_tests.rs"]
mod native_help_tests;
#[cfg(test)]
#[path = "../tests/unit/cli.rs"]
mod tests;
