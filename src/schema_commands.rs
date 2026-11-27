use anyhow::{Context, Result, ensure};
use clap::{Arg, ArgAction, ArgGroup, ArgMatches, Command};
use serde_json::{Map, Value, json};
use std::{collections::BTreeMap, sync::OnceLock};

use crate::{
    discovery::{Document, MethodSpec, Parameter, Resource},
    http, native, pagination,
    services::Invocation,
};

pub struct Contract {
    pub document: Document,
    pub compose_paths: BTreeMap<String, Vec<String>>,
    pub compose_schema: Value,
    pub inputs: BTreeMap<String, Value>,
}
macro_rules! sources {
    ($($name:literal;)*) => {
        static SOURCES:[(&str,&str,&str,OnceLock<Contract>);crate::services::SERVICES.len()]=[$(($name,include_str!(concat!("../api/",$name,".json")),include_str!(concat!("../schemas/",$name,".schema.json")),OnceLock::new())),*];
    }
}
sources! {"mail";"drive";"calendar";"sheets";"tasks";"contacts";}

pub fn contract(service: &str) -> &'static Contract {
    let (_, source, inputs, cache) = SOURCES
        .iter()
        .find(|(name, _, _, _)| *name == service)
        .expect("known service");
    cache.get_or_init(|| {
        let public: Value = serde_json::from_str(inputs).expect("checked CLI input schema");
        let inputs: BTreeMap<String, Value> =
            serde_json::from_value(public["$defs"].clone()).expect("checked CLI commands");
        let mut root: Value = serde_json::from_str(source).expect("checked bundled schema");
        let compose_paths = root["$defs"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(name, definition)| {
                definition.get("x-cli-compose-path").map(|path| {
                    (
                        name.clone(),
                        serde_json::from_value(path.clone()).expect("checked compose path"),
                    )
                })
            })
            .collect();
        let mut compose_schema = public["x-input-types"]["MailComposition"].clone();
        if let Some(properties) = compose_schema.get_mut("properties") {
            *properties = Value::Object(
                inputs["messages.send"]["properties"]
                    .as_object()
                    .expect("checked mail inputs")
                    .values()
                    .filter(|shape| shape["x-cli-binding"]["source"] == "compose")
                    .map(|shape| {
                        (
                            shape["x-cli-binding"]["property"]
                                .as_str()
                                .unwrap()
                                .to_owned(),
                            shape.clone(),
                        )
                    })
                    .collect(),
            );
        }
        let catalog: BTreeMap<String, MethodSpec> =
            serde_json::from_value(root["x-api-methods"].take()).expect("checked method catalog");
        let methods = root["$defs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, definition)| {
                let id = definition["x-api-method-id"]
                    .as_str()
                    .expect("checked method ID");
                let mut method = catalog.get(id).expect("checked catalog reference").clone();
                method.cli_inputs = serde_json::from_value(inputs[name]["properties"].clone())
                    .expect("checked CLI inputs");
                method.description = inputs[name]["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned();
                for (key, shape) in &method.cli_inputs {
                    if shape["x-cli-binding"]["source"] == "query" {
                        let property = shape["x-cli-binding"]["property"].as_str().unwrap();
                        if let Some(parameter) = method.parameters.get_mut(property) {
                            parameter.description =
                                shape["description"].as_str().unwrap_or_default().to_owned();
                            parameter.required =
                                inputs[name]["allOf"].as_array().into_iter().flatten().any(
                                    |rule| {
                                        rule["x-cli-kind"] == "required"
                                            && rule["x-cli-parameters"].as_array().is_some_and(
                                                |keys| keys.iter().any(|item| item == key),
                                            )
                                    },
                                );
                        }
                    }
                }
                if let Some(operation) = definition["x-batch-operation"].as_str() {
                    method.batch_operation = Some(operation.into());
                    method.operation_request = Some(inputs[name]["properties"]["body"].clone());
                    method.description = definition["properties"]["body"]["description"]
                        .as_str()
                        .expect("checked operation description")
                        .into();
                    method.batch_fields = definition["properties"]
                        .as_object()
                        .unwrap()
                        .iter()
                        .filter(|(key, _)| {
                            !matches!(key.as_str(), "body" | "fields")
                                && !method.parameters.contains_key(*key)
                        })
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect();
                }
                (name.clone(), method)
            })
            .collect();
        let document = Document {
            name: root["x-api-service"].as_str().unwrap().into(),
            version: root["x-api-version"].as_str().unwrap().into(),
            title: root["title"].as_str().unwrap_or_default().into(),
            root_url: root["x-api-root"].as_str().unwrap().into(),
            service_path: root["x-api-service-path"]
                .as_str()
                .unwrap_or_default()
                .into(),
            base_url: String::new(),
            parameters: serde_json::from_value(root["x-api-parameters"].take()).unwrap(),
            schemas: serde_json::from_value(
                public
                    .get("x-input-types")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            )
            .unwrap(),
            root: Resource {
                methods,
                resources: BTreeMap::new(),
            },
        };
        let service = crate::services::SERVICES
            .iter()
            .find(|item| item.cli == service)
            .unwrap();
        assert_eq!(document.name, service.api);
        assert_eq!(document.version, service.version);
        Contract {
            document,
            compose_paths,
            compose_schema,
            inputs,
        }
    })
}

