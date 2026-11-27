use clap::Command;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::{schema_commands, services::SERVICES};

fn common(shape: &Value) -> bool {
    if shape["x-cli-advanced"] == true {
        return false;
    }
    let binding = &shape["x-cli-binding"];
    let property = binding["property"].as_str().unwrap_or_default();
    match binding["source"].as_str().unwrap_or_default() {
        "compose" | "native" => true,
        "control" => matches!(property, "max" | "output"),
        "query" => matches!(
            property,
            "q" | "query"
                | "id"
                | "name"
                | "fileId"
                | "calendarId"
                | "eventId"
                | "formId"
                | "documentId"
                | "spreadsheetId"
                | "presentationId"
                | "resourceName"
                | "task"
                | "tasklist"
                | "part"
                | "range"
                | "order"
                | "orderBy"
                | "timeMin"
                | "timeMax"
                | "filter"
                | "mine"
                | "channelId"
                | "playlistId"
                | "videoId"
                | "personFields"
                | "readMask"
                | "sendUpdates"
                | "valueInputOption"
                | "valueRenderOption"
                | "showCompleted"
        ),
        "body" => matches!(
            property,
            "name"
                | "title"
                | "summary"
                | "text"
                | "description"
                | "notes"
                | "due"
                | "status"
                | "ids"
                | "labelIds"
                | "addLabelIds"
                | "removeLabelIds"
                | "parents"
                | "mimeType"
                | "role"
                | "type"
                | "emailAddress"
                | "domain"
                | "content"
                | "values"
                | "requests"
                | "names"
                | "emailAddresses"
                | "attendees"
                | "phoneNumbers"
                | "addresses"
                | "birthdays"
                | "organizations"
                | "memberships"
                | "albumId"
                | "mediaItemIds"
                | "newMediaItems"
                | "items"
                | "fields"
                | "range"
                | "textRange"
        ),
        "nested" => {
            let path = binding["path"].as_array().expect("checked nested input");
            let normal = path
                .iter()
                .map(|part| schema_commands::kebab(part.as_str().unwrap()))
                .collect::<Vec<_>>()
                .join("-");
            shape["x-cli-option"].as_str() != Some(normal.as_str())
                || (path.len() == 2
                    && matches!(path[0].as_str(), Some("start" | "end"))
                    && matches!(path[1].as_str(), Some("date" | "dateTime" | "timeZone")))
                || (path.len() == 2
                    && matches!(path[0].as_str(), Some("textStyle" | "style"))
                    && matches!(path[1].as_str(), Some("bold" | "italic" | "underline")))
        }
        _ => false,
    }
}

pub(crate) fn input_id(shape: &Value) -> String {
    let source = shape["x-cli-binding"]["source"]
        .as_str()
        .unwrap_or_default();
    let property = shape["x-cli-binding"]["property"]
        .as_str()
        .unwrap_or_default();
    match source {
        "query" => format!("q:{property}"),
        "body" => format!("b:{property}"),
        "nested" => format!("n:{property}"),
        "envelope" => format!("e:{property}"),
        "compose" => format!("compose:{property}"),
        _ => property.to_owned(),
    }
}

pub fn register_conflicts(command: Command, schema: &Value) -> Command {
    let ids = command
        .get_arguments()
        .map(|arg| arg.get_id().to_string())
        .collect::<BTreeSet<_>>();
    let mut conflicts = BTreeMap::<String, BTreeSet<String>>::new();
    for rule in schema["allOf"].as_array().into_iter().flatten() {
        let Some(groups) = rule["x-cli-groups"].as_array() else {
            continue;
        };
        for (index, left) in groups.iter().enumerate() {
            for right in groups.iter().skip(index + 1) {
                for a in left.as_array().into_iter().flatten() {
                    for b in right.as_array().into_iter().flatten() {
                        let a = input_id(&schema["properties"][a.as_str().unwrap()]);
                        let b = input_id(&schema["properties"][b.as_str().unwrap()]);
                        if a != b && ids.contains(&a) && ids.contains(&b) {
                            conflicts.entry(a.clone()).or_default().insert(b.clone());
                            conflicts.entry(b).or_default().insert(a);
                        }
                    }
                }
            }
        }
    }
    command.mut_args(|arg| {
        let extra = conflicts.remove(arg.get_id().as_str()).unwrap_or_default();
        arg.conflicts_with_all(extra)
    })
}

