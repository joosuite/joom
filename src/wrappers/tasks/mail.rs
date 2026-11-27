use super::{Backend, Request, changed, id, rows, values};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use mail_builder::MessageBuilder;
use mail_parser::MessageParser;
use serde_json::{Value, json};

fn addresses(address: Option<&mail_parser::Address<'_>>) -> Vec<String> {
    address
        .into_iter()
        .flat_map(|address| address.iter())
        .filter_map(|address| address.address.as_deref())
        .map(str::to_owned)
        .collect()
}

fn message(value: Value) -> Result<Value> {
    let mut result = crate::presentation::plain_value("mail", "get", &value)?;
    if let Some(thread) = value.get("threadId") {
        result["threadId"] = thread.clone();
    }
    fn attachments(part: &Value, result: &mut Vec<Value>) {
        if part["filename"]
            .as_str()
            .is_some_and(|name| !name.is_empty())
        {
            result.push(json!({"name":part["filename"],"type":part["mimeType"],"size":part.pointer("/body/size"),"id":part.pointer("/body/attachmentId")}));
        }
        for child in rows(part, "parts") {
            attachments(child, result);
        }
    }
    let mut items = Vec::new();
    attachments(&value["payload"], &mut items);
    result["attachments"] = json!(items);
    Ok(result)
}

fn composition(r: &Request) -> Value {
    let mut params = json!({});
    for name in ["to", "cc", "bcc", "subject"] {
        if let Some(value) = r.params.get(name) {
            params[name] = value.clone();
        }
    }
    if let Some(value) = r.params.get("body") {
        params["text"] = value.clone();
    }
    if let Some(value) = r.params.get("bodyFile") {
        params["textFile"] = value.clone();
    }
    if let Some(value) = r.params.get("attachments") {
        params["attach"] = value.clone();
    }
    if params.get("text").is_none() && params.get("textFile").is_none() {
        params["text"] = json!("");
    }
    params
}

