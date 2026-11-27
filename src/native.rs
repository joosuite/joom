use crate::services::Invocation;
use anyhow::{Context, Result, ensure};
use clap::{Arg, ArgMatches, Command};
use serde_json::{Map, Value, json};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

fn value(name: &'static str, label: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .long(name)
        .value_name(label)
        .help(help)
        .value_parser(clap::builder::NonEmptyStringValueParser::new())
}
fn positional(name: &'static str, label: &'static str, help: &'static str) -> Arg {
    Arg::new(name)
        .value_name(label)
        .help(help)
        .value_parser(clap::builder::NonEmptyStringValueParser::new())
}
fn command(name: &'static str, about: &'static str, read: bool, list: bool) -> Command {
    let mut command = Command::new(name)
        .about(about)
        .arg(value(
            "file",
            "PATH",
            "Read command parameters from a JSON file",
        ))
        .arg(value("fields", "MASK", "Select response fields"));
    if read {
        command = command.arg(
            value(
                "ttl",
                "MINUTES",
                "Reuse cached data for MINUTES; 0 disables caching",
            )
            .default_value("0")
            .value_parser(clap::value_parser!(u32)),
        );
    }
    if list {
        command = command
            .arg(
                value(
                    "max",
                    "N",
                    "Return at most N results; by default, return all results",
                )
                .value_parser(clap::value_parser!(u32).range(1..)),
            )
            .arg(
                value(
                    "max-pages",
                    "N",
                    "Stop after N pages; by default, fetch all pages",
                )
                .value_parser(clap::value_parser!(u32).range(1..=10000)),
            );
    }
    command
}
pub fn commands(service: &str) -> Vec<Command> {
    let commands = match service {
        "mail" => [
            ("read", "Mark a message as read"),
            ("unread", "Mark a message as unread"),
            ("star", "Star a message"),
            ("unstar", "Remove a message's star"),
        ]
        .into_iter()
        .map(|(name, about)| {
            command(name, about, false, false).arg(positional("id", "MESSAGE_ID", "Message ID"))
        })
        .collect(),
        "drive" => vec![
            command("search", "Search file contents", true, true)
                .arg(positional("query", "TEXT", "Search text"))
                .arg(value("parent", "ID", "Search inside this folder"))
                .arg(value("type", "MIME", "Filter by MIME type")),
        ],
        "calendar" => vec![
            command("events", "List events in chronological order", true, true)
                .arg(value("calendar", "ID", "Calendar ID").default_value("primary"))
                .arg(value(
                    "from",
                    "TIMESTAMP",
                    "RFC 3339 lower bound for event end times; defaults to now",
                ))
                .arg(value(
                    "until",
                    "TIMESTAMP",
                    "RFC 3339 upper bound for event start times; defaults to 30 days after --from",
                ))
                .arg(value("query", "TEXT", "Search event text")),
        ],
        _ => Vec::new(),
    };
    commands
        .into_iter()
        .map(|command| {
            let schema = &crate::schema_commands::contract(service).inputs[command.get_name()];
            let required = schema["allOf"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|rule| rule["x-cli-kind"] == "required")
                .flat_map(|rule| rule["x-cli-parameters"].as_array().into_iter().flatten())
                .filter_map(|key| {
                    schema["properties"][key.as_str().unwrap()]["x-cli-binding"]["property"]
                        .as_str()
                })
                .collect::<std::collections::BTreeSet<_>>();
            let command = command.mut_args(|arg| {
                if required.contains(arg.get_id().as_str()) {
                    arg.required_unless_present("file")
                } else {
                    arg
                }
            });
            let mut preview = command.clone().bin_name(format!(
                "joom [OPTIONS] {service} <ACCOUNT> {}",
                command.get_name()
            ));
            let usage = preview
                .render_usage()
                .to_string()
                .trim_start_matches("Usage: ")
                .replace(
                    &format!("{} [OPTIONS]", command.get_name()),
                    &format!("{} [COMMAND_OPTIONS]", command.get_name()),
                );
            command.override_usage(usage)
        })
        .collect()
}
pub fn arguments(args: &ArgMatches) -> &ArgMatches {
    args.subcommand().map_or(args, |(_, child)| child)
}

