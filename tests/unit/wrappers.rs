use super::*;
use backend::Backend;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

#[derive(Default)]
struct Recording {
    calls: Vec<(String, String, Value)>,
    microsoft_checks: Vec<Value>,
    calendar_profile: Option<Value>,
}

impl Backend for Recording {
    fn call(&mut self, service: &str, command: &str, params: Value) -> Result<Value> {
        // Resolve through the actual Joom parser and API contract. This catches
        // wire names, required inputs, invalid enums, and incompatible bodies.
        let file = tempfile::NamedTempFile::new()?;
        serde_json::to_writer(file.as_file(), &params)?;
        let parsed = crate::cli::parse_from([
            "joom",
            service,
            "me@example.com",
            command,
            "--file",
            file.path().to_str().unwrap(),
        ])?;
        let context = parsed.subcommand().unwrap().1;
        let spec = crate::services::SERVICES
            .iter()
            .find(|spec| spec.cli == service)
            .unwrap();
        let call = crate::service_call(spec, context)
            .with_context(|| format!("{service} {command} {params}"))?;
        let document = &crate::schema_commands::contract(service).document;
        let method = document.method(&call.method)?;
        let options = call
            .options
            .as_ref()
            .unwrap_or_else(|| crate::native::arguments(context));
        let has = |name| options.try_get_one::<String>(name).ok().flatten().is_some();
        let result = crate::microsoft::validate(
            document,
            method,
            &call.params,
            call.body.as_ref(),
            has("upload") || (service == "drive" && command == "upload"),
            has("output"),
        );
        self.microsoft_checks.push(match &result {
            Ok(())=>json!({"service":service,"command":command,"status":"mapped"}),
            Err(error)=>json!({"service":service,"command":command,"status":"blocked","reason":format!("{error:#}")}),
        });
        result?;
        self.calls
            .push((service.into(), command.into(), params.clone()));
        Ok(match (service, command) {
            ("calendar", "calendars.get") => self
                .calendar_profile
                .clone()
                .unwrap_or_else(|| json!({"id":"primary","timeZone":"Europe/Paris"})),
            ("calendar", "events.get") => {
                json!({"id":"event","start":{"dateTime":"2026-12-01T10:00:00+01:00"},"end":{"dateTime":"2026-12-01T11:00:00+01:00"}})
            }
            ("drive", "files.get") => {
                json!({"id":"resource","mimeType":"application/octet-stream","parents":["old-folder"]})
            }
            ("drive", "files.copy") => json!({"id":"copied"}),
            ("sheets", "get") => {
                json!({"spreadsheetId":"sheet","sheets":[{"properties":{"sheetId":0,"title":"First tab"}}]})
            }
            ("sheets", "values.get") => json!({"values":[["Name","Value"],["Lara","42"]]}),
            ("contacts", "people.get") => {
                json!({"resourceName":"people/contact","etag":"etag","metadata":{"sources":[{"id":"contact","type":"CONTACT","etag":"source-etag"}]},"names":[{"displayName":"Lara"}],"phoneNumbers":[{"value":"123"}]})
            }
            ("mail", "messages.list") => json!({"messages":[{"id":"message","threadId":"thread"}]}),
            ("mail", "drafts.list") => {
                json!({"drafts":[{"id":"draft","message":{"id":"message","threadId":"thread"}}]})
            }
            ("mail", "drafts.get") => {
                json!({"id":"draft","message":{"id":"message","threadId":"thread","snippet":"Draft text","payload":{"headers":[{"name":"From","value":"me@example.com"},{"name":"Subject","value":"Draft subject"}]}}})
            }
            ("mail", "messages.get") => {
                json!({"id":"message","threadId":"thread","raw":URL_SAFE_NO_PAD.encode(b"From: \"Lara, Martin\" <lara@example.com>\r\nTo: me@example.com, alex@example.com\r\nCc: other@example.com\r\nSubject: Hello\r\nMessage-ID: <original@example.com>\r\nReferences: <older@example.com>\r\nContent-Type: text/plain\r\n\r\nOriginal body\r\n"),"payload":{"mimeType":"text/plain","headers":[{"name":"From","value":"lara@example.com"},{"name":"To","value":"me@example.com"},{"name":"Subject","value":"Hello"},{"name":"Date","value":"Wed, 25 Nov 2026 10:00:00 +0100"}],"body":{"data":URL_SAFE_NO_PAD.encode("Hello")}}})
            }
            _ => json!({"id":"resource"}),
        })
    }
}