pub fn kebab(name: &str) -> String {
    let mut result = String::new();
    let chars = name.chars().collect::<Vec<_>>();
    for (index, &ch) in chars.iter().enumerate() {
        if ch.is_ascii_uppercase() {
            if index > 0
                && (chars[index - 1].is_ascii_lowercase()
                    || chars[index - 1].is_ascii_digit()
                    || (chars[index - 1].is_ascii_uppercase()
                        && chars.get(index + 1).is_some_and(char::is_ascii_lowercase)))
            {
                result.push('-');
            }
            result.push(ch.to_ascii_lowercase());
        } else {
            result.push(if matches!(ch, '_' | '.') { '-' } else { ch });
        }
    }
    result
}

fn type_schema<'a>(document: &'a Document, schema: &'a Value) -> &'a Value {
    let mut schema = schema;
    for _ in 0..64 {
        if let Some(choices) = schema["anyOf"].as_array() {
            schema = choices
                .iter()
                .find(|choice| choice["type"] != "null")
                .unwrap_or(schema);
        } else if let Some(reference) = schema["$ref"].as_str() {
            let name = reference
                .strip_prefix("#/x-body-types/")
                .or_else(|| reference.strip_prefix("#/x-input-types/"))
                .unwrap_or(reference);
            schema = document
                .schemas
                .get(name)
                .expect("checked schema reference");
        } else {
            return schema;
        }
    }
    schema
}
#[derive(Clone)]
enum Source {
    Query(Parameter),
    Body(Value),
    Envelope(Value),
    Nested(Value, Vec<String>),
}
#[derive(Clone)]
struct Binding {
    id: String,
    long: String,
    property: String,
    source: Source,
    repeated: bool,
}

fn required_for(schema: &Value, method: &str) -> bool {
    schema["x-required-in-methods"]
        .as_array()
        .is_some_and(|methods| methods.iter().any(|id| id == method))
        || schema["anyOf"]
            .as_array()
            .is_some_and(|choices| choices.iter().any(|schema| required_for(schema, method)))
}

fn input_request(method: &MethodSpec) -> Option<&Value> {
    method
        .operation_request
        .as_ref()
        .or(method.request.as_ref())
}

fn required_input_field(schema: &Value, key: &str, property: &Value, method: &str) -> bool {
    required_for(property, method)
        || ["required", "x-required-fields"].iter().any(|name| {
            schema[name]
                .as_array()
                .is_some_and(|fields| fields.iter().any(|field| field == key))
        })
}

