mod backend;
mod tasks;
#[cfg(test)]
#[path = "../../tests/unit/wrappers.rs"]
mod tests;
mod values;

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Map, Value, json};
use std::{ffi::OsString, io::Write, path::Path, process::ExitCode, sync::OnceLock};

pub fn catalog() -> &'static Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../../schemas/wrappers.schema.json"))
            .expect("wrapper schemas")
    })
}

pub fn executable(arg: &std::ffi::OsStr) -> Option<String> {
    let name = Path::new(arg).file_stem()?.to_str()?;
    if !name.starts_with("jm") {
        return None;
    }
    catalog()["x-wrappers"].get(name).map(|_| name.to_owned())
}

// Expand references for the parser and renderer. The authored file remains a
// standard JSON Schema, including the shared properties evaluated by allOf.
pub fn expand(schema: &Value) -> Value {
    match schema {
        Value::Object(map) => {
            let mut result = if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                expand(
                    catalog()
                        .pointer(reference.strip_prefix('#').unwrap())
                        .expect("schema reference"),
                )
            } else {
                json!({})
            };
            for (key, value) in map {
                if key != "$ref" {
                    result[key] = expand(value);
                }
            }
            if let Some(rules) = result["allOf"].as_array().cloned() {
                for rule in &rules {
                    for (key, shape) in rule["properties"].as_object().into_iter().flatten() {
                        if rule.get("if").is_none() && rule.get("description").is_none() {
                            if !result["properties"].is_object() {
                                result["properties"] = json!({});
                            }
                            result["properties"][key] = shape.clone();
                        }
                    }
                }
            }
            result
        }
        Value::Array(items) => Value::Array(items.iter().map(expand).collect()),
        _ => schema.clone(),
    }
}

pub fn definition(wrapper: &str, command: &str) -> Result<Value> {
    catalog()["$defs"]
        .get(format!("{wrapper}.{command}"))
        .map(expand)
        .with_context(|| format!("unknown command {command:?}; use {wrapper} --help"))
}

pub fn key(name: &str, shape: &Value) -> String {
    shape["x-key"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| crate::schema_commands::kebab(name))
}

fn value_label(shape: &Value) -> &str {
    if shape["type"] == "array" {
        return value_label(&shape["items"]);
    }
    if let Some(label) = shape["x-label"].as_str() {
        return label;
    }
    match shape["format"].as_str() {
        Some("email") => "EMAIL",
        Some("local-file" | "new-file") => "PATH",
        Some("local-date") => "DATE",
        Some("local-time" | "date-time") => "TIME",
        _ => match shape["type"].as_str() {
            Some("integer") => "N",
            Some("boolean") => "BOOL",
            _ => "TEXT",
        },
    }
}

#[derive(Debug)]
pub struct Request {
    pub wrapper: String,
    pub account: String,
    pub command: String,
    pub params: Map<String, Value>,
}

