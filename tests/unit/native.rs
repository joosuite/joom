use super::*;
use crate::{cli, discovery::Document, pagination::Plan, services::SERVICES};

fn sample(name: &str) -> &'static str {
    match name {
        "subject" | "title" | "given-name" | "name" => "Example",
        "text" | "notes" => "Message text",
        "query" => "test query",
        "range" => "Sheet1!A1:B2",
        "values" => "[[\"Name\",1],[\"Other\",2]]",
        "start" => "2026-12-01T10:00:00+01:00",
        "end" => "2026-12-01T11:00:00+01:00",
        "to" | "cc" | "bcc" => "recipient@example.com",
        "type" => "application/pdf",
        _ => "example-id",
    }
}

fn check_body(
    schema: &Value,
    value: &Value,
    schemas: &std::collections::BTreeMap<String, Value>,
    path: &str,
) {
    if let Some(name) = schema["$ref"].as_str() {
        return check_body(&schemas[name], value, schemas, path);
    }
    if value.is_null() {
        return;
    }
    match schema["type"].as_str().unwrap_or_default() {
        "object" => {
            let object = value
                .as_object()
                .unwrap_or_else(|| panic!("{path}: expected object"));
            for (key, child) in object {
                let child_schema = schema["properties"]
                    .get(key)
                    .or_else(|| schema.get("additionalProperties"))
                    .unwrap_or_else(|| panic!("{path}.{key}: unknown property"));
                assert_ne!(
                    child_schema["readOnly"], true,
                    "{path}.{key}: output-only property"
                );
                check_body(child_schema, child, schemas, &format!("{path}.{key}"));
            }
        }
        "array" => {
            for (index, child) in value.as_array().unwrap().iter().enumerate() {
                check_body(
                    &schema["items"],
                    child,
                    schemas,
                    &format!("{path}[{index}]"),
                );
            }
        }
        "string" => assert!(value.is_string(), "{path}: expected string"),
        "integer" => assert!(
            value.as_i64().is_some() || value.as_u64().is_some(),
            "{path}: expected integer"
        ),
        "number" => assert!(value.is_number(), "{path}: expected number"),
        "boolean" => assert!(value.is_boolean(), "{path}: expected boolean"),
        _ => {}
    }
    if let Some(choices) = schema["enum"].as_array() {
        assert!(choices.contains(value), "{path}: invalid enum value");
    }
}

#[test]
fn every_native_command_builds_its_supported_request() {
    let fixtures: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native-requests.json"
    )))
    .unwrap();
    let home = tempfile::tempdir().unwrap();
    let upload = home.path().join("upload.txt");
    std::fs::write(&upload, "Example upload").unwrap();
    let output = home.path().join("download.txt");
    let mut tested = 0;
    for service in SERVICES {
        let doc: Document = serde_json::from_value(fixtures[service.cli].clone()).unwrap();
        let commands = commands(service.cli);
        assert_eq!(
            commands.len(),
            doc.root.methods.len(),
            "{} command coverage",
            service.cli
        );
        for cmd in commands {
            let action = cmd.get_name();
            let mut args = vec![
                "joom".to_owned(),
                service.cli.into(),
                "sender@example.com".into(),
                action.into(),
            ];
            for arg in cmd
                .get_arguments()
                .filter(|arg| arg.is_required_set() || arg.get_long().is_none())
            {
                let name = arg.get_id().as_str();
                if let Some(long) = arg.get_long() {
                    args.push(format!("--{long}"));
                }
                let value = match name {
                    "path" => upload.to_str().unwrap(),
                    "output" => output.to_str().unwrap(),
                    _ => sample(name),
                };
                args.push(value.into());
            }
            match (service.cli, action) {
                ("mail", "send") => args.extend(["--to".into(), sample("to").into()]),
                ("calendar" | "tasks", "update") => {
                    args.extend(["--title".into(), "Changed title".into()])
                }
                ("contacts", "create") => args.extend(["--given-name".into(), "Alice".into()]),
                _ => {}
            }
            let root = cli::parse_from(&args).unwrap_or_else(|error| panic!("{args:?}: {error}"));
            let context = root.subcommand().unwrap().1;
            let native = context.subcommand().unwrap().1;
            let call = resolve(
                service.cli,
                "sender@example.com",
                action,
                native,
                Map::new(),
            )
            .unwrap_or_else(|error| panic!("{args:?}: {error:#}"));
            assert_eq!(
                call.method, fixtures[service.cli]["x-expected-methods"][action],
                "{args:?}"
            );
            let contract = crate::schema_commands::contract(service.cli);
            crate::microsoft::validate(
                &contract.document,
                contract.document.method(&call.method).unwrap(),
                &call.params,
                call.body.as_ref(),
                false,
                false,
            )
            .unwrap_or_else(|error| panic!("{} {action}: {error:#}", service.cli));
            let spec = &doc.root.methods[action];
            let url = doc
                .url(spec, &call.params)
                .unwrap_or_else(|error| panic!("{args:?}: {error:#}"));
            assert_eq!(url.scheme(), "https");
            if let Some(body) = &call.body {
                check_body(
                    spec.request.as_ref().expect("write request schema"),
                    body,
                    &doc.schemas,
                    &format!("{} {action}", service.cli),
                );
            }
            let read = spec.http_method == "GET";
            Plan::new(spec, read, None, None).unwrap();
            if cmd.get_arguments().any(|arg| arg.get_id() == "max") {
                Plan::new(spec, read, Some(2), None)
                    .unwrap_or_else(|error| panic!("{} {action}: {error}", service.cli));
            }
            if cmd.get_arguments().any(|arg| arg.get_id() == "max-pages") {
                Plan::new(spec, read, None, Some(2))
                    .unwrap_or_else(|error| panic!("{} {action}: {error}", service.cli));
            }
            let scopes = crate::services::scopes(service.cli).unwrap_or_default();
            assert!(
                spec.scopes.is_empty() || spec.scopes.iter().any(|scope| scopes.contains(scope)),
                "{} {action}: missing OAuth scope",
                service.cli
            );
            let mut json_args = args.clone();
            json_args.insert(3, "--json".into());
            let json = cli::parse_from(&json_args).unwrap();
            assert!(json.get_flag("json"));
            assert!(arguments(json.subcommand().unwrap().1).get_flag("json"));
            let mut misplaced = args.clone();
            misplaced.push("--json".into());
            assert!(
                cli::parse_from(misplaced).is_err(),
                "global options after {service_name} {action}",
                service_name = service.cli
            );
            tested += 1;
        }
    }
    assert_eq!(tested, 6);
}