fn sample(shape: &Value, dir: &Path) -> Value {
    if let Some(default) = shape.get("default") {
        return default.clone();
    }
    if let Some(value) = shape.get("const") {
        return value.clone();
    }
    if let Some(choices) = shape["enum"].as_array() {
        return choices[0].clone();
    }
    match shape["type"].as_str() {
        Some("array") => json!([sample(&shape["items"], dir)]),
        Some("boolean") => json!(true),
        Some("integer") => json!(shape["minimum"].as_u64().unwrap_or(1).max(1)),
        _ => json!(match shape["format"].as_str() {
            Some("email") => "lara@example.com".into(),
            Some("local-file") => dir.join("data.csv").to_str().unwrap().into(),
            Some("new-file") => dir.join("saved.csv").to_str().unwrap().into(),
            Some("local-date") => "2026-12-01".into(),
            Some("local-time") => "2026-12-01 10:00".into(),
            Some("duration") => "30m".into(),
            Some("date-time") => "2026-12-01T00:00:00Z".into(),
            Some("iana-time-zone") => "Europe/Paris".into(),
            Some("single-cell") => "B2".into(),
            Some("cell-range") => "A1:B2".into(),
            _ => "example".to_owned(),
        }),
    }
}

fn request(wrapper: &str, words: &[&str]) -> Result<Request> {
    let args = std::iter::once("me@example.com")
        .chain(words.iter().copied())
        .map(OsString::from)
        .collect::<Vec<_>>();
    match parse(wrapper, &args)? {
        Parsed::Request(request) => Ok(request),
        _ => bail!("unexpected help"),
    }
}

fn sample_parameters(schema: &Value, dir: &Path) -> Map<String, Value> {
    let mut params = Map::new();
    for name in schema["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        params.insert(name.into(), sample(&schema["properties"][name], dir));
    }
    for (name, shape) in schema["properties"].as_object().unwrap() {
        if let Some(default) = shape.get("default") {
            params
                .entry(name.clone())
                .or_insert_with(|| default.clone());
        }
    }
    for rule in schema["allOf"].as_array().into_iter().flatten() {
        if let Some(choices) = rule["anyOf"].as_array()
            && !assertion(rule, &json!(params))
        {
            for name in choices[0]["required"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                params.insert(name.into(), sample(&schema["properties"][name], dir));
            }
        }
    }
    params
}

#[test]
fn adversarial_files_reject_type_confusion_unknown_keys_and_missing_inputs_for_every_wrapper() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("data.csv"), "Lara,42\n").unwrap();
    let path = dir.path().join("parameters.json");
    let file = format!("file:{}", path.display());
    let mut checked = 0;
    let mut mutations = 0;
    for name in catalog()["$defs"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| name.contains('.'))
    {
        let (wrapper, command) = name.split_once('.').unwrap();
        let schema = definition(wrapper, command).unwrap();
        let valid = sample_parameters(&schema, dir.path());
        let parse_file = |params: &Map<String, Value>| {
            std::fs::write(&path, serde_json::to_vec(params).unwrap()).unwrap();
            request(wrapper, &[command, &file])
        };
        assert!(parse_file(&valid).is_ok(), "invalid baseline for {name}");
        for key in schema["properties"].as_object().unwrap().keys() {
            let mut invalid = valid.clone();
            invalid.insert(key.clone(), json!({"injected":"wrong type"}));
            assert!(
                parse_file(&invalid).is_err(),
                "{name} accepted an object for {key}"
            );
            mutations += 1;
        }
        for key in ["__proto__", "constructor", "unknownParameter"] {
            let mut invalid = valid.clone();
            invalid.insert(key.into(), json!({"injected":true}));
            assert!(parse_file(&invalid).is_err(), "{name} accepted {key}");
            mutations += 1;
        }
        for key in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            let mut invalid = valid.clone();
            invalid.remove(key);
            assert!(
                parse_file(&invalid).is_err(),
                "{name} accepted missing {key}"
            );
            mutations += 1;
        }
        checked += 1;
    }
    assert_eq!(checked, 53);
    assert!(mutations > 500);
}

