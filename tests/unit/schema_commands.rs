use super::*;
use crate::{cli, services::SERVICES};

#[test]
fn cli_schemas_cover_every_rest_input_and_registered_option() {
    for service in SERVICES {
        let contract = contract(service.cli);
        for (name, method) in &contract.document.root.methods {
            let actual = bindings(&contract.document, method);
            for expected in rest_bindings(&contract.document, method) {
                assert!(
                    actual
                        .iter()
                        .any(|binding| binding.id == expected.id && binding.long == expected.long),
                    "{} {name}: missing {}",
                    service.cli,
                    expected.long
                );
            }
            let command = command(service.cli, name);
            for shape in contract.inputs[name]["properties"]
                .as_object()
                .unwrap()
                .values()
            {
                if let Some(long) = shape["x-cli-option"].as_str() {
                    assert!(
                        command
                            .get_arguments()
                            .any(|arg| arg.get_long() == Some(long)),
                        "{} {name}: schema has unsupported --{long}",
                        service.cli
                    );
                }
            }
            for arg in command
                .get_arguments()
                .filter(|arg| arg.get_long().is_some_and(|long| long != "file"))
            {
                let long = arg.get_long().unwrap();
                assert!(
                    contract.inputs[name]["properties"]
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|shape| shape["x-cli-option"] == long),
                    "{} {name}: --{long} is absent from its CLI schema",
                    service.cli
                );
            }
        }
        for command in native::commands(service.cli) {
            let name = command.get_name();
            for arg in command
                .get_arguments()
                .filter(|arg| arg.get_long() != Some("file"))
            {
                assert!(
                    contract.inputs[name]["properties"]
                        .as_object()
                        .unwrap()
                        .values()
                        .any(|shape| shape["x-cli-binding"]["property"] == arg.get_id().as_str()),
                    "{} {name}: native input {} has no schema",
                    service.cli,
                    arg.get_id()
                );
            }
        }
    }
}

fn scalar(parameter: &Parameter) -> Value {
    if let Some(value) = parameter.choices.first() {
        return json!(value);
    }
    match parameter.kind.as_str() {
        "boolean" => json!(true),
        "integer" => json!(
            parameter
                .minimum
                .as_deref()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(1)
                .max(1)
        ),
        "number" => json!(1),
        _ => json!("example-id"),
    }
}
fn sample(document: &Document, schema: &Value, method: &str, depth: usize) -> Value {
    if depth > 32 {
        return Value::Null;
    }
    let shape = type_schema(document, schema);
    if let Some(value) = shape["enum"].as_array().and_then(|values| values.first()) {
        return value.clone();
    }
    match shape["type"].as_str().unwrap_or("object") {
        "string" => json!("Example"),
        "integer" | "number" => json!(1),
        "boolean" => json!(true),
        "array" => json!([]),
        _ => {
            let mut fields = Map::new();
            for (key, value) in shape["properties"].as_object().into_iter().flatten() {
                if required_for(value, method)
                    || shape["x-required-one-of"][method]
                        .as_array()
                        .and_then(|fields| fields.first())
                        .is_some_and(|field| field == key)
                    || shape
                        .get("required")
                        .or_else(|| shape.get("x-required-fields"))
                        .and_then(Value::as_array)
                        .is_some_and(|required| required.iter().any(|name| name == key))
                {
                    fields.insert(key.clone(), sample(document, value, method, depth + 1));
                }
            }
            Value::Object(fields)
        }
    }
}
pub(super) fn prepare(
    service: &str,
    arguments: &[&str],
    file: Option<&Value>,
) -> Result<Invocation> {
    let file_path = tempfile::NamedTempFile::new()?;
    if let Some(file) = file {
        std::fs::write(file_path.path(), serde_json::to_vec(file)?)?;
    }
    let mut args = vec!["joom", service, "reader@example.com"];
    args.extend(arguments);
    if file.is_some() {
        args.extend(["--file", file_path.path().to_str().unwrap()]);
    }
    let parsed = cli::parse_from(args)?;
    let context = parsed.subcommand().unwrap().1;
    let service = SERVICES.iter().find(|item| item.cli == service).unwrap();
    crate::service_call(service, context)
}