fn bindings(_document: &Document, method: &MethodSpec) -> Vec<Binding> {
    let schema = &method.cli_inputs;
    schema
        .iter()
        .filter_map(|(key, shape)| {
            let mapping = &shape["x-cli-binding"];
            let kind = mapping["source"].as_str()?;
            let property = mapping["property"].as_str().unwrap_or(key).to_owned();
            let source = match kind {
                "query" => Source::Query(method.parameters.get(&property)?.clone()),
                "body" => Source::Body(shape.clone()),
                "envelope" => Source::Envelope(shape.clone()),
                "nested" => Source::Nested(
                    shape.clone(),
                    serde_json::from_value(mapping["path"].clone()).expect("checked input path"),
                ),
                _ => return None,
            };
            let prefix = match kind {
                "query" => "q",
                "body" => "b",
                "envelope" => "e",
                _ => "n",
            };
            Some(Binding {
                id: format!("{prefix}:{property}"),
                long: shape["x-cli-option"]
                    .as_str()
                    .expect("checked option")
                    .into(),
                property,
                source,
                repeated: shape["x-cli-repeated"] == true,
            })
        })
        .collect()
}

fn option(name: &str, label: &str, help: &str) -> Arg {
    Arg::new(name.to_owned())
        .long(name.to_owned())
        .value_name(label.to_owned())
        .help(help.to_owned())
}
fn default_parameter<'a>(
    _document: &Document,
    method: &'a MethodSpec,
    name: &str,
) -> Option<&'a str> {
    method
        .cli_inputs
        .values()
        .find(|shape| {
            shape["x-cli-binding"]["source"] == "query"
                && shape["x-cli-binding"]["property"] == name
        })
        .and_then(|shape| shape["default"].as_str())
}
fn shape_kind<'a>(document: &'a Document, binding: &'a Binding) -> (&'a str, bool) {
    match &binding.source {
        Source::Query(parameter) => (parameter.kind.as_str(), false),
        Source::Body(schema) | Source::Envelope(schema) | Source::Nested(schema, _) => {
            let shape = type_schema(document, schema);
            let item = if binding.repeated {
                type_schema(document, &shape["items"])
            } else {
                shape
            };
            (item["type"].as_str().unwrap_or("object"), true)
        }
    }
}
fn label(document: &Document, binding: &Binding) -> String {
    let (kind, body) = shape_kind(document, binding);
    if (kind == "object" || kind == "array") && body {
        "JSON".into()
    } else if kind == "boolean" {
        "BOOL".into()
    } else {
        binding.long.to_ascii_uppercase().replace('-', "_")
    }
}
pub fn direct_download(method: &MethodSpec) -> bool {
    method.supports_media_download
        && (method.response.is_none() || method.id.ends_with(".media.download"))
}

pub fn upload_mime(method: &MethodSpec) -> &str {
    let accept = method
        .media_upload
        .as_ref()
        .and_then(|upload| upload["accept"].as_array());
    if accept.is_some_and(|values| values.len() == 1 && values[0] == "message/*") {
        "message/rfc822"
    } else {
        "application/octet-stream"
    }
}
pub fn validate_upload(method: &MethodSpec, path: &str, mime: &str) -> Result<()> {
    reqwest::header::HeaderValue::from_str(mime).context("invalid upload MIME type")?;
    let upload = method
        .media_upload
        .as_ref()
        .context("this method does not support uploads")?;
    let input = method.cli_inputs.get("upload");
    let limit = input
        .and_then(|shape| shape["x-file-max-bytes"].as_u64())
        .or_else(|| {
            upload["maxSize"]
                .as_str()
                .and_then(|value| value.parse::<u64>().ok())
        })
        .unwrap_or(u64::MAX)
        .min(1024 * 1024 * 1024);
    let metadata = std::fs::metadata(path).context("cannot read upload file")?;
    ensure!(
        metadata.is_file() && metadata.len() <= limit,
        "upload must be a regular file no larger than {limit} bytes"
    );
    if let Some(accepted) = input
        .and_then(|shape| shape["x-file-mime-types"].as_array())
        .or_else(|| upload["accept"].as_array())
    {
        ensure!(
            accepted
                .iter()
                .filter_map(Value::as_str)
                .any(|kind| kind == "*/*"
                    || kind == mime
                    || kind
                        .strip_suffix('*')
                        .is_some_and(|prefix| mime.starts_with(prefix))),
            "this method does not accept the upload MIME type"
        );
    }
    Ok(())
}
pub fn validate_file_parameters(
    method: &MethodSpec,
    parameters: &Map<String, Value>,
) -> Result<()> {
    for name in parameters.keys() {
        ensure!(
            matches!(name.as_str(), "body" | "fields")
                || method.parameters.contains_key(name)
                || method.batch_fields.contains_key(name),
            "unknown request-file parameter {name:?}"
        );
    }
    Ok(())
}