#[test]
fn adversarial_pagination_rejects_broken_json_and_incompatible_response_shapes() {
    assert_eq!(
        backend::merge_pages(
            br#"{"items":[{"id":"a"}],"nextPageToken":"one"}
{"items":[{"id":"b"}]}"#
        )
        .unwrap(),
        json!({"items":[{"id":"a"},{"id":"b"}]})
    );
    for bytes in [
        &b"{\"items\":["[..],
        &b"{}\n[]"[..],
        &b"[]\n{}"[..],
        &b"{\"items\":[]}\n{\"items\":\"invalid\"}"[..],
        &b"{\"items\":\"invalid\"}\n{\"items\":[]}"[..],
    ] {
        assert!(backend::merge_pages(bytes).is_err());
    }
}

#[test]
fn adversarial_limits_dates_headers_and_resource_urls_fail_without_a_backend() {
    for value in [
        "0m",
        "-1m",
        "18446744073709551615h",
        "9223372036854775807d",
        "NaNh",
        "1e9m",
    ] {
        assert!(
            values::duration(value).is_err(),
            "accepted duration {value}"
        );
    }
    for (kind, value) in [
        ("local-date", "2026-02-30"),
        ("local-date", "2026-13-01"),
        ("local-time", "tomorrow 25:61"),
        ("date-time", "2026-12-01T10:00:00"),
        ("single-cell", "A0"),
        ("single-cell", "A18446744073709551616"),
        ("cell-range", "A1:B2:C3"),
        ("email", "user@example.com\r\nBcc:other@example.com"),
        ("email", "Name <user@example.com>"),
        ("single-line", "Subject\nInjected: value"),
    ] {
        assert!(values::format(kind, value).is_err(), "accepted {kind}");
    }
    for kind in ["drive", "docs", "sheets", "slides", "forms"] {
        for value in [
            "https://onedrive.live.com.attacker.example/?id=secret",
            "https://attacker.example/?id=secret",
            "https://user:password@onedrive.live.com/?id=secret",
            "http://onedrive.live.com/?id=secret",
            "https://onedrive.live.com:444/?id=secret",
            "https://tenant.sharepoint.com/:w:/r/%2e%2e%2fsecret",
        ] {
            assert!(
                values::resource(kind, value).is_err(),
                "accepted untrusted {kind} URL"
            );
        }
    }
    for (wrapper, args) in [
        (
            "jmmail",
            vec![
                "draft",
                "to:user@example.com",
                "subject:Hello\r\nBcc:other@example.com",
            ],
        ),
        (
            "jmcal",
            vec![
                "add",
                "Test",
                "at:tomorrow 10:00",
                "for:18446744073709551615d",
            ],
        ),
        ("jmdrive", vec!["ls", "limit:18446744073709551615"]),
        ("jmphotos", vec!["ls", "limit:-1"]),
    ] {
        assert!(
            request(wrapper, &args).is_err(),
            "accepted adversarial {wrapper} request"
        );
    }
}

