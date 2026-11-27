use anyhow::{Context, Result, ensure};
use clap::{ArgMatches, Command, parser::ValueSource};
use serde_json::{Map, Value, json};

use crate::{native, schema_commands};

pub fn set_path(root: &mut Value, path: &[String], value: Value) -> Result<()> {
    let (key, parents) = path.split_last().context("empty CLI input target")?;
    let mut target = root;
    for part in parents {
        target = target
            .as_object_mut()
            .context("nested inputs require object parents")?
            .entry(part)
            .or_insert_with(|| json!({}));
    }
    target
        .as_object_mut()
        .context("nested inputs require an object")?
        .insert(key.clone(), value);
    Ok(())
}

pub struct RequestValues<'a> {
    pub params: &'a Map<String, Value>,
    pub body: Option<&'a Value>,
    pub envelope: &'a Map<String, Value>,
    pub compose: Option<&'a Value>,
}

pub fn validate_constraints(
    service: &str,
    action: &str,
    args: &ArgMatches,
    values: RequestValues<'_>,
) -> Result<()> {
    let contract = schema_commands::contract(service);
    let mut input = Map::new();
    let composition =
        crate::mail::merge_input(args, values.compose.cloned(), &contract.compose_schema)?;
    if let Some(compose) = &composition {
        input.insert("compose".into(), compose.clone());
    }
    for (key, shape) in contract.inputs[action]["properties"].as_object().unwrap() {
        let mapping = &shape["x-cli-binding"];
        let Some(source) = mapping["source"].as_str() else {
            continue;
        };
        let property = mapping["property"].as_str().unwrap();
        let id = crate::help::input_id(shape);
        let value = match source {
            "query" => values.params.get(property),
            "raw" => values.body,
            "body" => values.body.and_then(|body| body.get(property)),
            "envelope" => values.envelope.get(property),
            "nested" => {
                let mut value = values.body;
                for key in mapping["path"].as_array().unwrap() {
                    value = value.and_then(|value| value.get(key.as_str().unwrap()));
                }
                value
            }
            "compose" => composition
                .as_ref()
                .and_then(|compose| compose.get(property)),
            _ => None,
        };
        if let Some(value) = value {
            input.insert(key.clone(), value.clone());
        } else if matches!(source, "control" | "native") && explicit(args, &id) {
            if let Ok(Some(value)) = args.try_get_one::<String>(&id) {
                input.insert(key.clone(), Value::String(value.clone()));
            } else if let Ok(Some(value)) = args.try_get_one::<u32>(&id) {
                input.insert(key.clone(), json!(value));
            }
        }
    }
    validate_rules(&contract.inputs[action], &Value::Object(input))
}

// Command assertions use the same JSON Schema keywords in files and at runtime.
// Input types are validated separately by the request and argument parsers.
pub(crate) fn matches_rule(schema: &Value, value: &Value) -> bool {
    if let Some(kind) = schema["type"].as_str() {
        let valid = match kind {
            "null" => value.is_null(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "string" => value.is_string(),
            "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            _ => false,
        };
        if !valid {
            return false;
        }
    }
    if let Some(object) = value.as_object() {
        if schema["required"].as_array().is_some_and(|keys| {
            keys.iter()
                .any(|key| !object.contains_key(key.as_str().unwrap()))
        }) {
            return false;
        }
        for (key, assertion) in schema["properties"].as_object().into_iter().flatten() {
            if object
                .get(key)
                .is_some_and(|value| !matches_rule(assertion, value))
            {
                return false;
            }
        }
    }
    if let Some(items) = value.as_array()
        && (schema["minItems"]
            .as_u64()
            .is_some_and(|min| (items.len() as u64) < min)
            || schema
                .get("items")
                .is_some_and(|rule| items.iter().any(|value| !matches_rule(rule, value))))
    {
        return false;
    }
    if schema["allOf"]
        .as_array()
        .is_some_and(|rules| !rules.iter().all(|rule| matches_rule(rule, value)))
        || schema["anyOf"]
            .as_array()
            .is_some_and(|rules| !rules.iter().any(|rule| matches_rule(rule, value)))
        || schema
            .get("not")
            .is_some_and(|rule| matches_rule(rule, value))
    {
        return false;
    }
    if let Some(condition) = schema.get("if") {
        let branch = if matches_rule(condition, value) {
            "then"
        } else {
            "else"
        };
        if schema
            .get(branch)
            .is_some_and(|rule| !matches_rule(rule, value))
        {
            return false;
        }
    }
    true
}

pub(crate) fn validate_rules(schema: &Value, value: &Value) -> Result<()> {
    for rule in schema["allOf"].as_array().into_iter().flatten() {
        ensure!(
            matches_rule(rule, value),
            "{}",
            rule["description"]
                .as_str()
                .unwrap_or("invalid command parameters")
        );
    }
    Ok(())
}

fn definition(service: &str, action: &str) -> Command {
    if schema_commands::contract(service)
        .document
        .root
        .methods
        .contains_key(action)
    {
        schema_commands::command(service, action)
    } else {
        native::commands(service)
            .into_iter()
            .find(|command| command.get_name() == action)
            .expect("registered command")
    }
}

fn explicit(args: &ArgMatches, id: &str) -> bool {
    args.try_contains_id(id).unwrap_or(false)
        && args.value_source(id) == Some(ValueSource::CommandLine)
}

fn append(argv: &mut Vec<String>, long: Option<&str>, value: &Value, equals: bool) {
    if let Some(values) = value.as_array() {
        for value in values {
            append(argv, long, value, equals);
        }
        return;
    }
    let value = value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string());
    if let Some(long) = long {
        if equals {
            argv.push(format!("--{long}={value}"));
        } else {
            argv.extend([format!("--{long}"), value]);
        }
    } else {
        argv.push(value);
    }
}