impl Request {
    pub fn text(&self, name: &str) -> Option<&str> {
        self.params.get(name).and_then(Value::as_str)
    }
    pub fn flag(&self, name: &str) -> bool {
        self.params
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
    pub fn strings(&self, name: &str) -> Vec<String> {
        self.params
            .get(name)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    }
    pub fn target(&self) -> &str {
        self.text("target").expect("validated target")
    }
    pub fn limit(&self) -> Option<u64> {
        self.params.get("limit").and_then(Value::as_u64)
    }
    pub fn list_params(&self, mut params: Value) -> Value {
        if let Some(limit) = self.limit() {
            params["max"] = json!(limit);
        }
        params
    }
}

pub enum Parsed {
    Help(String),
    Request(Request),
}

pub fn parse(wrapper: &str, args: &[OsString]) -> Result<Parsed> {
    let words = args
        .iter()
        .map(|word| {
            word.to_str()
                .context("arguments must be UTF-8")
                .map(str::to_owned)
        })
        .collect::<Result<Vec<_>>>()?;
    let options_end = words
        .iter()
        .position(|word| word == "--")
        .unwrap_or(words.len());
    let verbose = words[..options_end].iter().any(|word| word == "--verbose");
    let help = words[..options_end]
        .iter()
        .any(|word| matches!(word.as_str(), "--help" | "-h"));
    ensure!(
        !verbose || help,
        "use --help --verbose to see advanced parameters"
    );
    if words.is_empty()
        || words
            .first()
            .is_some_and(|word| matches!(word.as_str(), "--help" | "-h"))
    {
        return Ok(Parsed::Help(help_text(wrapper, None, verbose)?));
    }
    let account = words[0].clone();
    let remaining = words
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(index, word)| {
            *index >= options_end || !matches!(word.as_str(), "--help" | "-h" | "--verbose")
        })
        .map(|(_, word)| word)
        .collect::<Vec<_>>();
    let explicit = remaining.first().is_some_and(|word| {
        catalog()["$defs"]
            .get(format!("{wrapper}.{word}"))
            .is_some()
    });
    let command = if explicit {
        remaining[0].as_str()
    } else if wrapper == "jmmail"
        && remaining.iter().any(|word| {
            ["to:", "cc:", "bcc:"]
                .iter()
                .any(|key| word.starts_with(key))
        })
    {
        "send"
    } else {
        catalog()["x-wrappers"][wrapper]["default"]
            .as_str()
            .context("unknown wrapper")?
    };
    if help {
        return Ok(Parsed::Help(help_text(
            wrapper,
            (explicit || !remaining.is_empty()).then_some(command),
            verbose,
        )?));
    }
    let schema = definition(wrapper, command)?;
    let properties = schema["properties"].as_object().unwrap();
    let mut positionals = properties
        .iter()
        .filter_map(|(name, shape)| {
            shape["x-position"]
                .as_u64()
                .map(|position| (position, name))
        })
        .collect::<Vec<_>>();
    positionals.sort_by_key(|(position, _)| *position);
    let mut explicit_values = Map::new();
    let mut file = None;
    let mut literal = false;
    for word in remaining.iter().skip(usize::from(explicit)) {
        if *word == "--" && !literal {
            literal = true;
            continue;
        }
        if !literal && let Some(path) = word.strip_prefix("file:") {
            ensure!(file.is_none() && !path.is_empty(), "supply file:PATH once");
            file = Some(path.to_owned());
            continue;
        }
        let named =
            (!literal)
                .then(|| word.split_once(':'))
                .flatten()
                .and_then(|(option, value)| {
                    properties
                        .iter()
                        .find(|(name, shape)| option == key(name, shape))
                        .map(|(name, shape)| (name, shape, value))
                });
        if let Some((name, shape, value)) = named {
            let value = parse_value(shape, value)?;
            if shape["type"] == "array" {
                explicit_values
                    .entry(name.clone())
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap()
                    .push(value);
            } else {
                ensure!(
                    !explicit_values.contains_key(name),
                    "{}: can only be supplied once",
                    key(name, shape)
                );
                explicit_values.insert(name.clone(), value);
            }
        } else if let Some((_, name)) = positionals
            .iter()
            .find(|(_, name)| !explicit_values.contains_key(name.as_str()))
        {
            explicit_values.insert((*name).clone(), parse_value(&properties[*name], word)?);
        } else {
            bail!("unexpected argument {word:?}; use {wrapper} ACCOUNT {command} --help");
        }
    }
    let mut params = if let Some(path) = file {
        let bytes = crate::http::limited(
            std::fs::File::open(path).context("cannot read parameter file")?,
            crate::http::DISCOVERY_LIMIT,
        )?;
        let value: Value = serde_json::from_slice(&bytes).context("parameter file must be JSON")?;
        value
            .as_object()
            .context("parameter file must contain an object")?
            .clone()
    } else {
        Map::new()
    };
    // Inline content and file content are alternative representations of one
    // value. An explicit representation replaces the other from a JSON file.
    for rule in schema["allOf"].as_array().into_iter().flatten() {
        if let Some(group) = rule.pointer("/not/required").and_then(Value::as_array)
            && group.iter().all(|name| {
                matches!(
                    name.as_str(),
                    Some("body" | "bodyFile" | "text" | "textFile")
                )
            })
            && group
                .iter()
                .any(|name| explicit_values.contains_key(name.as_str().unwrap()))
        {
            for name in group.iter().filter_map(Value::as_str) {
                if !explicit_values.contains_key(name) {
                    params.remove(name);
                }
            }
        }
    }
    params.extend(explicit_values);
    for (name, shape) in properties {
        if let Some(default) = shape.get("default")
            && shape
                .get("x-default-when")
                .is_none_or(|condition| assertion(condition, &Value::Object(params.clone())))
        {
            params
                .entry(name.clone())
                .or_insert_with(|| default.clone());
        }
    }
    validate(&schema, &Value::Object(params.clone()), "parameters")?;
    for (name, shape) in properties {
        if let Some(value) = params.get_mut(name) {
            values::normalize(shape, value)
                .with_context(|| format!("invalid {}:", key(name, shape)))?;
        }
    }
    values::preflight(&params, wrapper == "jmcal")?;
    Ok(Parsed::Request(Request {
        wrapper: wrapper.into(),
        command: command.into(),
        account,
        params,
    }))
}