pub fn text<'a>(args: &'a ArgMatches, name: &str) -> Option<&'a str> {
    args.try_get_one::<String>(name)
        .ok()
        .flatten()
        .map(String::as_str)
}

fn set(params: &mut Map<String, Value>, key: &str, args: &ArgMatches, flag: &str) {
    if let Some(value) = text(args, flag)
        && (args.value_source(flag) == Some(clap::parser::ValueSource::CommandLine)
            || !params.contains_key(key))
    {
        params.insert(key.into(), json!(value));
    }
}

fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "\\'")
}

fn timestamp(value: &str) -> Result<()> {
    OffsetDateTime::parse(value, &Rfc3339)
        .context("use an RFC 3339 timestamp with a time zone offset")?;
    Ok(())
}

pub fn resolve(
    service: &str,
    _account: &str,
    action: &str,
    args: &ArgMatches,
    mut params: Map<String, Value>,
) -> Result<Invocation> {
    let custom_fields = params.contains_key("fields") || text(args, "fields").is_some();
    let mut body = None;
    let mut relative_times = Vec::new();
    let method = match (service, action) {
        ("mail", "read" | "unread" | "star" | "unstar") => {
            params.insert(
                "id".into(),
                json!(text(args, "id").context("missing message ID")?),
            );
            body = Some(match action {
                "read" => json!({"removeLabelIds":["UNREAD"]}),
                "unread" => json!({"addLabelIds":["UNREAD"]}),
                "star" => json!({"addLabelIds":["STARRED"]}),
                _ => json!({"removeLabelIds":["STARRED"]}),
            });
            "messages.modify"
        }
        ("drive", "search") => {
            let mut clauses = vec!["trashed = false".to_owned()];
            if let Some(parent) = text(args, "parent") {
                clauses.push(format!("'{}' in parents", quoted(parent)));
            }
            if let Some(kind) = text(args, "type") {
                clauses.push(format!("mimeType = '{}'", quoted(kind)));
            }
            if let Some(query) = text(args, "query") {
                clauses.push(format!("fullText contains '{}'", quoted(query)));
            }
            if let Some(query) = params.remove("q") {
                ensure!(query.is_string(), "q must be a string");
                clauses.push(format!("({})", query.as_str().unwrap()));
            }
            params.insert("q".into(), json!(clauses.join(" and ")));
            params.entry("fields").or_insert(json!(
                "nextPageToken,files(id,name,mimeType,size,modifiedTime,webViewLink)"
            ));
            "files.list"
        }
        ("calendar", "events") => {
            set(&mut params, "calendarId", args, "calendar");
            params.entry("orderBy").or_insert(json!("startTime"));
            for (flag, key) in [("from", "timeMin"), ("until", "timeMax")] {
                if let Some(value) = text(args, flag) {
                    timestamp(value)?;
                    params.insert(key.into(), json!(value));
                }
            }
            if !params.contains_key("timeMin") {
                relative_times.push("timeMin");
                let now = OffsetDateTime::now_utc();
                params.insert("timeMin".into(), json!(now.format(&Rfc3339)?));
            }
            if !params.contains_key("timeMax") {
                relative_times.push("timeMax");
                let start = params["timeMin"]
                    .as_str()
                    .context("timeMin must be a timestamp string")?;
                let end = OffsetDateTime::parse(start, &Rfc3339)?
                    .checked_add(time::Duration::days(30))
                    .context("calendar window is outside the supported date range")?;
                params.insert("timeMax".into(), json!(end.format(&Rfc3339)?));
            }
            if let (Some(start), Some(end)) = (
                params.get("timeMin").and_then(Value::as_str),
                params.get("timeMax").and_then(Value::as_str),
            ) {
                ensure!(
                    OffsetDateTime::parse(start, &Rfc3339)? < OffsetDateTime::parse(end, &Rfc3339)?,
                    "--until must be later than --from"
                );
            }
            set(&mut params, "q", args, "query");
            "events.list"
        }
        _ => anyhow::bail!("unsupported command {service} {action}"),
    };
    if let Some(fields) = text(args, "fields") {
        params.insert("fields".into(), json!(fields));
    }
    Ok(Invocation {
        options: None,
        method: method.into(),
        params,
        body,
        custom_fields,
        relative_times,
    })
}

#[cfg(test)]
#[path = "../tests/unit/native.rs"]
mod tests;