#[test]
fn every_declared_wrapper_command_builds_requests_accepted_by_joom() {
    let mut count = 0;
    let mut failures = Vec::new();
    let mut provider_report = Vec::new();
    for (name, _) in catalog()["$defs"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(name, _)| name.contains('.'))
    {
        let (wrapper, command) = name.split_once('.').unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("data.csv"), "Lara,42\nAlex,43\n").unwrap();
        let schema = definition(wrapper, command).unwrap();
        let mut params = Map::new();
        for name in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            params.insert(name.into(), sample(&schema["properties"][name], dir.path()));
        }
        for (name, shape) in schema["properties"].as_object().unwrap() {
            if let Some(default) = shape.get("default") {
                params
                    .entry(name.clone())
                    .or_insert_with(|| default.clone());
            }
        }
        for rule in schema["allOf"].as_array().into_iter().flatten() {
            if let Some(choices) = rule["anyOf"].as_array()
                && !assertion(rule, &json!(params))
            {
                for name in choices[0]["required"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    params.insert(name.into(), sample(&schema["properties"][name], dir.path()));
                }
            }
        }
        // Exercise actual row parsing and numeric category IDs rather than an
        // unrelated category-list fixture.
        if wrapper == "jmsheets" && command == "append" {
            params.insert("row".into(), json!(["Lara,42"]));
        }
        if wrapper == "jmyt" && command == "upload" {
            params.insert("category".into(), json!("27"));
        }
        let file = dir.path().join("request.json");
        std::fs::write(&file, serde_json::to_vec(&params).unwrap()).unwrap();
        let mut request = request(wrapper, &[command, &format!("file:{}", file.display())])
            .unwrap_or_else(|error| panic!("{name}: {error:#}"));
        let mut api = Recording::default();
        match tasks::execute(&request, &mut api) {
            Ok(value) => {
                assert!(!api.calls.is_empty(), "{name} made no requests");
                // The same provider responses must yield identical data in
                // both output modes, including for compound commands.
                if let Some(path) = request.text("out")
                    && Path::new(path).exists()
                {
                    std::fs::remove_file(path).unwrap();
                }
                request.params.insert("json".into(), json!(true));
                let mut json_api = Recording::default();
                let json_value = tasks::execute(&request, &mut json_api)
                    .unwrap_or_else(|error| panic!("{name} JSON: {error:#}"));
                assert_eq!(
                    value, json_value,
                    "{name}: output format changed the result data"
                );
                assert_eq!(
                    api.calls
                        .iter()
                        .map(|(service, command, _)| (service, command))
                        .collect::<Vec<_>>(),
                    json_api
                        .calls
                        .iter()
                        .map(|(service, command, _)| (service, command))
                        .collect::<Vec<_>>(),
                    "{name}: output format changed the operations performed"
                );
            }
            Err(error) => failures.push(format!("{name}: {error:#}")),
        }
        provider_report.push(json!({"wrapper":wrapper,"command":command,"status":if api.microsoft_checks.iter().all(|check|check["status"]=="mapped") {"mapped"} else {"blocked"},"checks":api.microsoft_checks,"verification":"local request planning with sample input; not live"}));
        count += 1;
    }
    assert_eq!(count, 53);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    let expected: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/data/microsoft-wrapper-support.json"
        )))
        .unwrap();
    let actual = provider_report
        .iter()
        .map(|row| {
            (
                format!(
                    "{}.{}",
                    row["wrapper"].as_str().unwrap(),
                    row["command"].as_str().unwrap()
                ),
                row["status"].as_str().unwrap().to_owned(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(
        actual, expected,
        "Microsoft wrapper support changed; inspect the provider checks before updating the baseline"
    );
    if let Ok(directory) = std::env::var("JOOM_AUDIT_DIRECTORY") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            Path::new(&directory).join("wrappers.json"),
            serde_json::to_vec_pretty(&provider_report).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn mail_formats_share_message_metadata_identifiers_and_requests() {
    for words in [
        vec!["inbox", "limit:10", "unread:true"],
        vec!["find", "Lara"],
        vec!["read", "message"],
        vec!["drafts"],
    ] {
        let mut query = request("jmmail", &words).unwrap();
        let mut plain_api = Recording::default();
        let plain = tasks::execute(&query, &mut plain_api).unwrap();
        query.params.insert("json".into(), json!(true));
        let mut json_api = Recording::default();
        let json = tasks::execute(&query, &mut json_api).unwrap();
        assert_eq!(plain, json);
        assert_eq!(plain_api.calls, json_api.calls);
        let row = if plain.is_array() { &plain[0] } else { &plain };
        assert_eq!(row["id"], "message");
        assert_eq!(row["threadId"], "thread");
        assert!(row["from"].is_string());
        assert!(row["subject"].is_string());
        assert!(row["text"].is_string());
        if words[0] == "drafts" {
            assert_eq!(row["draftId"], "draft");
        }
        if words[0] == "inbox" {
            assert_eq!(plain_api.calls[0].2["q"], "in:inbox is:unread");
            assert_eq!(plain_api.calls[0].2["max"], 10);
        }
    }
}

#[test]
fn wrappers_have_their_own_syntax_and_defaults() {
    assert!(executable(std::ffi::OsStr::new("jkeep")).is_none());
    assert_eq!(request("jmmail", &[]).unwrap().command, "inbox");
    let sent = request(
        "jmmail",
        &[
            "to:lara@example.com",
            "to:alex@example.com",
            r"body:Hello\nHow are you?",
        ],
    )
    .unwrap();
    assert_eq!(sent.command, "send");
    assert_eq!(
        sent.params["to"],
        json!(["lara@example.com", "alex@example.com"])
    );
    assert_eq!(sent.text("body"), Some("Hello\nHow are you?"));
    assert!(request("jmyt", &[]).is_err());
    assert!(
        crate::cli::parse_from([
            "joom",
            "mail",
            "me@example.com",
            "send",
            "to:lara@example.com"
        ])
        .is_err()
    );
    assert!(
        crate::cli::parse_from([
            "joom",
            "mail",
            "me@example.com",
            "messages.send",
            "--to",
            "lara@example.com"
        ])
        .is_ok()
    );
    for wrapper in catalog()["x-wrappers"].as_object().unwrap().keys() {
        assert!(matches!(
            parse(wrapper, &["--help".into()]).unwrap(),
            Parsed::Help(_)
        ));
    }
    let common = help_text("jmmail", Some("send"), false).unwrap();
    assert!(!common.contains("home:PATH"));
    assert!(
        help_text("jmmail", Some("send"), true)
            .unwrap()
            .contains("home:PATH")
    );
    assert!(common.contains("Provide at least one recipient"));
}

#[test]
fn schema_constraints_reject_invalid_wrapper_inputs_before_execution() {
    for (wrapper, words) in [
        ("jmmail", vec!["send", "body:Hello"]),
        ("jmmail", vec!["send", "to:not-an-email"]),
        ("jmmail", vec!["send", "to:lara@example.com", "unknown:yes"]),
        ("jmdrive", vec!["move", "file"]),
        (
            "jmdrive",
            vec!["share", "file", "to:lara@example.com", "role:owner"],
        ),
        (
            "jmcal",
            vec!["add", "Lunch", "at:2026-12-01 10:00", "on:2026-12-01"],
        ),
        (
            "jmcal",
            vec!["add", "Lunch", "at:2026-12-01 10:00", "for:0m"],
        ),
        ("jmsheets", vec!["clear", "sheet"]),
        ("jmsheets", vec!["set", "sheet", "cell:A1:B2", "value:x"]),
        ("jmdocs", vec!["append", "doc"]),
        ("jmslides", vec!["set", "deck", "slide:0", "title:Hello"]),
        ("jmforms", vec!["question", "form", "Title", "type:choice"]),
        (
            "jmforms",
            vec!["question", "form", "Title", "type:text", "option:wrong"],
        ),
        ("jmtasks", vec!["add", "Task", "due:2026-02-30"]),
        ("jmcontacts", vec!["set", "contact"]),
        ("jmyt", vec!["new-playlist", "Title", "privacy:friends"]),
        ("jmphotos", vec!["find", "album:unsupported"]),
    ] {
        assert!(
            request(wrapper, &words).is_err(),
            "accepted {wrapper} {words:?}"
        );
    }
}

#[test]
fn file_overrides_arrays_text_representations_and_unknown_fields() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("input.json");
    std::fs::write(
        &file,
        r#"{"to":["old@example.com"],"bodyFile":"must-not-be-read.txt"}"#,
    )
    .unwrap();
    let parsed = request(
        "jmmail",
        &[
            "send",
            &format!("file:{}", file.display()),
            "to:lara@example.com",
            "body:New",
        ],
    )
    .unwrap();
    assert_eq!(parsed.strings("to"), ["lara@example.com"]);
    assert!(parsed.text("bodyFile").is_none());
    std::fs::write(&file, r#"{"to":["lara@example.com"],"unknown":true}"#).unwrap();
    assert!(request("jmmail", &["send", &format!("file:{}", file.display())]).is_err());
}

#[test]
fn contact_updates_only_replace_named_fields_and_keep_concurrency_metadata() {
    let mut api = Recording::default();
    tasks::execute(
        &request("jmcontacts", &["set", "contact", "email:new@example.com"]).unwrap(),
        &mut api,
    )
    .unwrap();
    let params = &api.calls.last().unwrap().2;
    assert!(params.get("updatePersonFields").is_none());
    assert!(params["body"].get("metadata").is_none());
    assert_eq!(
        params["body"]["emailAddresses"][0]["value"],
        "new@example.com"
    );
    assert!(params["body"].get("phoneNumbers").is_none());
    assert_eq!(params.pointer("/body/etag"), Some(&json!("etag")));
}

#[test]
fn csv_exports_preserve_ragged_rows_and_literal_formulas() {
    let data = vec![
        vec!["one".into(), "quoted, cell".into(), "=1+1".into()],
        vec!["Unicode café".into()],
        vec![],
    ];
    let bytes = values::csv_bytes(&data).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "one,\"quoted, cell\",=1+1\nUnicode café,,\n,,\n"
    );
}

#[test]
fn csv_quotes_timezones_and_atomic_output_are_preserved() {
    assert!(values::duration("1é").is_err());
    let rows = values::csv_rows(
        &json!({"row":["\"Lara, Martin\",42","Alex,43"]})
            .as_object()
            .unwrap()
            .clone(),
    )
    .unwrap();
    assert_eq!(rows[0], ["Lara, Martin", "42"]);
    let now = "2026-03-28T12:00:00Z".parse::<jiff::Timestamp>().unwrap();
    let zone = jiff::tz::TimeZone::get("Europe/Paris").unwrap();
    assert!(values::timestamp("2026-03-29 02:30", &zone, now).is_err());
    assert_eq!(
        values::rfc3339(
            values::timestamp("2026-03-29T03:30:00+02:00", &zone, now).unwrap(),
            &zone
        ),
        "2026-03-29T03:30:00+02:00"
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("saved.csv");
    values::save(b"original", path.to_str().unwrap()).unwrap();
    assert!(values::save(b"replacement", path.to_str().unwrap()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"original");
    assert_eq!(
        backend::merge_pages(b"{\"items\":[1],\"nextPageToken\":\"next\"}\n{\"items\":[2]}")
            .unwrap(),
        json!({"items":[1,2]})
    );
}

#[test]
fn microsoft_urls_and_literal_text_are_normalized_without_accepting_other_hosts() {
    assert_eq!(
        values::resource("docs", "https://onedrive.live.com/?resid=abc123").unwrap(),
        "abc123"
    );
    assert_eq!(
        values::resource(
            "drive",
            "https://graph.microsoft.com/v1.0/me/drive/items/ABCD%21123"
        )
        .unwrap(),
        "ABCD!123"
    );
    assert_eq!(values::resource("drive", "ABCD!123").unwrap(), "ABCD!123");
    assert_eq!(values::resource("mail", "AAMkAg==").unwrap(), "AAMkAg==");
    assert!(values::resource("docs", "https://onedrive.live.com.evil.example/?id=abc123").is_err());
    assert!(values::resource("forms", "https://example.com/untrusted-document").is_err());
    assert_eq!(values::unescape(r"a\\nb\nc"), "a\\nb\nc");
}

#[test]
fn schema_constraints_are_supported_and_explained_by_help() {
    assert!(
        help_text("jmcal", Some("cancel"), false)
            .unwrap()
            .contains("Choices: true.")
    );
    fn assertion_keywords(rule: &Value) {
        for (key, value) in rule.as_object().unwrap() {
            match key.as_str() {
                "required" | "const" | "enum" | "description" => {}
                "properties" => value
                    .as_object()
                    .unwrap()
                    .values()
                    .for_each(assertion_keywords),
                "allOf" | "anyOf" => value
                    .as_array()
                    .unwrap()
                    .iter()
                    .for_each(assertion_keywords),
                "if" | "then" | "else" | "not" => assertion_keywords(value),
                key => panic!("unsupported wrapper assertion {key}"),
            }
        }
    }
    for name in catalog()["$defs"]
        .as_object()
        .unwrap()
        .keys()
        .filter(|name| name.contains('.'))
    {
        let (wrapper, command) = name.split_once('.').unwrap();
        let schema = definition(wrapper, command).unwrap();
        let help = help_text(wrapper, Some(command), true).unwrap();
        for rule in schema["allOf"].as_array().into_iter().flatten() {
            if let Some(description) = rule["description"].as_str() {
                assertion_keywords(rule);
                assert!(help.contains(description), "{name}: {description}");
            }
        }
        for (name, shape) in schema["properties"].as_object().unwrap() {
            assert!(help.contains(&format!("{}:", key(name, shape))));
            assert!(
                shape["description"]
                    .as_str()
                    .is_some_and(|text| !text.is_empty()),
                "{name} has no help description"
            );
        }
    }
    for invalid in ["A0", "A-1", "1A", "A1:B2:C3", "$$A1"] {
        assert!(
            values::format("cell-range", invalid).is_err(),
            "accepted {invalid}"
        );
    }
    for valid in ["A1", "$A$1", "A1:B20", "A:Z", "1:10"] {
        values::format("cell-range", valid).unwrap();
    }
}

#[test]
fn relative_calendar_bounds_are_checked_in_the_selected_timezone() {
    let bounds = request(
        "jmcal",
        &[
            "find",
            "Meeting",
            "from:tomorrow",
            "until:today",
            "timezone:Pacific/Honolulu",
        ],
    )
    .unwrap();
    let mut api = Recording::default();
    let error = tasks::execute(&bounds, &mut api).unwrap_err();
    assert!(error.to_string().contains("calendar timezone"));
    assert!(api.calls.is_empty());
    assert_eq!(
        request("jmcal", &["add", "Lunch", "at:tomorrow 12:00"])
            .unwrap()
            .text("for"),
        Some("30m")
    );
    assert!(
        request("jmcal", &["add", "Holiday", "on:tomorrow"])
            .unwrap()
            .text("for")
            .is_none()
    );
}

#[test]
fn calendar_without_a_mailbox_timezone_uses_the_system_timezone() {
    let local = jiff::tz::TimeZone::try_system().unwrap();
    for profile in [
        json!({"id":"primary"}),
        json!({"timeZone":null}),
        json!({"timeZone":""}),
    ] {
        let mut api = Recording {
            calendar_profile: Some(profile),
            ..Recording::default()
        };
        tasks::execute(&request("jmcal", &[]).unwrap(), &mut api).unwrap();
        let (_, _, params) = api
            .calls
            .iter()
            .find(|(_, command, _)| command == "events.list")
            .unwrap();
        let start = params["timeMin"]
            .as_str()
            .unwrap()
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned_datetime(local.clone());
        let end = params["timeMax"]
            .as_str()
            .unwrap()
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned_datetime(local.clone());
        assert_eq!(start.time(), jiff::civil::Time::midnight());
        assert_eq!(end.time(), jiff::civil::Time::midnight());
        assert_eq!(end.date(), start.date().tomorrow().unwrap());
    }
    for explicit in [false, true] {
        let mut api = Recording {
            calendar_profile: Some(json!({"timeZone":"Pacific/Honolulu"})),
            ..Recording::default()
        };
        let words = if explicit {
            vec!["today", "timezone:Asia/Tokyo"]
        } else {
            vec!["today"]
        };
        tasks::execute(&request("jmcal", &words).unwrap(), &mut api).unwrap();
        assert_eq!(api.calls.len(), if explicit { 1 } else { 2 });
        let zone = jiff::tz::TimeZone::get(if explicit {
            "Asia/Tokyo"
        } else {
            "Pacific/Honolulu"
        })
        .unwrap();
        let params = &api.calls.last().unwrap().2;
        let start = params["timeMin"]
            .as_str()
            .unwrap()
            .parse::<jiff::Timestamp>()
            .unwrap()
            .to_zoned_datetime(zone);
        assert_eq!(start.time(), jiff::civil::Time::midnight());
    }
}

#[test]
fn optional_parameters_build_complete_compound_requests() {
    let cases = [
        ("jmdrive", vec!["copy", "file", "to:folder", "name:Copy"]),
        (
            "jmdrive",
            vec![
                "share",
                "file",
                "to:lara@example.com",
                "role:writer",
                "notify:false",
            ],
        ),
        (
            "jmsheets",
            vec!["read", "sheet", "tab:January", "range:A1:D20"],
        ),
        (
            "jmdocs",
            vec![
                "share",
                "doc",
                "to:lara@example.com",
                "role:reader",
                "notify:false",
            ],
        ),
        (
            "jmtasks",
            vec![
                "set",
                "task",
                "title:Updated",
                "notes:Notes",
                "due:tomorrow",
                "list:list",
            ],
        ),
        (
            "jmcontacts",
            vec![
                "new",
                "Lara Martin",
                "email:lara@example.com",
                "phone:+33123456789",
                "company:Company",
            ],
        ),
        (
            "jmcontacts",
            vec!["set", "contact", "phone:+33123456789", "company:Company"],
        ),
    ];
    let mut failures = Vec::new();
    for (wrapper, words) in cases {
        let request = request(wrapper, &words).unwrap();
        if let Err(error) = tasks::execute(&request, &mut Recording::default()) {
            failures.push(format!("{wrapper} {words:?}: {error:#}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn onedrive_upload_limit_is_checked_before_execution() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.bin");
    let file = std::fs::File::create(&path).unwrap();
    file.set_len(250 * 1024 * 1024).unwrap();
    assert!(request("jmdrive", &["put", path.to_str().unwrap()]).is_ok());
    file.set_len(250 * 1024 * 1024 + 1).unwrap();
    let error = request("jmdrive", &["put", path.to_str().unwrap()]).unwrap_err();
    assert!(error.to_string().contains("maximum file size"));
}