fn parse_value(shape: &Value, value: &str) -> Result<Value> {
    let shape = if shape["type"] == "array" {
        &shape["items"]
    } else {
        shape
    };
    Ok(match shape["type"].as_str() {
        Some("boolean") => json!(value.parse::<bool>().context("use true or false")?),
        Some("integer") => json!(value.parse::<u64>().context("use a positive integer")?),
        _ if shape.get("anyOf").is_some() => value
            .parse::<u64>()
            .map(|number| json!(number))
            .unwrap_or_else(|_| json!(value)),
        _ if shape["x-text"] == true => json!(values::unescape(value)),
        _ => json!(value),
    })
}

// This validator intentionally accepts only the assertion vocabulary used by
// the wrapper contracts. Tests check that every authored keyword is supported.
pub fn validate(shape: &Value, value: &Value, path: &str) -> Result<()> {
    if let Some(choices) = shape["anyOf"].as_array() {
        ensure!(
            choices
                .iter()
                .any(|choice| validate(choice, value, path).is_ok()),
            "{path}: {}",
            shape["description"]
                .as_str()
                .unwrap_or("choose an allowed value")
        );
    }
    if let Some(kind) = shape["type"].as_str() {
        let valid = match kind {
            "object" => value.is_object(),
            "string" => value.is_string(),
            "array" => value.is_array(),
            "boolean" => value.is_boolean(),
            "integer" => value.as_u64().is_some(),
            _ => false,
        };
        ensure!(valid, "{path} must be {kind}");
    }
    if let Some(expected) = shape.get("const") {
        ensure!(value == expected, "{path} must be {expected}");
    }
    if let Some(choices) = shape["enum"].as_array() {
        ensure!(
            choices.contains(value),
            "{path}: choose {}",
            choices
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if let Some(text) = value.as_str() {
        ensure!(
            shape["minLength"]
                .as_u64()
                .is_none_or(|min| text.chars().count() as u64 >= min),
            "{path} must not be empty"
        );
        ensure!(
            shape["maxLength"]
                .as_u64()
                .is_none_or(|max| text.chars().count() as u64 <= max),
            "{path} is too long"
        );
        if let Some(format) = shape["format"].as_str() {
            values::format(format, text).with_context(|| path.to_owned())?;
            if format == "local-file"
                && let Some(maximum) = shape["x-max-bytes"].as_u64()
            {
                ensure!(
                    std::fs::metadata(text)?.len() <= maximum,
                    "{path} exceeds the maximum file size of {maximum} bytes"
                );
            }
        }
    }
    if let Some(number) = value.as_u64() {
        ensure!(
            shape["minimum"].as_u64().is_none_or(|min| number >= min)
                && shape["maximum"].as_u64().is_none_or(|max| number <= max),
            "{path} is outside its allowed range"
        );
    }
    if let Some(items) = value.as_array() {
        ensure!(
            shape["minItems"]
                .as_u64()
                .is_none_or(|min| items.len() as u64 >= min)
                && shape["maxItems"]
                    .as_u64()
                    .is_none_or(|max| items.len() as u64 <= max),
            "{path}: wrong number of values"
        );
        for (index, item) in items.iter().enumerate() {
            validate(&shape["items"], item, &format!("{path}[{index}]"))?;
        }
    }
    if let Some(object) = value.as_object() {
        for required in shape["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            ensure!(
                object.contains_key(required),
                "provide {}",
                key(required, &shape["properties"][required])
            );
        }
        if shape["unevaluatedProperties"] == false {
            for name in object.keys() {
                ensure!(
                    shape["properties"].get(name).is_some(),
                    "unknown parameter {name:?}"
                );
            }
        }
        for (name, shape) in shape["properties"].as_object().into_iter().flatten() {
            if let Some(value) = object.get(name) {
                validate(shape, value, &key(name, shape))?;
            }
        }
    }
    for rule in shape["allOf"].as_array().into_iter().flatten() {
        if rule.get("description").is_some() {
            ensure!(
                assertion(rule, value),
                "{}",
                rule["description"].as_str().unwrap()
            );
        } else {
            validate(rule, value, path)?;
        }
    }
    Ok(())
}

fn assertion(rule: &Value, value: &Value) -> bool {
    if rule["required"].as_array().is_some_and(|keys| {
        value.as_object().is_some_and(|object| {
            keys.iter()
                .any(|key| !object.contains_key(key.as_str().unwrap()))
        })
    }) {
        return false;
    }
    if let Some(expected) = rule.get("const")
        && expected != value
    {
        return false;
    }
    if rule["enum"]
        .as_array()
        .is_some_and(|choices| !choices.contains(value))
    {
        return false;
    }
    for (name, rule) in rule["properties"].as_object().into_iter().flatten() {
        if let Some(value) = value.get(name)
            && !assertion(rule, value)
        {
            return false;
        }
    }
    if rule["anyOf"]
        .as_array()
        .is_some_and(|rules| !rules.iter().any(|rule| assertion(rule, value)))
        || rule["allOf"]
            .as_array()
            .is_some_and(|rules| !rules.iter().all(|rule| assertion(rule, value)))
        || rule.get("not").is_some_and(|rule| assertion(rule, value))
    {
        return false;
    }
    if let Some(condition) = rule.get("if") {
        let branch = if assertion(condition, value) {
            "then"
        } else {
            "else"
        };
        if rule.get(branch).is_some_and(|rule| !assertion(rule, value)) {
            return false;
        }
    }
    true
}

pub fn help_text(wrapper: &str, command: Option<&str>, verbose: bool) -> Result<String> {
    let mut out = String::new();
    if let Some(command) = command {
        let schema = definition(wrapper, command)?;
        let mut positions = schema["properties"]
            .as_object()
            .unwrap()
            .iter()
            .filter_map(|(name, shape)| {
                shape["x-position"]
                    .as_u64()
                    .map(|position| (position, name, shape))
            })
            .collect::<Vec<_>>();
        positions.sort_by_key(|(position, _, _)| *position);
        out.push_str(schema["description"].as_str().unwrap());
        out.push_str(&format!("\n\nUsage: {wrapper} ACCOUNT {command}"));
        for (_, name, shape) in positions {
            let label = shape["x-label"].as_str().unwrap_or(name);
            let required = schema["required"]
                .as_array()
                .is_some_and(|keys| keys.iter().any(|key| key == name));
            out.push_str(&if required {
                format!(" {label}")
            } else {
                format!(" [{label}]")
            });
        }
        out.push_str(" [key:value ...]\n");
        for (heading, required, advanced) in [
            ("Mandatory parameters", true, false),
            ("Optional parameters", false, false),
            ("Optional parameters (advanced)", false, true),
        ] {
            let fields = schema["properties"]
                .as_object()
                .unwrap()
                .iter()
                .filter(|(name, shape)| {
                    (verbose || !advanced)
                        && (shape["x-advanced"] == true) == advanced
                        && schema["required"]
                            .as_array()
                            .is_some_and(|keys| keys.iter().any(|key| key == *name))
                            == required
                })
                .collect::<Vec<_>>();
            if !fields.is_empty() {
                out.push_str(&format!("\n{heading}:\n"));
                for (name, shape) in fields {
                    let label = key(name, shape);
                    let fallback = format!("{}.", shape["x-label"].as_str().unwrap_or(name));
                    let description = shape["description"].as_str().unwrap_or(&fallback);
                    out.push_str(&format!("  {label}:{}  {description}", value_label(shape)));
                    if shape["x-repeat"] == true {
                        out.push_str(" Repeat for multiple values.");
                    }
                    if let Some(choices) = shape["enum"].as_array() {
                        out.push_str(&format!(
                            " Choices: {}.",
                            choices
                                .iter()
                                .map(|value| value
                                    .as_str()
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| value.to_string()))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    if let Some(default) = shape.get("default") {
                        out.push_str(&format!(" Default: {default}."));
                    }
                    if let Some(max) = shape["maxLength"].as_u64() {
                        out.push_str(&format!(" Maximum {max} characters."));
                    }
                    if let Some(max) = shape["maxItems"].as_u64() {
                        out.push_str(&format!(" Maximum {max} values."));
                    }
                    if let (Some(min), Some(max)) =
                        (shape["minimum"].as_u64(), shape["maximum"].as_u64())
                    {
                        out.push_str(&format!(" Allowed range: {min} to {max}."));
                    }
                    out.push('\n');
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        let choices = schema["allOf"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|rule| rule["description"].as_str())
            .filter(|text| seen.insert(*text))
            .collect::<Vec<_>>();
        if !choices.is_empty() {
            out.push_str("\nInput choices:\n");
            for choice in choices {
                out.push_str(&format!("  {choice}\n"));
            }
        }
    } else {
        out.push_str(
            catalog()["x-wrappers"][wrapper]["description"]
                .as_str()
                .context("unknown wrapper")?,
        );
        out.push_str(&format!(
            "\n\nUsage: {wrapper} ACCOUNT [COMMAND] [key:value ...]\n\nCommands:\n"
        ));
        for (name, schema) in catalog()["$defs"].as_object().unwrap() {
            if let Some(command) = name.strip_prefix(&format!("{wrapper}.")) {
                out.push_str(&format!(
                    "  {command:14} {}\n",
                    schema["description"].as_str().unwrap()
                ));
            }
        }
        out.push_str(&format!(
            "\nDefault: {}.\n",
            catalog()["x-wrappers"][wrapper]["default"]
                .as_str()
                .unwrap()
        ));
    }
    out.push_str("\nUse file:PATH to read JSON parameters. Explicit values replace file values.\nUse --help --verbose for advanced parameters.\nGuide: docs/wrappers.md\n");
    Ok(out)
}

pub fn main(wrapper: &str, args: Vec<OsString>) -> ExitCode {
    let parsed = match parse(wrapper, &args) {
        Ok(parsed) => parsed,
        Err(error) => {
            let json = args.iter().any(|arg| arg == "json:true");
            let _ = if json {
                writeln!(
                    std::io::stderr().lock(),
                    "{}",
                    json!({"error":format!("{error:#}")})
                )
            } else {
                writeln!(std::io::stderr().lock(), "error: {error:#}")
            };
            return ExitCode::from(2);
        }
    };
    match parsed {
        Parsed::Help(text) => {
            let _ = write!(std::io::stdout().lock(), "{text}");
            ExitCode::SUCCESS
        }
        Parsed::Request(request) => {
            let result = backend::Joom::new(&request)
                .and_then(|mut backend| tasks::execute(&request, &mut backend));
            match result.and_then(|value| crate::output::emit(&value, request.flag("json"), false))
            {
                Ok(()) => ExitCode::SUCCESS,
                Err(error)
                    if error.chain().any(|error| {
                        error
                            .downcast_ref::<std::io::Error>()
                            .is_some_and(|error| error.kind() == std::io::ErrorKind::BrokenPipe)
                    }) =>
                {
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    let _ = if request.flag("json") {
                        writeln!(
                            std::io::stderr().lock(),
                            "{}",
                            json!({"error":format!("{error:#}")})
                        )
                    } else {
                        writeln!(std::io::stderr().lock(), "error: {error:#}")
                    };
                    ExitCode::FAILURE
                }
            }
        }
    }
}