fn mime(call: &Invocation) -> String {
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    let body = call.body.as_ref().unwrap();
    let raw = body.get("message").unwrap_or(body)["raw"].as_str().unwrap();
    String::from_utf8(URL_SAFE_NO_PAD.decode(raw).unwrap()).unwrap()
}

#[test]
fn flat_cli_parameter_files_match_options_for_every_service() {
    let cases: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cli-inputs.json"
    )))
    .unwrap();
    assert_eq!(cases.as_array().unwrap().len(), SERVICES.len());
    for case in cases.as_array().unwrap() {
        let service = case["service"].as_str().unwrap();
        let name = case["command"].as_str().unwrap();
        let mut arguments = vec![name];
        arguments.extend(
            case["cli"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap()),
        );
        let from_cli = prepare(service, &arguments, None)
            .unwrap_or_else(|error| panic!("{service} {name} CLI: {error:#}"));
        let from_file = prepare(service, &[name], Some(&case["file"]))
            .unwrap_or_else(|error| panic!("{service} {name} file: {error:#}"));
        assert_eq!(
            from_file.params, from_cli.params,
            "{service} {name} parameters"
        );
        if service == "mail" {
            for call in [&from_cli, &from_file] {
                let raw = mime(call);
                for header in [
                    "To: <alice@example.com>",
                    "Cc: <bob@example.com>",
                    "Bcc: <archive@example.com>",
                    "Subject: Report",
                    "Hello",
                ] {
                    assert!(raw.contains(header), "missing {header}");
                }
            }
        } else {
            assert_eq!(
                from_file.body.as_ref(),
                Some(&case["body"]),
                "{service} {name} request body"
            );
            assert_eq!(from_file.body, from_cli.body, "{service} {name} body");
        }
    }
}

#[test]
fn file_controls_and_native_positionals_share_cli_validation_and_overrides() {
    let call = prepare(
        "drive",
        &["files.list", "--max", "2"],
        Some(&json!({"max":5,"maxPages":3,"ttl":10,"pageSize":7})),
    )
    .unwrap();
    let options = call.options.unwrap();
    assert_eq!(options.get_one::<u32>("max"), Some(&2));
    assert_eq!(options.get_one::<u32>("max-pages"), Some(&3));
    assert_eq!(options.get_one::<u32>("ttl"), Some(&10));
    assert_eq!(call.params["pageSize"], 7);
    for invalid in [
        json!({"max":0}),
        json!({"maxPages":10001}),
        json!({"ttl":-1}),
        json!({"max":false}),
    ] {
        assert!(
            prepare("drive", &["files.list"], Some(&invalid)).is_err(),
            "accepted {invalid}"
        );
    }
    let native = prepare("mail", &["read"], Some(&json!({"id":"message"}))).unwrap();
    assert_eq!(native.params["id"], "message");
    assert_eq!(native.body.unwrap(), json!({"removeLabelIds":["UNREAD"]}));
    assert!(prepare("mail", &["read"], Some(&json!({}))).is_err());
    let search = prepare(
        "drive",
        &["search", "explicit"],
        Some(&json!({"query":"file query","parent":"folder","max":4})),
    )
    .unwrap();
    let q = search.params["q"].as_str().unwrap();
    assert!(q.contains("explicit") && !q.contains("file query") && q.contains("folder"));
    assert_eq!(search.options.unwrap().get_one::<u32>("max"), Some(&4));
    let upload = prepare(
        "drive",
        &["files.create"],
        Some(&json!({"name":"Report","upload":"local.pdf","contentType":"application/pdf"})),
    )
    .unwrap();
    assert_eq!(
        native::text(upload.options.as_ref().unwrap(), "upload"),
        Some("local.pdf")
    );
    assert_eq!(
        native::text(upload.options.as_ref().unwrap(), "content-type"),
        Some("application/pdf")
    );
    let download = prepare(
        "drive",
        &["files.export"],
        Some(&json!({"fileId":"doc","mimeType":"application/pdf","output":"report.pdf"})),
    )
    .unwrap();
    assert_eq!(
        native::text(download.options.as_ref().unwrap(), "output"),
        Some("report.pdf")
    );
}