pub fn execute(r: &Request, api: &mut impl Backend) -> Result<Value> {
    match r.command.as_str() {
        "inbox" | "find" => {
            let mut query = if r.command == "inbox" {
                "in:inbox".to_owned()
            } else {
                r.text("query").unwrap().to_owned()
            };
            if r.flag("unread") {
                query.push_str(" is:unread");
            }
            if let Some(from) = r.text("from") {
                query.push_str(&format!(" from:{from}"));
            }
            if let Some(after) = r.text("after") {
                query.push_str(&format!(
                    " after:{}",
                    values::date(after, jiff::ZonedDateTime::now().date())?.strftime("%Y/%m/%d")
                ));
            }
            if r.text("has").is_some() {
                query.push_str(" has:attachment");
            }
            let value = api.call("mail", "messages.list", r.list_params(json!({"q":query})))?;
            let mut messages = Vec::new();
            for item in rows(&value, "messages") {
                let full = api.call("mail", "messages.get", json!({"id":id(item,"id")?}))?;
                messages.push(message(full)?);
            }
            Ok(json!(messages))
        }
        "read" => message(api.call("mail", "messages.get", json!({"id":r.target()}))?),
        "send" | "draft" => Ok(changed(
            api.call(
                "mail",
                if r.command == "send" {
                    "messages.send"
                } else {
                    "drafts.create"
                },
                composition(r),
            )?,
            if r.command == "send" {
                "Message sent."
            } else {
                "Draft saved."
            },
        )),
        "drafts" => {
            let value = api.call("mail", "drafts.list", r.list_params(json!({})))?;
            let mut drafts = Vec::new();
            for draft in rows(&value, "drafts") {
                let full = api.call("mail", "drafts.get", json!({"id":id(draft,"id")?}))?;
                let mut item = message(full["message"].clone())?;
                item["draftId"] = draft["id"].clone();
                drafts.push(item);
            }
            Ok(json!(drafts))
        }
        "send-draft" => Ok(changed(
            api.call("mail", "drafts.send", json!({"id":r.target()}))?,
            "Draft sent.",
        )),
        "trash" => Ok(changed(
            api.call("mail", "messages.trash", json!({"id":r.target()}))?,
            "Message moved to Trash.",
        )),
        "mark" => {
            let (key, label) = match r.text("state").unwrap() {
                "read" => ("removeLabelIds", "UNREAD"),
                "unread" => ("addLabelIds", "UNREAD"),
                "starred" => ("addLabelIds", "STARRED"),
                "unstarred" => ("removeLabelIds", "STARRED"),
                _ => unreachable!(),
            };
            let mut params = json!({"id":r.target()});
            params[key] = json!([label]);
            Ok(changed(
                api.call("mail", "messages.modify", params)?,
                "Message updated.",
            ))
        }
        "reply" | "forward" => {
            let original = api.call(
                "mail",
                "messages.get",
                json!({"id":r.target(),"format":"raw"}),
            )?;
            let bytes = URL_SAFE_NO_PAD
                .decode(id(&original, "raw")?.trim_end_matches('='))
                .context("Microsoft returned invalid MIME encoding")?;
            let parsed = MessageParser::default()
                .parse(&bytes)
                .context("cannot parse the original message")?;
            let subject = parsed.subject().unwrap_or_default();
            let subject = if r.command == "reply" {
                if subject.to_lowercase().starts_with("re:") {
                    subject.to_owned()
                } else {
                    format!("Re: {subject}")
                }
            } else {
                format!("Fwd: {subject}")
            };
            let mut to = if r.command == "reply" {
                addresses(parsed.reply_to().or_else(|| parsed.from()))
            } else {
                r.strings("to")
            };
            let mut cc = Vec::new();
            if r.command == "reply" && r.flag("all") {
                to.extend(addresses(parsed.to()));
                cc = addresses(parsed.cc());
            }
            if r.command == "reply" {
                to.retain(|email| !email.eq_ignore_ascii_case(&r.account));
                cc.retain(|email| {
                    !email.eq_ignore_ascii_case(&r.account)
                        && !to.iter().any(|to| to.eq_ignore_ascii_case(email))
                });
            }
            let dedup = |values: &mut Vec<String>| {
                let mut seen = std::collections::BTreeSet::new();
                values.retain(|value| seen.insert(value.to_lowercase()));
            };
            dedup(&mut to);
            dedup(&mut cc);
            ensure!(
                !to.is_empty() || !cc.is_empty(),
                "the original message has no reply recipient other than your account"
            );
            for email in to.iter().chain(&cc) {
                values::format("email", email)?;
            }
            let mut builder = MessageBuilder::new()
                .from(r.account.clone())
                .subject(subject)
                .text_body(values::text(&r.params, "body")?.unwrap_or_default());
            if !to.is_empty() {
                builder = builder.to(to);
            }
            if !cc.is_empty() {
                builder = builder.cc(cc);
            }
            if r.command == "reply" {
                let message_id = parsed
                    .message_id()
                    .context("the original message has no Message-ID for a threaded reply")?;
                builder = builder.in_reply_to(message_id.to_owned());
                let mut references = parsed
                    .references()
                    .as_text_list()
                    .unwrap_or_default()
                    .iter()
                    .map(|text| text.to_string())
                    .collect::<Vec<_>>();
                references.push(message_id.to_owned());
                builder = builder.references(references);
            } else {
                builder = builder.attachment("message/rfc822", "forwarded.eml", bytes.clone());
            }
            let mut remaining = crate::http::DISCOVERY_LIMIT;
            for path in r.strings("attachments") {
                let data = crate::http::limited(std::fs::File::open(&path)?, remaining)?;
                remaining = remaining.saturating_sub(data.len() as u64);
                let name = std::path::Path::new(&path)
                    .file_name()
                    .context("attachment has no filename")?
                    .to_string_lossy()
                    .into_owned();
                builder = builder.attachment(values::upload_mime(&path), name, data);
            }
            let mime = builder.write_to_vec()?;
            ensure!(
                mime.len().div_ceil(3) * 4 <= crate::http::DISCOVERY_LIMIT as usize,
                "encoded message exceeds 16 MiB"
            );
            let mut params = json!({"raw":URL_SAFE_NO_PAD.encode(mime)});
            if r.command == "reply" {
                params["threadId"] = json!(id(&original, "threadId")?);
            }
            Ok(changed(
                api.call("mail", "messages.send", params)?,
                "Message sent.",
            ))
        }
        _ => anyhow::bail!("unknown Mail command"),
    }
}