fn prepare(service: &str, tail: &[&str]) -> Result<Invocation> {
    let args = vec!["joom", service, "sender@example.com"]
        .into_iter()
        .chain(tail.iter().copied());
    let root = cli::parse_from(args)?;
    let context = root.subcommand().unwrap().1;
    let (action, args) = context.subcommand().unwrap();
    resolve(service, "sender@example.com", action, args, Map::new())
}

#[test]
fn command_defaults_do_not_override_parameter_files() {
    let root = cli::parse_from(["joom", "calendar", "sender@example.com", "events"]).unwrap();
    let args = arguments(root.subcommand().unwrap().1);
    let params = json!({"calendarId":"selected-calendar","timeMin":"2026-12-01T00:00:00Z","timeMax":"2026-12-03T00:00:00Z"});
    let call = resolve(
        "calendar",
        "sender@example.com",
        "events",
        args,
        params.as_object().unwrap().clone(),
    )
    .unwrap();
    assert_eq!(call.params["calendarId"], "selected-calendar");
    let root = cli::parse_from([
        "joom",
        "calendar",
        "sender@example.com",
        "events",
        "--calendar",
        "explicit-calendar",
    ])
    .unwrap();
    let args = arguments(root.subcommand().unwrap().1);
    let call = resolve(
        "calendar",
        "sender@example.com",
        "events",
        args,
        params.as_object().unwrap().clone(),
    )
    .unwrap();
    assert_eq!(call.params["calendarId"], "explicit-calendar");
    assert!(!call.custom_fields);
    let root = cli::parse_from([
        "joom",
        "drive",
        "sender@example.com",
        "files.get",
        "--file-id",
        "document",
        "--fields",
        "title",
    ])
    .unwrap();
    let call = crate::service_call(
        crate::services::SERVICES
            .iter()
            .find(|s| s.cli == "drive")
            .unwrap(),
        root.subcommand().unwrap().1,
    )
    .unwrap();
    assert!(call.custom_fields);
}

#[test]
fn relative_calendar_windows_cache_by_intent_and_do_not_collide_with_unbounded_requests() {
    let fixtures: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native-requests.json"
    )))
    .unwrap();
    let doc: Document = serde_json::from_value(fixtures["calendar"].clone()).unwrap();
    let spec = &doc.root.methods["events"];
    let mut call = prepare("calendar", &["events"]).unwrap();
    let key = |call: &Invocation| {
        crate::cache::request_identity(&doc, spec, &call.params, &call.relative_times).unwrap()
    };
    let first = key(&call);
    assert!(call.params.contains_key("timeMin") && call.params.contains_key("timeMax"));
    call.params
        .insert("timeMin".into(), json!("2026-12-02T00:00:00Z"));
    call.params
        .insert("timeMax".into(), json!("2027-01-01T00:00:00Z"));
    assert_eq!(first, key(&call));
    let mut unbounded = call.params.clone();
    unbounded.remove("timeMin");
    unbounded.remove("timeMax");
    let generic = crate::cache::request_identity(&doc, spec, &unbounded, &[]).unwrap();
    assert_ne!(first, generic);
    let explicit = prepare(
        "calendar",
        &[
            "events",
            "--from",
            "2026-12-01T00:00:00Z",
            "--until",
            "2026-12-04T00:00:00Z",
        ],
    )
    .unwrap();
    assert_eq!(
        key(&explicit),
        doc.url(spec, &explicit.params).unwrap().as_str()
    );
    assert_ne!(first, key(&explicit));
}
#[test]
fn selected_actions_preserve_filtering_label_changes_and_response_ids() {
    let call = prepare("mail", &["read", "message"]).unwrap();
    assert_eq!(call.body.unwrap(), json!({"removeLabelIds":["UNREAD"]}));
    assert!(prepare("drive", &["restore", "file"]).is_err());
    let call = prepare("drive", &["search", "O'Brien", "--parent", "folder"]).unwrap();
    assert!(call.params["q"].as_str().unwrap().contains("O\\'Brien"));

    assert!(
        prepare(
            "calendar",
            &[
                "events",
                "--from",
                "2026-12-02T00:00:00Z",
                "--until",
                "2026-12-01T00:00:00Z"
            ]
        )
        .is_err()
    );
    assert!(
        prepare(
            "forms",
            &["responses", "form", "--since", "2026-12-01T00:00:00+01:00"]
        )
        .is_err()
    );
}