#[test]
fn merged_constraints_reject_files_and_raw_bodies_before_execution() {
    for (service, arguments, file) in [
        (
            "calendar",
            vec!["events.insert", "--start-time", "2026-12-01T10:00:00Z"],
            json!({"startDate":"2026-12-01","endDate":"2026-12-02"}),
        ),
        (
            "calendar",
            vec!["events.insert"],
            json!({"body":{"start":{},"end":{"date":"2026-12-02"}}}),
        ),
        (
            "photos",
            vec!["media-items.search"],
            json!({"albumId":"album","mediaTypes":["PHOTO"]}),
        ),
        (
            "photos",
            vec!["media-items.search"],
            json!({"body":{"albumId":"album","filters":{}}}),
        ),
        (
            "mail",
            vec!["messages.send", "--text", "Body"],
            json!({"to":["recipient@example.com"],"raw":"encoded"}),
        ),
        ("forms", vec!["create"], json!({})),
    ] {
        assert!(
            prepare(service, &arguments, Some(&file)).is_err(),
            "accepted {service} {arguments:?} {file}"
        );
    }
    // Explicit options replace their file counterpart before conflict checks.
    assert!(
        prepare(
            "mail",
            &["messages.send", "--text", "Body"],
            Some(&json!({"to":["recipient@example.com"],"textFile":"must-not-be-read.txt"}))
        )
        .is_ok()
    );
    // A partial event patch can update one timestamp without replacing the event.
    assert!(
        prepare(
            "calendar",
            &["events.patch", "--event-id", "event"],
            Some(&json!({"startTime":"2026-12-01T10:00:00+01:00","startTimeZone":"Europe/Paris"}))
        )
        .is_ok()
    );
}

#[test]
fn input_validation_applies_to_every_service() {
    let cases: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/cli-inputs.json"
    )))
    .unwrap();
    for (service, invalid) in [
        ("mail", json!({"subject":42})),
        ("drive", json!({"name":42})),
        ("calendar", json!({"summary":42})),
        ("sheets", json!({"title":42})),
        ("tasks", json!({"title":42})),
        ("contacts", json!({"names":42})),
    ] {
        let case = cases
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["service"] == service)
            .unwrap();
        let mut file = case["file"].as_object().unwrap().clone();
        file.extend(invalid.as_object().unwrap().clone());
        assert!(
            prepare(
                service,
                &[case["command"].as_str().unwrap()],
                Some(&Value::Object(file))
            )
            .is_err(),
            "accepted invalid {service} inputs"
        );
    }
}

#[test]
fn mail_composition_builds_recipients_and_mime_for_messages_and_drafts() {
    let call = prepare(
        "mail",
        &[
            "messages.send",
            "--to",
            "alice@example.com",
            "--to",
            "bob@example.com",
            "--cc",
            "carol@example.com",
            "--bcc",
            "archive@example.com",
            "--reply-to",
            "replies@example.com",
            "--subject",
            "Report",
            "--text",
            "Plain report",
            "--html",
            "<p>HTML report</p>",
        ],
        None,
    )
    .unwrap();
    assert_eq!(call.params, Map::new());
    let message = mime(&call);
    for header in [
        "From: <reader@example.com>",
        "To: <alice@example.com>, <bob@example.com>",
        "Cc: <carol@example.com>",
        "Bcc: <archive@example.com>",
        "Reply-To: <replies@example.com>",
        "Subject: Report",
        "multipart/alternative",
        "text/plain",
        "text/html",
        "Plain report",
        "<p>HTML report</p>",
    ] {
        assert!(message.contains(header), "missing {header}: {message}");
    }
    let draft = prepare(
        "mail",
        &["drafts.create", "--subject", "Unfinished draft"],
        None,
    )
    .unwrap();
    assert!(draft.body.as_ref().unwrap()["message"]["raw"].is_string());
    assert!(mime(&draft).contains("Subject: Unfinished draft"));
    // A Bcc-only send is valid, and an explicit sender replaces ACCOUNT.
    let blind = prepare(
        "mail",
        &[
            "messages.send",
            "--bcc",
            "archive@example.com",
            "--from",
            "alias@example.com",
        ],
        None,
    )
    .unwrap();
    assert!(mime(&blind).contains("From: <alias@example.com>"));
    assert!(
        prepare(
            "mail",
            &["drafts.send"],
            Some(&json!({"id":"draft-id","to":["recipient@example.com"]}))
        )
        .is_err()
    );
    let existing = prepare("mail", &["drafts.send", "--id", "draft-id"], None).unwrap();
    assert_eq!(existing.body.unwrap(), json!({"id":"draft-id"}));
}