pub fn configure(mut command: Command, parent_service: Option<&str>, verbose: bool) -> Command {
    let service = SERVICES
        .iter()
        .find(|service| service.cli == command.get_name())
        .map(|service| service.cli)
        .or(parent_service);
    let name = command.get_name().to_owned();
    let contract = service
        .filter(|service| *service != name)
        .map(schema_commands::contract);
    let definition = contract.and_then(|contract| contract.inputs.get(&name));
    let inputs = definition.and_then(|schema| schema["properties"].as_object());
    let by_id = inputs
        .into_iter()
        .flatten()
        .filter(|(_, shape)| shape.get("x-cli-binding").is_some())
        .map(|(_, shape)| (input_id(shape), shape))
        .collect::<BTreeMap<_, _>>();
    let composition = contract.is_some_and(|contract| contract.compose_paths.contains_key(&name));
    let rules = definition.and_then(|schema| schema["allOf"].as_array());
    let required_keys = rules
        .into_iter()
        .flatten()
        .filter(|rule| rule["x-cli-kind"] == "required")
        .flat_map(|rule| rule["x-cli-parameters"].as_array().into_iter().flatten())
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    let required_ids = inputs
        .into_iter()
        .flatten()
        .filter(|(key, _)| required_keys.contains(key.as_str()))
        .map(|(_, shape)| input_id(shape))
        .collect::<BTreeSet<_>>();
    let conditional_ids = rules
        .into_iter()
        .flatten()
        .filter(|rule| rule["x-cli-kind"] == "choice")
        .flat_map(|rule| rule["x-cli-parameters"].as_array().into_iter().flatten())
        .filter_map(|key| inputs.and_then(|inputs| inputs.get(key.as_str()?)))
        .filter(|shape| shape["x-cli-binding"]["source"] == "body")
        .map(input_id)
        .collect::<BTreeSet<_>>();
    let mut requirements = rules
        .into_iter()
        .flatten()
        .filter(|rule| {
            rule["x-cli-help"] != false
                && matches!(
                    rule["x-cli-kind"].as_str(),
                    Some("choice" | "conflict" | "dependency")
                )
        })
        .filter(|rule| {
            verbose
                || rule["x-cli-parameters"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|key| {
                        inputs
                            .and_then(|inputs| inputs.get(key.as_str().unwrap()))
                            .is_some_and(common)
                    })
        })
        .filter_map(|rule| rule["description"].as_str())
        .collect::<Vec<_>>();
    requirements.dedup();
    let mut about = if verbose {
        command.get_long_about().or(command.get_about())
    } else {
        command.get_about()
    }
    .map(ToString::to_string)
    .unwrap_or_default();
    if !requirements.is_empty() {
        about.push_str("\n\nInput choices:\n");
        about.push_str(
            &requirements
                .iter()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        );
    }
    command = command.long_about(about);
    command = command.mut_args(|arg| {
        let id = arg.get_id().as_str();
        let shape = by_id.get(id).copied();
        let binding = shape.map(|shape| &shape["x-cli-binding"]);
        let source = binding
            .and_then(|binding| binding["source"].as_str())
            .unwrap_or_default();
        let property = binding
            .and_then(|binding| binding["property"].as_str())
            .unwrap_or_default();
        let conditional = conditional_ids.contains(id)
            || source == "body" && composition && matches!(property, "raw" | "message");
        let required = required_ids.contains(id) || arg.is_required_set();
        let advanced = !required
            && (arg.is_hide_set()
                || if matches!(
                    id,
                    "help" | "version" | "verbose" | "json" | "file" | "max" | "output"
                ) {
                    false
                } else if matches!(
                    id,
                    "home" | "timeout" | "fields" | "ttl" | "max-pages" | "body" | "content-type"
                ) {
                    true
                } else if id == "upload" {
                    composition
                } else if conditional
                    || (composition && matches!(source, "body" | "nested"))
                    || (service == Some("forms") && name == "create" && source == "body")
                {
                    true
                } else {
                    shape.is_some_and(|shape| !common(shape))
                });
        let heading = match (required, advanced) {
            (true, false) => "Mandatory parameters",
            (false, false) => "Optional parameters",
            (true, true) => "Mandatory parameters (advanced)",
            (false, true) => "Optional parameters (advanced)",
        };
        let mut text = arg.get_help().map(ToString::to_string).unwrap_or_default();
        if !verbose {
            let choices = text
                .split_once(" Choices: ")
                .map(|(_, choices)| choices.to_owned());
            text = text
                .lines()
                .next()
                .unwrap_or_default()
                .split(". ")
                .next()
                .unwrap_or_default()
                .trim_end_matches('.')
                .to_owned();
            if !text.is_empty() {
                text.push('.');
            }
            if let Some(choices) = choices {
                text.push_str(&format!(" Choices: {choices}"));
            }
        }
        if arg.is_global_set() && id != "verbose" {
            text.push_str(" Place before the command.");
        }
        arg.help_heading(heading)
            .hide(advanced && !verbose)
            .help(text)
    });
    let previous = command
        .get_after_help()
        .map(ToString::to_string)
        .unwrap_or_default();
    let mut footer =
        "Help:\n  Use --help --verbose to see all parameters and full descriptions.".to_owned();
    if command
        .get_arguments()
        .any(|arg| arg.get_long() == Some("file"))
    {
        footer.push_str(
            "\n  Read parameters from --file PATH; explicit options override file values.",
        );
    }
    if !previous.is_empty() {
        footer.push_str("\n\n");
        footer.push_str(&previous);
    }
    command = command.after_help(footer);
    command.mut_subcommands(|child| configure(child, service, verbose))
}

// Clap groups custom headings by insertion order. Put requirements first.
pub fn order_sections(text: &str) -> String {
    let headings = [
        "Mandatory parameters",
        "Optional parameters",
        "Mandatory parameters (advanced)",
        "Optional parameters (advanced)",
    ];
    let mut sections = headings
        .iter()
        .filter_map(|heading| {
            text.find(&format!("\n{heading}:\n"))
                .map(|at| (*heading, at))
        })
        .collect::<Vec<_>>();
    if sections.is_empty() {
        return text.to_owned();
    }
    sections.sort_by_key(|(_, at)| *at);
    let footer = text.find("\nHelp:\n").unwrap_or(text.len());
    let mut result = text[..sections[0].1].to_owned();
    for heading in headings {
        if let Some(index) = sections.iter().position(|(name, _)| *name == heading) {
            let end = sections.get(index + 1).map(|(_, at)| *at).unwrap_or(footer);
            result.push_str(&text[sections[index].1..end]);
        }
    }
    result.push_str(&text[footer..]);
    result
}