pub fn command(service: &str, name: &str) -> Command {
    let contract = contract(service);
    let document = &contract.document;
    let method = &document.root.methods[name];
    let about = method
        .description
        .split('\n')
        .next()
        .unwrap_or(name)
        .to_owned();
    let about = about.split(". ").next().unwrap_or(&about).to_owned();
    let read = method.http_method == "GET" || method.http_method == "HEAD" || method.read_only;
    let mut command = Command::new(name.to_owned())
        .about(about)
        .long_about(method.description.clone())
        .override_usage(format!(
            "joom [OPTIONS] {service} <ACCOUNT> {name} [COMMAND_OPTIONS]"
        ))
        .arg(option(
            "file",
            "PATH",
            "Read command parameters from a JSON file",
        ))
        .arg(option("fields", "MASK", "Select response fields"));
    if read && !method.fresh_response && !direct_download(method) {
        command = command.arg(
            option(
                "ttl",
                "MINUTES",
                "Reuse encrypted response data; 0 disables caching",
            )
            .default_value("0")
            .value_parser(clap::value_parser!(u32)),
        );
    }
    if pagination::Plan::new(method, read, Some(1), None).is_ok() {
        command = command.arg(
            option(
                "max",
                "N",
                "Return at most N results; by default, return all results",
            )
            .value_parser(clap::value_parser!(u32).range(1..)),
        );
    }
    if pagination::Plan::new(method, read, None, Some(1)).is_ok() {
        command = command.arg(
            option(
                "max-pages",
                "N",
                "Stop after N pages; by default, fetch all pages",
            )
            .value_parser(clap::value_parser!(u32).range(1..=10000)),
        );
    }
    if method.supports_media_download || method.id == "mail.messages.attachments.get" {
        let mut output = option(
            "output",
            "PATH",
            "Save the downloaded content to a new file",
        )
        .short('o');
        if contract.inputs[name]["allOf"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|rule| {
                rule["x-cli-kind"] == "required"
                    && rule["x-cli-parameters"]
                        .as_array()
                        .is_some_and(|keys| keys.iter().any(|key| key == "output"))
            })
        {
            output = output.required_unless_present("file");
        }
        command = command.arg(output);
    }
    if method.media_upload.is_some() {
        command = command
            .arg(option(
                "upload",
                "PATH",
                contract.inputs[name]["properties"]["upload"]["description"]
                    .as_str()
                    .unwrap_or("Upload a local file"),
            ))
            .arg(
                option("content-type", "MIME", "Upload MIME type")
                    .default_value(upload_mime(method).to_owned()),
            );
    }
    let mut body_input = vec!["body".to_owned(), "file".to_owned()];
    if contract.compose_paths.contains_key(name) {
        for arg in crate::mail::options(&contract.compose_schema) {
            body_input.push(arg.get_id().to_string());
            command = command.arg(arg);
        }
    }
    if method.request.is_some() {
        command = command.arg(option("body", "JSON", "Inline JSON request body"));
    } else {
        body_input.retain(|name| name != "body");
    }
    for binding in bindings(document, method) {
        let (kind, body) = shape_kind(document, &binding);
        let mut help = match &binding.source {
            Source::Query(p) => p.description.as_str(),
            Source::Body(schema) | Source::Envelope(schema) | Source::Nested(schema, _) => {
                type_schema(document, schema)["description"]
                    .as_str()
                    .unwrap_or("Request body field")
            }
        }
        .to_owned();
        if let Source::Query(parameter) = &binding.source {
            if !parameter.choices.is_empty() {
                help.push_str(&format!(" Choices: {}.", parameter.choices.join(", ")));
            }
        } else if let Source::Body(schema) | Source::Envelope(schema) | Source::Nested(schema, _) =
            &binding.source
        {
            let shape = type_schema(document, schema);
            if let Some(choices) = shape["enum"].as_array() {
                help.push_str(&format!(
                    " Choices: {}.",
                    choices
                        .iter()
                        .map(|value| value.as_str().unwrap_or_default())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        let mut arg = Arg::new(binding.id.clone())
            .long(binding.long.clone())
            .value_name(label(document, &binding))
            .help(help.to_owned());
        if binding.repeated {
            arg = arg.action(ArgAction::Append).num_args(1..);
        } else if kind == "boolean" {
            arg = arg
                .num_args(0..=1)
                .require_equals(true)
                .default_missing_value("true");
        }
        if let Source::Query(p) = &binding.source {
            if let Some(value) = default_parameter(document, method, &binding.property) {
                arg = arg.default_value(value.to_owned());
            } else if p.required {
                arg = arg.required_unless_present("file");
            }
            let p = p.clone();
            let property = binding.property.clone();
            arg = arg.value_parser(move |value: &str| {
                let value = Value::String(value.into());
                crate::discovery::validate_parameter(&property, &p, &value)
                    .map(|_| value.as_str().unwrap().to_owned())
                    .map_err(|error| error.to_string())
            });
        } else if body {
            body_input.push(binding.id.clone());
        }
        command = command.arg(arg);
    }
    if let Some(request) = input_request(method) {
        let schema = type_schema(document, request);
        let empty = schema["properties"].as_object().is_some_and(Map::is_empty)
            || schema["type"] == "object"
                && schema.get("properties").is_none()
                && schema.get("additionalProperties").is_none();
        if !empty {
            if method.media_upload.is_some() {
                body_input.push("upload".into());
            }
            command = command.group(
                ArgGroup::new("request-content")
                    .args(body_input)
                    .required(
                        contract.inputs[name]["allOf"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|rule| rule["x-cli-kind"] == "content"),
                    )
                    .multiple(true),
            );
        }
    }
    crate::help::register_conflicts(command.after_help(format!("Microsoft support: docs/microsoft.md\nParameter schema: schemas/{service}.schema.json#/$defs/{name}")), &contract.inputs[name])
}
pub fn commands(service: &str) -> Vec<Command> {
    contract(service)
        .document
        .root
        .methods
        .keys()
        .map(|name| command(service, name))
        .collect()
}

fn parse_scalar(kind: &str, text: &str) -> Result<Value> {
    match kind {
        "integer" | "number" | "boolean" => {
            serde_json::from_str(text).context("invalid numeric or boolean value")
        }
        "object" | "array" => {
            ensure!(
                text.len() as u64 <= http::DISCOVERY_LIMIT,
                "JSON input exceeds 16 MiB"
            );
            serde_json::from_str(text).context("invalid JSON field value")
        }
        _ => Ok(json!(text)),
    }
}
fn value_for(document: &Document, binding: &Binding, values: Vec<&str>) -> Result<Value> {
    let (kind, _) = shape_kind(document, binding);
    if binding.repeated {
        Ok(Value::Array(
            values
                .into_iter()
                .map(|value| parse_scalar(kind, value))
                .collect::<Result<_>>()?,
        ))
    } else {
        parse_scalar(kind, values[0])
    }
}

pub fn validate_body(
    document: &Document,
    method: &str,
    schema: &Value,
    value: &Value,
    path: &str,
) -> Result<()> {
    fn check(
        document: &Document,
        method: &str,
        schema: &Value,
        value: &Value,
        path: &str,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth <= 128, "request body exceeds nesting limit");
        if let Some(reference) = schema["$ref"].as_str() {
            let name = reference
                .strip_prefix("#/x-body-types/")
                .or_else(|| reference.strip_prefix("#/x-input-types/"))
                .unwrap_or(reference);
            return check(
                document,
                method,
                document.schemas.get(name).context("unknown body type")?,
                value,
                path,
                depth + 1,
            );
        }
        if let Some(choices) = schema["anyOf"].as_array() {
            ensure!(
                choices.iter().any(|schema| check(
                    document,
                    method,
                    schema,
                    value,
                    path,
                    depth + 1
                )
                .is_ok()),
                "invalid value for {path}"
            );
            return Ok(());
        }
        match schema["type"].as_str().unwrap_or_default() {
            "null" => ensure!(value.is_null(), "{path} must be null"),
            "string" => ensure!(value.is_string(), "{path} must be a string"),
            "integer" => ensure!(
                value.as_i64().is_some() || value.as_u64().is_some(),
                "{path} must be an integer"
            ),
            "number" => ensure!(value.is_number(), "{path} must be a number"),
            "boolean" => ensure!(value.is_boolean(), "{path} must be a boolean"),
            "array" => {
                let items = value
                    .as_array()
                    .with_context(|| format!("{path} must be an array"))?;
                if let Some(maximum) = schema["maxItems"].as_u64() {
                    ensure!(items.len() as u64 <= maximum, "{path} has too many items");
                }
                if let Some(minimum) = schema["minItems"].as_u64() {
                    ensure!(items.len() as u64 >= minimum, "{path} has too few items");
                }
                for (index, item) in items.iter().enumerate() {
                    check(
                        document,
                        method,
                        &schema["items"],
                        item,
                        &format!("{path}[{index}]"),
                        depth + 1,
                    )?;
                }
            }
            "object" => {
                let object = value
                    .as_object()
                    .with_context(|| format!("{path} must be an object"))?;
                if let Some(fields) = schema["x-required-one-of"]
                    .get(method)
                    .and_then(Value::as_array)
                {
                    ensure!(
                        fields.iter().any(|key| object
                            .get(key.as_str().unwrap())
                            .is_some_and(|value| !value.is_null())),
                        "{path}: provide one of {}",
                        fields
                            .iter()
                            .map(|key| key.as_str().unwrap())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                for group in schema["x-exclusive-fields"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    let active = group
                        .as_array()
                        .expect("checked exclusive fields")
                        .iter()
                        .filter_map(|key| {
                            let key = key.as_str().unwrap();
                            object
                                .get(key)
                                .is_some_and(|value| !value.is_null())
                                .then_some(key)
                        })
                        .collect::<Vec<_>>();
                    ensure!(
                        active.len() <= 1,
                        "{path}: conflicting fields {}",
                        active.join(", ")
                    );
                }
                if !method.is_empty()
                    && let Some(required) = schema
                        .get("required")
                        .or_else(|| schema.get("x-required-fields"))
                        .and_then(Value::as_array)
                {
                    for key in required {
                        let key = key.as_str().context("invalid required property")?;
                        ensure!(object.contains_key(key), "{path}.{key} is required");
                    }
                }
                for (key, property) in schema["properties"].as_object().into_iter().flatten() {
                    if !method.is_empty() && required_input_field(schema, key, property, method) {
                        ensure!(
                            object.get(key).is_some_and(|value| !value.is_null()),
                            "{path}.{key} is required"
                        );
                    }
                }
                for (key, value) in object {
                    let property = schema["properties"]
                        .get(key)
                        .or_else(|| {
                            schema
                                .get("additionalProperties")
                                .filter(|value| value.is_object())
                        })
                        .with_context(|| format!("unknown body field {path}[{key:?}]"))?;
                    check(
                        document,
                        method,
                        property,
                        value,
                        &format!("{path}[{key:?}]"),
                        depth + 1,
                    )?;
                }
            }
            _ => {}
        }
        if let Some(choices) = schema["enum"].as_array() {
            ensure!(choices.contains(value), "invalid choice for {path}");
        }
        Ok(())
    }
    check(document, method, schema, value, path, 0)
}

pub fn resolve(
    service: &str,
    account: &str,
    name: &str,
    args: &ArgMatches,
    mut params: Map<String, Value>,
) -> Result<Invocation> {
    let contract = contract(service);
    let document = &contract.document;
    let method = &document.root.methods[name];
    let compose = if contract.compose_paths.contains_key(name) {
        params.remove("compose")
    } else {
        None
    };
    validate_file_parameters(method, &params)?;
    let mut envelope = Map::new();
    for key in method.batch_fields.keys() {
        if let Some(value) = params.remove(key) {
            envelope.insert(key.clone(), value);
        }
    }
    if native::text(args, "content-type").is_some()
        && args.value_source("content-type") == Some(clap::parser::ValueSource::CommandLine)
    {
        ensure!(
            native::text(args, "upload").is_some(),
            "--content-type requires --upload"
        );
    }
    let custom_fields = params.contains_key("fields")
        || native::text(args, "fields").is_some()
        || ["personFields", "readMask", "part"].iter().any(|name| {
            params.contains_key(*name)
                || native::text(args, &format!("q:{name}")).is_some()
                    && args
                        .value_source(&format!("q:{name}"))
                        .is_some_and(|source| source == clap::parser::ValueSource::CommandLine)
        })
        || params.get("format").is_some_and(|format| format != "full")
        || native::text(args, "q:format").is_some_and(|format| format != "full");
    let mut body = params.remove("body");
    if let Some(raw) = native::text(args, "body") {
        ensure!(
            body.is_none(),
            "choose either --body or the body's entry in --file"
        );
        ensure!(
            raw.len() as u64 <= http::DISCOVERY_LIMIT,
            "request body exceeds 16 MiB"
        );
        body = Some(crate::read_json(raw)?);
    }
    for binding in bindings(document, method) {
        let Some(values) = args.try_get_many::<String>(&binding.id).ok().flatten() else {
            continue;
        };
        let value = value_for(document, &binding, values.map(String::as_str).collect())?;
        match binding.source {
            Source::Query(_) => {
                if args.value_source(&binding.id) == Some(clap::parser::ValueSource::CommandLine)
                    || !params.contains_key(&binding.property)
                {
                    params.insert(binding.property, value);
                }
            }
            Source::Body(_) => {
                body.get_or_insert_with(|| json!({}))
                    .as_object_mut()
                    .context("named body fields require an object body")?
                    .insert(binding.property, value);
            }
            Source::Envelope(_) => {
                envelope.insert(binding.property, value);
            }
            Source::Nested(_, path) => {
                crate::file_inputs::set_path(body.get_or_insert_with(|| json!({})), &path, value)?;
            }
        }
    }
    for name in method.parameters.keys() {
        if let Some(value) = default_parameter(document, method, name) {
            params.entry(name).or_insert(json!(value));
        }
    }
    if let Some(fields) = native::text(args, "fields") {
        params.insert("fields".into(), json!(fields));
    }
    crate::file_inputs::validate_constraints(
        service,
        name,
        args,
        crate::file_inputs::RequestValues {
            params: &params,
            body: body.as_ref(),
            envelope: &envelope,
            compose: compose.as_ref(),
        },
    )?;
    if let Some(path) = contract.compose_paths.get(name) {
        crate::mail::compose(
            account,
            name,
            args,
            compose,
            &contract.compose_schema,
            path,
            &mut body,
        )?;
    }
    if let Some(request) = input_request(method) {
        if body.is_none() && native::text(args, "upload").is_none() {
            body = Some(json!({}));
        }
        if let Some(body) = &body {
            validate_body(document, &method.id, request, body, "body")?;
            ensure!(
                serde_json::to_vec(body)?.len() as u64 <= http::DISCOVERY_LIMIT,
                "request body exceeds 16 MiB"
            );
        }
    } else {
        ensure!(body.is_none(), "this command does not accept a body");
    }
    if let Some(operation) = &method.batch_operation {
        for (key, value) in &envelope {
            validate_body(document, &method.id, &method.batch_fields[key], value, key)?;
        }
        envelope.insert(
            "requests".into(),
            json!([{operation:body.unwrap_or_else(||json!({}))}]),
        );
        let encoded = Value::Object(envelope);
        validate_body(
            document,
            &method.id,
            method
                .request
                .as_ref()
                .context("missing batch request schema")?,
            &encoded,
            "body",
        )?;
        ensure!(
            serde_json::to_vec(&encoded)?.len() as u64 <= http::DISCOVERY_LIMIT,
            "request body exceeds 16 MiB"
        );
        body = Some(encoded);
    }
    document.url_with_body(method, &params, body.as_ref())?;
    Ok(Invocation {
        method: name.into(),
        params,
        body,
        custom_fields,
        relative_times: Vec::new(),
        options: None,
    })
}

#[cfg(test)]
#[path = "../tests/unit/schema_commands.rs"]
mod tests;
