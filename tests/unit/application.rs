use super::*;

fn prepare(args: &[&str]) -> (String, services::Invocation) {
    let parsed = cli::parse_from(args.iter().copied()).unwrap();
    let (name, context) = parsed.subcommand().unwrap();
    let service = services::SERVICES.iter().find(|s| s.cli == name).unwrap();
    (
        string(context, "account").unwrap().to_owned(),
        service_call(service, context).unwrap(),
    )
}

#[test]
fn service_actions_preserve_parameters_and_build_write_requests_directly() {
    let (account, list) = prepare(&["joom", "drive", "reader@example.com", "files.list"]);
    assert_eq!(account, "reader@example.com");
    assert_eq!(list.method, "files.list");
    assert!(cli::parse_from(["joom", "drive", "writer@example.com", "restore", "AbC123"]).is_err());
    let (_, raw) = prepare(&[
        "joom",
        "drive",
        "writer@example.com",
        "files.create",
        "--body",
        "{\"name\":\"Example\"}",
    ]);
    assert_eq!(raw.method, "files.create");
    assert_eq!(raw.body.unwrap()["name"], "Example");
}

#[test]
fn every_service_has_a_schema_and_an_example_accepted_by_request_validation() {
    let fixtures: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/file-parameters.json"
    )))
    .unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(
        fixtures.as_object().unwrap().len(),
        services::SERVICES.len()
    );
    for service in services::SERVICES {
        let schema: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("schemas/{}.schema.json", service.cli))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            schema["title"],
            format!("Joom {} command parameters", service.cli)
        );
        assert!(schema.get("x-api-methods").is_none());
        assert!(schema.get("x-body-types").is_none());
        let definitions = schema["$defs"].as_object().unwrap();
        assert!(!definitions.is_empty());
        assert_eq!(schema["anyOf"].as_array().unwrap().len(), definitions.len());
        for definition in definitions.values() {
            assert_eq!(definition["additionalProperties"], false);
            let properties = definition["properties"].as_object().unwrap();
            assert!(!properties.contains_key("access_token"));
            assert!(!properties.contains_key("oauth_token"));
        }
        let fixture = &fixtures[service.cli];
        let method = fixture["method"].as_str().unwrap();
        assert!(definitions.contains_key(method));
        let path = root.join(format!("examples/requests/{}.json", service.cli));
        let (_, call) = prepare(&[
            "joom",
            service.cli,
            "reader@example.com",
            method,
            "--file",
            path.to_str().unwrap(),
        ]);
        let document = &schema_commands::contract(service.cli).document;
        let method_id = &schema_commands::contract(service.cli)
            .document
            .method(&call.method)
            .unwrap()
            .id;
        let specification = document.method(method_id).unwrap();
        let url = document
            .url(specification, &call.params)
            .unwrap_or_else(|error| panic!("{} example: {error}", service.cli));
        http::contract_url(&url).unwrap();
    }
}

#[test]
fn every_documented_request_example_resolves_to_microsoft() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/requests");
    let index = std::fs::read_to_string(directory.join("README.md")).unwrap();
    let mut documented = std::collections::BTreeSet::new();
    for line in index.lines().filter(|line| line.starts_with("| [")) {
        let columns = line.split('|').collect::<Vec<_>>();
        let filename = columns[1]
            .split_once("](")
            .unwrap()
            .1
            .strip_suffix(") ")
            .unwrap();
        assert!(
            documented.insert(filename.to_owned()),
            "duplicate {filename}"
        );
        let path = directory.join(filename);
        let mut args = columns[2]
            .trim()
            .trim_matches('`')
            .split_whitespace()
            .collect::<Vec<_>>();
        args[2] = "reader@example.com";
        args.extend(["--file", path.to_str().unwrap()]);
        let (_, call) = prepare(&args);
        let contract = schema_commands::contract(args[1]);
        let method = contract.document.method(&call.method).unwrap();
        microsoft::validate(
            &contract.document,
            method,
            &call.params,
            call.body.as_ref(),
            false,
            false,
        )
        .unwrap_or_else(|error| panic!("{filename}: {error:#}"));
    }
    let files = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .map(|path| path.file_name().unwrap().to_str().unwrap().to_owned())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(!files.is_empty());
    assert_eq!(
        documented, files,
        "every request example must be documented"
    );
}

#[test]
fn request_parameters_load_from_a_direct_file_path_and_body_stays_inline() {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), br#"{"fileId":"AbC123","fields":"id,name"}"#).unwrap();
    let (_, call) = prepare(&[
        "joom",
        "drive",
        "reader@example.com",
        "files.update",
        "--file",
        file.path().to_str().unwrap(),
        "--body",
        r#"{"name":"Updated"}"#,
    ]);
    assert_eq!(call.method, "files.update");
    assert_eq!(
        call.params,
        json!({"fileId":"AbC123","fields":"id,name"})
            .as_object()
            .unwrap()
            .clone()
    );
    assert_eq!(call.body.unwrap(), json!({"name":"Updated"}));
    assert!(read_json("@-").is_err());
    assert!(read_json(&format!("@{}", file.path().display())).is_err());
}

#[test]
fn json_preserves_large_sheet_numbers() {
    let text = "{\"values\":[[18446744073709551616001]]}";
    assert_eq!(super::read_json(text).unwrap().to_string(), text);
}