// File fields use the same input contract as named CLI arguments. Translation
// happens before the existing Microsoft request resolver sees any parameters.
pub fn prepare(
    service: &str,
    action: &str,
    args: &ArgMatches,
    mut file: Map<String, Value>,
) -> Result<(Map<String, Value>, ArgMatches)> {
    let contract = schema_commands::contract(service);
    let schema = &contract.inputs[action];
    let mut injected = Map::new();
    for (key, shape) in schema["properties"]
        .as_object()
        .expect("checked command inputs")
    {
        let mapping = &shape["x-cli-binding"];
        let Some(source) = mapping["source"].as_str() else {
            continue;
        };
        let Some(value) = file.remove(key) else {
            continue;
        };
        let property = mapping["property"].as_str().unwrap_or(key);
        let id = match source {
            "query" => format!("q:{property}"),
            "body" => format!("b:{property}"),
            "envelope" => format!("e:{property}"),
            "nested" => format!("n:{property}"),
            "compose" => format!("compose:{property}"),
            _ => property.to_owned(),
        };
        // Validate the value that survives merging, so an explicit option can
        // replace an invalid or obsolete file value without reading its input.
        if source != "raw" && explicit(args, &id) {
            continue;
        }
        if matches!(source, "control" | "native") {
            schema_commands::validate_body(&contract.document, "", shape, &value, key)?;
        }
        match source {
            "query" | "envelope" => {
                file.insert(property.to_owned(), value);
            }
            "raw" => {
                file.insert("body".into(), value);
            }
            "body" => {
                let body = file.entry("body").or_insert_with(|| json!({}));
                set_path(body, &[property.to_owned()], value)?;
            }
            "nested" => {
                let path: Vec<String> = serde_json::from_value(mapping["path"].clone())?;
                let body = file.entry("body").or_insert_with(|| json!({}));
                set_path(body, &path, value)?;
            }
            "compose" => {
                file.entry("compose")
                    .or_insert_with(|| json!({}))
                    .as_object_mut()
                    .context("compose must be an object")?
                    .insert(property.to_owned(), value);
            }
            "control" | "native" => {
                injected.insert(property.to_owned(), value);
            }
            _ => anyhow::bail!("unknown CLI input binding"),
        }
    }
    let mut effective = args.clone();
    if !injected.is_empty() {
        let command = definition(service, action);
        let mut argv = vec![action.to_owned()];
        // Match the command's positional order, then its named arguments.
        let mut arguments = command.get_arguments().collect::<Vec<_>>();
        arguments.sort_by_key(|arg| (arg.get_long().is_some(), arg.get_index().unwrap_or(0)));
        for arg in arguments {
            let id = arg.get_id().as_str();
            let equals = arg
                .get_num_args()
                .is_some_and(|range| range.min_values() == 0);
            if explicit(args, id) {
                if let Ok(Some(values)) = args.try_get_many::<String>(id) {
                    for value in values {
                        append(&mut argv, arg.get_long(), &json!(value), equals);
                    }
                } else if let Ok(Some(value)) = args.try_get_one::<u32>(id) {
                    append(&mut argv, arg.get_long(), &json!(value), false);
                }
            } else if let Some(value) = injected.get(id) {
                append(&mut argv, arg.get_long(), value, equals);
            }
        }
        effective = command
            .try_get_matches_from(argv)
            .context("invalid command options in --file")?;
    }
    for rule in schema["allOf"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|rule| rule["x-cli-kind"] == "required")
    {
        for key in rule["x-cli-parameters"].as_array().into_iter().flatten() {
            let shape = &schema["properties"][key.as_str().unwrap()];
            if !matches!(
                shape["x-cli-binding"]["source"].as_str(),
                Some("native" | "control")
            ) {
                continue;
            }
            let id = shape["x-cli-binding"]["property"].as_str().unwrap();
            ensure!(
                native::text(&effective, id).is_some(),
                "{}",
                rule["description"].as_str().unwrap()
            );
        }
    }
    Ok((file, effective))
}