#[test]
fn mail_composition_reads_utf8_body_files_and_binary_attachments() {
    let dir = tempfile::tempdir().unwrap();
    let text = dir.path().join("body.txt");
    let attachment = dir.path().join("report.bin");
    std::fs::write(&text, "A UTF-8 body: café").unwrap();
    std::fs::write(&attachment, [0, 1, 2, 255]).unwrap();
    let call = prepare(
        "mail",
        &[
            "messages.send",
            "--to",
            "alice@example.com",
            "--subject",
            "Résumé",
            "--text-file",
            text.to_str().unwrap(),
            "--attach",
            attachment.to_str().unwrap(),
        ],
        None,
    )
    .unwrap();
    let message = mime(&call);
    assert!(message.contains("multipart/mixed"));
    assert!(message.contains("application/octet-stream"));
    assert!(message.contains("report.bin"));
    assert!(message.contains("AAEC/w=="));
    assert!(message.to_ascii_lowercase().contains("=?utf-8?"));
    let invalid = dir.path().join("invalid.txt");
    std::fs::write(&invalid, [255]).unwrap();
    assert!(
        prepare(
            "mail",
            &[
                "messages.send",
                "--to",
                "alice@example.com",
                "--text-file",
                invalid.to_str().unwrap()
            ],
            None
        )
        .is_err()
    );
}

#[test]
fn mail_composition_enforces_input_and_encoded_request_limits() {
    let file = tempfile::NamedTempFile::new().unwrap();
    file.as_file().set_len(http::DISCOVERY_LIMIT + 1).unwrap();
    let error = prepare(
        "mail",
        &[
            "messages.send",
            "--to",
            "alice@example.com",
            "--attach",
            file.path().to_str().unwrap(),
        ],
        None,
    )
    .err()
    .unwrap();
    assert!(format!("{error:#}").contains("16 MiB input limit"));
    // The MIME content fits the input limit but its base64url JSON does not.
    std::fs::write(file.path(), "x".repeat(13 * 1024 * 1024)).unwrap();
    let error = prepare(
        "mail",
        &[
            "messages.send",
            "--to",
            "alice@example.com",
            "--text-file",
            file.path().to_str().unwrap(),
        ],
        None,
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("16 MiB JSON request limit"));
}

#[test]
fn invalid_or_ambiguous_mail_composition_is_rejected_locally() {
    for (args, file) in [
        (
            vec!["messages.send", "--subject", "No recipient"],
            json!({}),
        ),
        (
            vec!["messages.send", "--to", "Name <alice@example.com>"],
            json!({}),
        ),
        (
            vec!["messages.send", "--to", "alice@example.com,bob@example.com"],
            json!({}),
        ),
        (
            vec![
                "messages.send",
                "--to",
                "alice@example.com\r\nBcc:evil@example.com",
            ],
            json!({}),
        ),
        (
            vec![
                "messages.send",
                "--to",
                "alice@example.com",
                "--subject",
                "Hello\nInjected: value",
            ],
            json!({}),
        ),
        (
            vec![
                "messages.send",
                "--to",
                "alice@example.com",
                "--upload",
                "must-not-be-read.eml",
            ],
            json!({}),
        ),
        (
            vec!["messages.send", "--to", "alice@example.com"],
            json!({"body":{"raw":"encoded"}}),
        ),
        (
            vec!["messages.send", "--to", "alice@example.com"],
            json!({"body":{"payload":{}}}),
        ),
        (
            vec![
                "messages.send",
                "--to",
                "alice@example.com",
                "--text",
                "inline",
                "--text-file",
                "other.txt",
            ],
            json!({}),
        ),
        (
            vec!["messages.send"],
            json!({"compose":{"to":["alice@example.com"],"text":"inline","textFile":"other.txt"}}),
        ),
        (
            vec!["messages.send"],
            json!({"compose":{"to":"alice@example.com"}}),
        ),
        (
            vec!["messages.send"],
            json!({"compose":{"to":["alice@example.com"],"unknown":true}}),
        ),
        (vec!["messages.send"], json!({"compose":null})),
        (
            vec!["drafts.send"],
            json!({"compose":{"to":["alice@example.com"]}}),
        ),
    ] {
        assert!(
            prepare("mail", &args, Some(&file)).is_err(),
            "accepted {args:?} {file}"
        );
    }
}

