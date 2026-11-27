use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use clap::{Arg, ArgAction, ArgMatches, builder::NonEmptyStringValueParser};
use mail_builder::MessageBuilder;
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::path::Path;

use crate::{http, schema_commands};

// Composition inputs are converted to MIME before Microsoft Graph submission.
pub fn options(schema: &Value) -> Vec<Arg> {
    schema["properties"]
        .as_object()
        .expect("checked composition schema")
        .iter()
        .map(|(name, shape)| {
            let long = shape["x-cli-option"]
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| schema_commands::kebab(name));
            let mut arg = Arg::new(format!("compose:{name}"))
                .long(long)
                .value_name(
                    shape["x-cli-value-name"]
                        .as_str()
                        .unwrap_or("TEXT")
                        .to_owned(),
                )
                .help(shape["description"].as_str().unwrap_or_default().to_owned());
            if shape["type"] == "array" {
                arg = arg
                    .action(ArgAction::Append)
                    .num_args(1..)
                    .value_parser(NonEmptyStringValueParser::new());
            }
            arg
        })
        .collect()
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct Composition {
    to: Vec<String>,
    cc: Vec<String>,
    bcc: Vec<String>,
    from: Option<String>,
    reply_to: Option<String>,
    subject: Option<String>,
    text: Option<String>,
    html: Option<String>,
    text_file: Option<String>,
    html_file: Option<String>,
    attachments: Vec<String>,
}

fn mailbox(value: &str) -> Result<()> {
    let valid = value.split_once('@').is_some_and(|(local, domain)| {
        !local.is_empty()
            && local.len() <= 64
            && !local.starts_with('.')
            && !local.ends_with('.')
            && !local.contains("..")
            && local
                .bytes()
                .all(|ch| ch.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&ch))
            && !domain.is_empty()
            && domain.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && !label.starts_with('-')
                    && !label.ends_with('-')
                    && label
                        .bytes()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-')
            })
    });
    ensure!(
        value.len() <= 254 && valid,
        "use a bare ASCII email address, one address per recipient value"
    );
    Ok(())
}

fn read_file(path: &str, remaining: &mut u64) -> Result<Vec<u8>> {
    let file = std::fs::File::open(path).context("cannot open composition file")?;
    ensure!(
        file.metadata()?.is_file(),
        "composition input must be a regular file"
    );
    let bytes = http::limited(file, *remaining)
        .context("cannot read composition file within the 16 MiB input limit")?;
    *remaining -= bytes.len() as u64;
    Ok(bytes)
}

fn content(
    inline: Option<String>,
    file: Option<String>,
    remaining: &mut u64,
) -> Result<Option<String>> {
    ensure!(
        inline.is_none() || file.is_none(),
        "choose inline text or its file input for each body format"
    );
    if let Some(path) = file {
        return String::from_utf8(read_file(&path, remaining)?)
            .context("mail body files must be UTF-8")
            .map(Some);
    }
    if let Some(text) = &inline {
        ensure!(
            text.len() as u64 <= *remaining,
            "composition input exceeds 16 MiB"
        );
        *remaining -= text.len() as u64;
    }
    Ok(inline)
}

pub(crate) fn merge_input(
    args: &ArgMatches,
    file: Option<Value>,
    schema: &Value,
) -> Result<Option<Value>> {
    let mut input = match file {
        Some(Value::Object(input)) => Some(input),
        Some(_) => anyhow::bail!("compose must be an object"),
        None => None,
    };
    for (name, shape) in schema["properties"].as_object().into_iter().flatten() {
        if let Some(values) = args
            .try_get_many::<String>(&format!("compose:{name}"))
            .ok()
            .flatten()
        {
            let values = values.cloned().collect::<Vec<_>>();
            let input = input.get_or_insert_with(Map::new);
            // Explicit text flags replace the other representation from a file.
            for group in schema["x-exclusive-fields"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let fields = group.as_array().expect("checked composition alternatives");
                if fields.iter().any(|field| field == name) {
                    for other in fields
                        .iter()
                        .filter_map(Value::as_str)
                        .filter(|other| *other != name)
                    {
                        input.remove(other);
                    }
                }
            }
            input.insert(
                name.clone(),
                if shape["type"] == "array" {
                    json!(values)
                } else {
                    json!(values[0])
                },
            );
        }
    }
    Ok(input.map(Value::Object))
}

pub fn compose(
    account: &str,
    _method: &str,
    args: &ArgMatches,
    file: Option<Value>,
    schema: &Value,
    path: &[String],
    body: &mut Option<Value>,
) -> Result<()> {
    let input = merge_input(args, file, schema)?;
    let Some(input) = input else {
        return Ok(());
    };
    let doc = &schema_commands::contract("mail").document;
    schema_commands::validate_body(doc, "", schema, &input, "compose")?;
    let input: Composition = serde_json::from_value(input).context("invalid compose input")?;
    let sender = input.from.as_deref().unwrap_or(account);
    for address in input
        .to
        .iter()
        .chain(&input.cc)
        .chain(&input.bcc)
        .map(String::as_str)
        .chain(std::iter::once(sender))
        .chain(input.reply_to.as_deref())
    {
        mailbox(address)?;
    }
    if let Some(subject) = &input.subject {
        ensure!(
            !subject.chars().any(char::is_control),
            "mail subject cannot contain control characters"
        );
    }
    let message = body
        .get_or_insert_with(|| json!({}))
        .as_object_mut()
        .context("compose requires an object body")?;
    let mut target = message;
    let (raw, parents) = path.split_last().context("missing composition target")?;
    for key in parents {
        target = target
            .entry(key)
            .or_insert_with(|| json!({}))
            .as_object_mut()
            .context("compose target must be an object")?;
    }
    let mut remaining = http::DISCOVERY_LIMIT;
    let text = content(input.text, input.text_file, &mut remaining)?;
    let html = content(input.html, input.html_file, &mut remaining)?;
    let mut message = MessageBuilder::new().from(sender.to_owned());
    if !input.to.is_empty() {
        message = message.to(input.to);
    }
    if !input.cc.is_empty() {
        message = message.cc(input.cc);
    }
    if !input.bcc.is_empty() {
        message = message.bcc(input.bcc);
    }
    if let Some(reply_to) = input.reply_to {
        message = message.reply_to(reply_to);
    }
    if let Some(subject) = input.subject {
        message = message.subject(subject);
    }
    if let Some(text) = text {
        message = message.text_body(text);
    } else if html.is_none() {
        message = message.text_body("");
    }
    if let Some(html) = html {
        message = message.html_body(html);
    }
    for attachment in input.attachments {
        let filename = Path::new(&attachment)
            .file_name()
            .and_then(|name| name.to_str())
            .context("attachment needs a UTF-8 filename")?;
        ensure!(
            !filename.chars().any(char::is_control),
            "attachment filename cannot contain control characters"
        );
        let bytes = read_file(&attachment, &mut remaining)?;
        message = message.attachment("application/octet-stream", filename.to_owned(), bytes);
    }
    let mime = message
        .write_to_vec()
        .context("cannot build MIME message")?;
    ensure!(
        mime.len().div_ceil(3) * 4 <= http::DISCOVERY_LIMIT as usize,
        "encoded mail exceeds the 16 MiB JSON request limit; use --upload with an .eml file for larger mail"
    );
    target.insert(raw.clone(), json!(URL_SAFE_NO_PAD.encode(mime)));
    Ok(())
}
