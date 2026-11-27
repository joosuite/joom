use std::process::{Command, Stdio};

#[test]
fn auth_list_reads_only_registrations_and_keeps_the_same_accounts_in_both_formats() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().to_str().unwrap();
    let run = |json: bool| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_joom"));
        command.args(["--home", home]);
        if json {
            command.arg("--json");
        }
        command.args(["auth", "list"]).output().unwrap()
    };
    let empty = run(false);
    assert!(
        empty.status.success(),
        "status {:?}: {}",
        empty.status,
        String::from_utf8_lossy(&empty.stderr)
    );
    assert!(empty.stdout.is_empty());
    assert!(empty.stderr.is_empty());
    assert!(!directory.path().join("joom").exists());
    let empty = run(true);
    assert!(
        empty.status.success(),
        "status {:?}: {}",
        empty.status,
        String::from_utf8_lossy(&empty.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&empty.stdout).unwrap(),
        serde_json::json!([])
    );
    assert!(!directory.path().join("joom").exists());

    let vault = directory.path().join("joom");
    std::fs::create_dir(&vault).unwrap();
    let config = br#"{"account":"second@example.com","accounts":["first@example.com","second@example.com"]}"#;
    std::fs::write(vault.join("config.json"), config).unwrap();
    // No vault identity, master key, or tokens exist: listing needs only config.json.
    let plain = run(false);
    assert!(plain.status.success());
    assert!(plain.stderr.is_empty());
    assert_eq!(
        String::from_utf8(plain.stdout).unwrap(),
        "first@example.com\nsecond@example.com\n"
    );
    let json = run(true);
    assert!(json.status.success());
    assert!(json.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&json.stdout).unwrap(),
        serde_json::json!(["first@example.com", "second@example.com"])
    );
    assert_eq!(std::fs::read(vault.join("config.json")).unwrap(), config);
    assert_eq!(std::fs::read_dir(&vault).unwrap().count(), 1);
}

#[test]
fn root_help_contains_only_supported_namespaces_and_options() {
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    for removed in [
        "--plain",
        "--anonymous",
        "--quota-project",
        "--readonly",
        "--dry-run",
        "--no-input",
        "GOG_",
        "BEHAVIOR",
    ] {
        assert!(!text.contains(removed), "{removed}");
    }
    let commands = text
        .split("Commands:\n")
        .nth(1)
        .unwrap()
        .split("\nOptional parameters:")
        .next()
        .unwrap()
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect::<Vec<_>>();
    assert_eq!(
        commands,
        [
            "auth", "mail", "drive", "calendar", "sheets", "tasks", "contacts", "help"
        ]
    );
}