#[test]
fn adversarial_input_types_are_rejected_for_all_public_parameters() {
    let mut commands = 0;
    let mut parameters = 0;
    let mut arbitrary_json = 0;
    for service in SERVICES {
        let contract = contract(service.cli);
        for (name, schema) in &contract.inputs {
            for (key, shape) in schema["properties"].as_object().unwrap() {
                let resolved = type_schema(&contract.document, shape);
                // Discovery deliberately leaves a few marker values untyped.
                if resolved.get("type").is_none() && resolved.get("enum").is_none() {
                    arbitrary_json += 1;
                    continue;
                }
                let invalid = if resolved["type"] == "object" {
                    json!([])
                } else {
                    json!({"injected":true})
                };
                assert!(
                    validate_body(&contract.document, "", shape, &invalid, key).is_err(),
                    "{} {name} accepted the wrong JSON type for {key}",
                    service.cli
                );
                parameters += 1;
            }
            commands += 1;
        }
    }
    assert_eq!(commands, 77);
    assert_eq!(parameters + arbitrary_json, 436);
}

#[test]
fn every_schema_method_has_a_registered_command_and_builds_the_declared_request() {
    let mut implemented = 0;
    let file = tempfile::NamedTempFile::new().unwrap();
    for service in SERVICES {
        let doc = &contract(service.cli).document;
        for (name, spec) in &doc.root.methods {
            command(service.cli, name).debug_assert();
            let help = cli::parse_from(["joom", "help", service.cli, name]).unwrap_err();
            assert_eq!(
                help.kind(),
                clap::error::ErrorKind::DisplayHelp,
                "{} {name}",
                service.cli
            );
            let mut params = Map::new();
            for (key, parameter) in &spec.parameters {
                if parameter.required {
                    params.insert(key.clone(), scalar(parameter));
                }
            }
            if let Some(request) = input_request(spec) {
                params.insert("body".into(), sample(doc, request, &spec.id, 0));
            }
            if contract(service.cli).inputs[name]["allOf"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|rule| {
                    rule["x-cli-kind"] == "required"
                        && rule["x-cli-parameters"] == json!(["upload"])
                })
            {
                params.insert("upload".into(), json!("fixture.png"));
            }
            std::fs::write(file.path(), serde_json::to_vec(&params).unwrap()).unwrap();
            let mut argv = vec![
                "joom",
                service.cli,
                "reader@example.com",
                name,
                "--file",
                file.path().to_str().unwrap(),
            ];
            if direct_download(spec) {
                argv.extend(["--output", "download.bin"]);
            }
            let matches = cli::parse_from(argv)
                .unwrap_or_else(|error| panic!("{} {name}: {error}", service.cli));
            let result = crate::service_call(service, matches.subcommand().unwrap().1);
            let call = result.unwrap_or_else(|error| panic!("{} {name}: {error:#}", service.cli));
            assert_eq!(call.method, *name);
            assert_eq!(doc.method(name).unwrap().http_method, spec.http_method);
            let url = doc
                .url_with_body(spec, &call.params, call.body.as_ref())
                .unwrap();
            crate::http::contract_url(&url).unwrap();
            for (key, value) in &params {
                if !matches!(key.as_str(), "body" | "upload") {
                    assert_eq!(
                        call.params.get(key),
                        Some(value),
                        "{} {name}: file input {key} was lost",
                        service.cli
                    );
                }
            }
            if let Some(body) = call.body.as_ref() {
                validate_body(doc, &spec.id, spec.request.as_ref().unwrap(), body, "body").unwrap();
            }
            let scopes = crate::services::scopes(service.cli).unwrap_or_default();
            assert!(
                spec.scopes.is_empty() || spec.scopes.iter().any(|scope| scopes.contains(scope)),
                "{} {name}: missing login permission",
                service.cli
            );
            if spec.media_upload.is_some() {
                crate::microsoft::validate(
                    doc,
                    spec,
                    &call.params,
                    call.body.as_ref(),
                    true,
                    false,
                )
                .unwrap();
            }
            let fields = command(service.cli, name)
                .get_arguments()
                .filter_map(|argument| argument.get_long())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            assert!(!fields.iter().any(|field| field == "params"));
            if fields.iter().any(|field| field == "max") {
                pagination::Plan::new(spec, true, Some(1), None).unwrap();
            }
            if fields.iter().any(|field| field == "max-pages") {
                pagination::Plan::new(spec, true, None, Some(1)).unwrap();
            }
            params.insert("__proto__".into(), json!({"injected":true}));
            std::fs::write(file.path(), serde_json::to_vec(&params).unwrap()).unwrap();
            assert!(
                crate::service_call(service, matches.subcommand().unwrap().1).is_err(),
                "{} {name} accepted an unknown file field",
                service.cli
            );
            implemented += 1;
        }
    }
    assert_eq!(implemented, 71);
}

#[test]
fn named_parameters_and_body_fields_work_with_files_and_preserve_explicit_values() {
    let call = prepare(
        "drive",
        &[
            "files.update",
            "--file-id",
            "document",
            "--name",
            "New name",
        ],
        Some(&json!({"fileId":"old", "body":{"description":"Keep this", "name":"Old name"}})),
    )
    .unwrap();
    assert_eq!(call.params["fileId"], "document");
    assert_eq!(
        call.body.unwrap(),
        json!({"description":"Keep this", "name":"New name"})
    );
    let call = prepare(
        "drive",
        &[
            "permissions.create",
            "--file-id",
            "document",
            "--type",
            "user",
            "--role",
            "reader",
            "--email-address",
            "alice@example.com",
        ],
        None,
    )
    .unwrap();
    assert_eq!(
        call.body.unwrap(),
        json!({"type":"user", "role":"reader", "emailAddress":"alice@example.com"})
    );
    let call = prepare(
        "calendar",
        &["events.get", "--event-id", "event"],
        Some(&json!({"calendarId":"selected"})),
    )
    .unwrap();
    assert_eq!(call.params["calendarId"], "selected");
}

#[test]
fn invalid_requests_are_rejected_before_authentication() {
    for (service, args, file) in [
        ("drive", vec!["files.get"], json!({})),
        ("drive", vec!["files.get"], json!({"fileId":".."})),
        (
            "drive",
            vec!["files.get"],
            json!({"fileId":"file","access_token":"must-not-be-used"}),
        ),
        (
            "drive",
            vec!["files.get"],
            json!({"fileId":"file","body":{}}),
        ),
        (
            "drive",
            vec!["files.create"],
            json!({"body":{"inventedField":"no"}}),
        ),
        (
            "drive",
            vec!["files.create", "--body", "{}"],
            json!({"body":{"name":"Conflicting body"}}),
        ),
        (
            "youtube",
            vec!["playlists.insert"],
            json!({"body":{"snippet":{"title":"Example"},"status":{"privacyStatus":"INVALID"}}}),
        ),
        (
            "youtube",
            vec!["videos.update"],
            json!({"part":"snippet","body":{"snippet":{"title":"Missing video ID"}}}),
        ),
        (
            "photos",
            vec!["media-items.search"],
            json!({"body":{"pageSize":"wrong type"}}),
        ),
    ] {
        assert!(
            prepare(service, &args, Some(&file)).is_err(),
            "{service} {args:?}"
        );
    }
}