#[test]
fn login_help_explains_the_all_services_selector() {
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args(["auth", "me@example.com", "login", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("login all"));
    assert!(help.contains("every supported service"));
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args(["auth", "me@example.com", "all", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("auth <ACCOUNT> all"));
    assert!(help.contains("authorize all supported services"));
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args(["auth", "me@example.com", "all", "--help", "--verbose"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .contains("Equivalent to login all")
    );
}

#[test]
fn help_modes_show_requirements_and_conflicts_without_executing() {
    let run = |arguments: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_joom"))
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(!text.starts_with("error:"));
        text
    };
    let common = run(&["mail", "ACCOUNT", "messages.send", "--help"]);
    for flag in [
        "--to <EMAIL>",
        "--cc <EMAIL>",
        "--bcc <EMAIL>",
        "--subject <TEXT>",
    ] {
        assert!(common.contains(flag));
    }
    for flag in ["--raw <RAW>", "--history-id <", "--home <PATH>"] {
        assert!(!common.contains(flag));
    }
    assert!(common.contains("at least one recipient with --to, --cc, or --bcc"));
    assert!(common.contains("Optional parameters:"));
    assert!(common.contains("Use --text for inline content or --text-file to read a file."));
    for text in [
        "Conflicting with",
        "after merging",
        "Alternative input",
        "Mandatory requirements",
    ] {
        assert!(!common.contains(text), "developer wording in help: {text}");
    }
    let advanced = run(&["mail", "ACCOUNT", "messages.send", "--help", "--verbose"]);
    assert!(advanced.contains("Optional parameters (advanced):"));
    assert!(advanced.contains("--raw <RAW>"));
    assert!(advanced.contains("--home <PATH>"));
    let help_command = run(&["help", "mail", "messages.send", "--verbose"]);
    assert_eq!(advanced, help_command);
    let required = run(&["drive", "ACCOUNT", "files.get", "--help"]);
    assert!(
        required.find("Mandatory parameters:").unwrap()
            < required.find("Optional parameters:").unwrap()
    );
    assert!(
        required
            .split("Optional parameters:")
            .next()
            .unwrap()
            .contains("--file-id")
    );
    let instances = run(&["calendar", "ACCOUNT", "events.instances", "--help"]);
    let mandatory = instances.split("Optional parameters:").next().unwrap();
    for flag in ["--event-id", "--time-min", "--time-max"] {
        assert!(mandatory.contains(flag));
    }
}

#[test]
fn removed_interfaces_fail_before_execution() {
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args([
            "auth",
            "me@example.com",
            "login",
            "--scopes",
            "Files.ReadWrite",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());

    for option in [
        "--plain",
        "--anonymous",
        "--quota-project",
        "--readonly",
        "--dry-run",
        "--no-input",
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_joom"))
            .args([option, "drive", "me@example.com", "ls"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    for service in [
        "docs",
        "slides",
        "forms",
        "youtube",
        "photos",
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
        let output = Command::new(env!("CARGO_BIN_EXE_joom"))
            .args([service, "me@example.com", "list"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn legacy_body_inputs_are_rejected_without_reading_stdin_or_files() {
    let home = tempfile::tempdir().unwrap();
    let input = home.path().join("body.json");
    std::fs::write(&input, br#"{"name":"must-not-be-read"}"#).unwrap();
    for body in ["@-".to_owned(), format!("@{}", input.display())] {
        let output = Command::new(env!("CARGO_BIN_EXE_joom"))
            .args([
                "--home",
                home.path().to_str().unwrap(),
                "drive",
                "me@example.com",
                "files.create",
                "--body",
                &body,
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON input"));
        assert!(!home.path().join("rust").exists());
    }
}

#[test]
fn invalid_parameter_file_fails_before_network_or_vault_access() {
    for json in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let input = home.path().join("request.json");
        std::fs::write(&input, b"[]").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_joom"));
        if json {
            command.arg("--json");
        }
        let output = command
            .args([
                "--home",
                home.path().to_str().unwrap(),
                "drive",
                "me@example.com",
                "files.list",
                "--file",
                input.to_str().unwrap(),
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        if json {
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], "--file must contain a JSON object");
        } else {
            assert_eq!(output.stderr, b"error: --file must contain a JSON object\n");
        }
        assert!(!home.path().join("rust").exists());
    }
}

#[test]
fn invalid_mail_composition_fails_before_network_or_vault_access() {
    for json in [false, true] {
        let home = tempfile::tempdir().unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_joom"));
        if json {
            command.arg("--json");
        }
        let output = command
            .args([
                "--home",
                home.path().to_str().unwrap(),
                "mail",
                "me@example.com",
                "messages.send",
                "--subject",
                "Missing recipient",
                "--text",
                "Hello",
            ])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let expected = "When composing a message to send, provide at least one recipient with --to, --cc, or --bcc.";
        if json {
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], expected);
        } else {
            assert_eq!(
                String::from_utf8(output.stderr).unwrap(),
                format!("error: {expected}\n")
            );
        }
        assert!(!home.path().join("rust").exists());
    }
}

#[test]
fn wrapper_entry_points_show_help_and_validate_without_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_joom"));
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/wrappers.schema.json")).unwrap();
    for name in schema["x-wrappers"].as_object().unwrap().keys() {
        let executable = directory
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::hard_link(binary, &executable)
            .or_else(|_| std::fs::copy(binary, &executable).map(|_| ()))
            .unwrap();
        let output = Command::new(&executable).arg("--help").output().unwrap();
        assert!(output.status.success(), "{name}: {:?}", output.stderr);
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains(&format!("Usage: {name} ACCOUNT")));
        assert!(!help.contains("Conflicting with"));
        let output = Command::new(&executable)
            .args(["me@example.com", "json:true", "timeout:invalid"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{name}");
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert!(error["error"].is_string(), "{name}");
    }
    let executable = directory
        .path()
        .join(format!("jmmail{}", std::env::consts::EXE_SUFFIX));
    let output = Command::new(executable)
        .args(["me@example.com", "send", "--help", "--verbose"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("home:PATH"));
    assert!(help.contains("Input choices:"));
    assert!(!directory.path().join("rust").exists());
}