#[test]
fn canonical_names_and_global_option_positions_select_the_registered_command() {
    for args in [
        vec![
            "joom",
            "drive",
            "user@example.com",
            "files.get",
            "--file-id",
            "file",
        ],
        vec![
            "joom",
            "--home",
            "drive",
            "drive",
            "user@example.com",
            "--json",
            "files.get",
            "--file-id",
            "file",
        ],
    ] {
        let parsed = cli::parse_from(args).unwrap();
        let context = parsed.subcommand().unwrap().1;
        assert_eq!(context.subcommand_name(), Some("files.get"));
    }
    assert_eq!(
        cli::parse_from(["joom", "--home", "drive", "help", "drive", "files.get"])
            .unwrap_err()
            .kind(),
        clap::error::ErrorKind::DisplayHelp
    );
    assert!(
        cli::parse_from([
            "joom",
            "drive",
            "user@example.com",
            "files.get",
            "--file-id",
            "file",
            "--supports-all-drives",
            "--json"
        ])
        .is_err()
    );
    assert!(cli::parse_from(["joom", "drive", "user@example.com", "unknown.method"]).is_err());
}
#[test]
fn redundant_prefixes_deprecated_methods_and_alternate_spellings_are_absent() {
    for (service, canonical, old) in [
        ("mail", "messages.get", "users.messages.get"),
        ("sheets", "values.get", "spreadsheets.values.get"),
    ] {
        assert_eq!(
            cli::parse_from(["joom", "help", service, canonical])
                .unwrap_err()
                .kind(),
            clap::error::ErrorKind::DisplayHelp
        );
        assert_ne!(
            cli::parse_from(["joom", "help", service, old])
                .unwrap_err()
                .kind(),
            clap::error::ErrorKind::DisplayHelp
        );
        assert!(!contract(service).document.root.methods.contains_key(old));
    }
    for (service, method) in [
        ("drive", "teamdrives.create"),
        ("drive", "teamdrives.delete"),
        ("drive", "teamdrives.get"),
        ("drive", "teamdrives.list"),
        ("drive", "teamdrives.update"),
        ("photos", "albums.share"),
        ("photos", "albums.unshare"),
        ("photos", "sharedAlbums.get"),
        ("photos", "sharedAlbums.join"),
        ("photos", "sharedAlbums.leave"),
        ("photos", "sharedAlbums.list"),
        ("youtube", "comments.markAsSpam"),
    ] {
        if SERVICES.iter().any(|s| s.cli == service) {
            assert!(!contract(service).document.root.methods.contains_key(method));
        }
        assert_ne!(
            cli::parse_from(["joom", "help", service, method])
                .unwrap_err()
                .kind(),
            clap::error::ErrorKind::DisplayHelp
        );
    }
    for args in [
        ["joom", "drive", "user@example.com", "files", "get"],
        ["joom", "mail", "user@example.com", "messages", "get"],
    ] {
        assert!(cli::parse_from(args).is_err());
    }
    let mut methods = BTreeSet::new();
    for service in SERVICES {
        for method in contract(service.cli).document.root.methods.values() {
            assert!(!method.description.to_lowercase().starts_with("deprecated:"));
            assert!(
                methods.insert((&method.id, &method.batch_operation)),
                "duplicate method {}",
                method.id
            );
        }
    }
    assert!(cli::parse_from(["joom", "people", "user@example.com", "get"]).is_err());
    assert!(crate::services::scopes("people").is_err());
}

#[test]
fn selected_commands_use_kebab_case_and_removed_commands_have_no_aliases() {
    let removed: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/removed-commands.json"
    )))
    .unwrap();
    let mut cli = crate::cli::command();
    cli.build();
    for service in SERVICES {
        let namespace = cli.find_subcommand(service.cli).unwrap();
        for command in namespace.get_subcommands() {
            assert!(
                !command.get_name().chars().any(|ch| ch.is_ascii_uppercase()),
                "{} {}",
                service.cli,
                command.get_name()
            );
        }
        for old in removed[service.cli].as_array().unwrap() {
            assert!(
                namespace.find_subcommand(old.as_str().unwrap()).is_none(),
                "{} {old}",
                service.cli
            );
        }
    }
}

#[test]
fn every_service_write_example_builds_its_documented_request() {
    let examples: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/write-examples.json"
    )))
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for service in SERVICES {
        let name = examples[service.cli].as_str().unwrap();
        let file: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("examples/requests/{}-write.json", service.cli)))
                .unwrap(),
        )
        .unwrap();
        let call = prepare(service.cli, &[name], Some(&file))
            .unwrap_or_else(|error| panic!("{} {name}: {error:#}", service.cli));
        assert_eq!(call.method, name);
        assert!(call.body.is_some());
    }
}
#[test]
fn removed_microsoft_inputs_cannot_bypass_the_public_schema() {
    for (service, command, file) in [
        (
            "mail",
            "messages.send",
            json!({"to":["me@example.com"], "text":"Hello", "threadId":"thread"}),
        ),
        (
            "mail",
            "messages.modify",
            json!({"id":"message", "body":{"addLabelIds":["CUSTOM"]}}),
        ),
        (
            "drive",
            "files.update",
            json!({"fileId":"file", "body":{"trashed":false}}),
        ),
        (
            "calendar",
            "events.patch",
            json!({"eventId":"event", "body":{"recurrence":["RRULE:FREQ=DAILY"]}}),
        ),
        (
            "contacts",
            "people.create-contact",
            json!({"body":{"names":[{"givenName":"A","phoneticFullName":"unsupported"}]}}),
        ),
        (
            "contacts",
            "people.create-contact",
            json!({"body":{"names":[{"givenName":"A"},{"givenName":"B"}]}}),
        ),
        (
            "tasks",
            "tasks.insert",
            json!({"parent":"parent","title":"task"}),
        ),
        (
            "sheets",
            "values.update",
            json!({"spreadsheetId":"book", "range":"Sheet1!A1", "values":[[1]], "valueInputOption":"USER_ENTERED"}),
        ),
    ] {
        assert!(
            prepare(service, &[command], Some(&file)).is_err(),
            "accepted {service} {command}: {file}"
        );
    }
    for (service, command, flag) in [
        ("mail", "messages.get", "--format"),
        ("drive", "files.list", "--supports-all-drives"),
        ("calendar", "events.insert", "--recurrence"),
        ("tasks", "tasks.insert", "--parent"),
    ] {
        assert!(
            crate::cli::parse_from([
                "joom",
                service,
                "offline@example.com",
                command,
                flag,
                "invalid"
            ])
            .is_err()
        );
    }
    let call = prepare(
        "sheets",
        &["values.update"],
        Some(&json!({"spreadsheetId":"book","range":"Sheet1!A1:B1","values":[[1,true]]})),
    )
    .unwrap();
    assert_eq!(call.body.unwrap()["values"], json!([[1, true]]));
}

use std::collections::BTreeSet;

fn hidden_parameter(name: &str) -> bool {
    matches!(
        name,
        "fields"
            | "access_token"
            | "oauth_token"
            | "$.xgafv"
            | "alt"
            | "key"
            | "quotaUser"
            | "callback"
            | "prettyPrint"
            | "uploadType"
            | "upload_protocol"
            | "pageSize"
            | "maxResults"
    )
}

fn rest_bindings(document: &Document, method: &MethodSpec) -> Vec<Binding> {
    let reserved = [
        "file",
        "params",
        "body",
        "fields",
        "max",
        "max-pages",
        "ttl",
        "upload",
        "content-type",
        "output",
        "home",
        "timeout",
        "json",
        "help",
    ];
    let mut used = reserved
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut result = Vec::new();
    for (name, parameter) in &method.parameters {
        if hidden_parameter(name) {
            continue;
        }
        let mut long = kebab(name);
        if used.contains(&long) {
            long = format!("query-{long}");
        }
        assert!(used.insert(long.clone()), "duplicate query option");
        result.push(Binding {
            id: format!("q:{name}"),
            long,
            property: name.clone(),
            source: Source::Query(parameter.clone()),
            repeated: parameter.repeated,
        });
    }
    if let Some(request) = input_request(method) {
        let body = type_schema(document, request);
        for (name, schema) in body["properties"].as_object().into_iter().flatten() {
            let shape = type_schema(document, schema);
            if shape["readOnly"] == true {
                continue;
            }
            let mut long = kebab(name);
            if used.contains(&long) {
                long = format!("body-{long}");
            }
            assert!(used.insert(long.clone()), "duplicate body option");
            let items = type_schema(document, &shape["items"]);
            let repeated = shape["type"] == "array"
                && matches!(
                    items["type"].as_str(),
                    Some("string" | "integer" | "number" | "boolean")
                );
            result.push(Binding {
                id: format!("b:{name}"),
                long,
                property: name.clone(),
                source: Source::Body(schema.clone()),
                repeated,
            });
        }
    }
    for (name, schema) in &method.batch_fields {
        let mut long = kebab(name);
        if used.contains(&long) {
            long = format!("batch-{long}");
        }
        assert!(used.insert(long.clone()), "duplicate batch option");
        result.push(Binding {
            id: format!("e:{name}"),
            long,
            property: name.clone(),
            source: Source::Envelope(schema.clone()),
            repeated: false,
        });
    }
    result
}
