//! Microsoft Graph adapter for the unchanged command and response contracts.
//!
//! The bundled method IDs are compatibility identifiers, never network targets.
//! Unsupported operations fail before credentials are read or changes are sent.
use anyhow::{Context, Result, bail, ensure};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use reqwest::Method;
use serde_json::{Map, Value, json};
use url::Url;

use crate::{
    discovery::{Document, MethodSpec},
    http::{self, Http},
};

const ROOT: &str = "https://graph.microsoft.com/v1.0";
mod windows_zones;

fn iana_zone(zone: &str) -> &str {
    windows_zones::WINDOWS_TO_IANA
        .iter()
        .find(|(windows, _)| windows.eq_ignore_ascii_case(zone))
        .map(|(_, iana)| *iana)
        .unwrap_or(zone)
}

fn mailbox_zone(settings: &Value) -> Result<&str> {
    settings
        .get("timeZone")
        .and_then(Value::as_str)
        .or_else(|| settings.get("value").and_then(Value::as_str))
        .or_else(|| settings.as_str())
        .filter(|zone| !zone.is_empty())
        .context("Microsoft did not return the mailbox timezone")
}

pub fn graph_url(url: &Url) -> Result<()> {
    ensure!(
        url.scheme() == "https"
            && url.host_str() == Some("graph.microsoft.com")
            && url.username().is_empty()
            && url.password().is_none()
            && url.port_or_known_default() == Some(443)
            && url.fragment().is_none()
            && url.path().starts_with("/v1.0/"),
        "refusing a request outside https://graph.microsoft.com/v1.0/"
    );
    Ok(())
}

fn collection_path(url: &Url) -> Result<Vec<String>> {
    let decode = |value: &str| {
        percent_encoding::percent_decode_str(value)
            .decode_utf8()
            .map(|value| value.into_owned())
            .context("invalid Microsoft collection path")
    };
    let mut path = Vec::new();
    for segment in url
        .path_segments()
        .context("invalid Microsoft collection URL")?
    {
        // Graph nextLink can spell /mailFolders/inbox as /mailFolders('inbox').
        // Recognize delimiters before decoding so encoded resource IDs stay one key.
        if let Some((name, key)) = segment.split_once("('")
            && !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            && let Some(key) = key.strip_suffix("')")
        {
            let key = decode(key)?;
            let mut literal = String::new();
            let mut chars = key.chars();
            while let Some(ch) = chars.next() {
                if ch == '\'' {
                    ensure!(
                        chars.next() == Some('\''),
                        "invalid Microsoft collection key"
                    );
                }
                literal.push(ch);
            }
            path.push(name.to_owned());
            path.push(literal);
        } else {
            path.push(decode(segment)?);
        }
    }
    Ok(path)
}

fn collection_page(collection: &Url, next: &str) -> Result<Url> {
    let next = Url::parse(next).context("invalid Microsoft page token")?;
    graph_url(&next)?;
    ensure!(
        collection_path(&next)? == collection_path(collection)?,
        "Microsoft page token belongs to another collection"
    );
    Ok(next)
}

fn segment(value: &str) -> Result<String> {
    ensure!(
        !value.is_empty() && value != "." && value != "..",
        "invalid Microsoft resource ID"
    );
    Ok(
        percent_encoding::utf8_percent_encode(value, percent_encoding::NON_ALPHANUMERIC)
            .to_string(),
    )
}

fn text<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}
fn required(params: &Map<String, Value>, key: &str) -> Result<String> {
    segment(text(params, key).with_context(|| format!("missing {key}"))?)
}
fn unsupported(id: &str) -> anyhow::Error {
    anyhow::anyhow!("{id} has no implemented Microsoft Graph equivalent; see docs/microsoft.md")
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shape {
    Message,
    Draft,
    Label,
    File,
    Calendar,
    Event,
    TaskList,
    Task,
    Contact,
    Permission,
    Revision,
    Workbook,
    Range,
    Identity,
}

pub struct Request {
    method: Method,
    url: Url,
    body: Option<Value>,
    shape: Shape,
    collection: Option<&'static str>,
    mime: Option<String>,
    filters: Vec<(String, String)>,
    etag: Option<String>,
}

impl Request {
    fn new(
        method: Method,
        path: &str,
        body: Option<Value>,
        shape: Shape,
        collection: Option<&'static str>,
    ) -> Result<Self> {
        let url = Url::parse(&format!("{ROOT}/{path}"))?;
        graph_url(&url)?;
        Ok(Self {
            method,
            url,
            body,
            shape,
            collection,
            mime: None,
            filters: Vec::new(),
            etag: None,
        })
    }
}

fn copy(source: &Value, target: &mut Value, mappings: &[(&str, &str)]) {
    for (from, to) in mappings {
        if let Some(value) = source.get(*from) {
            target[*to] = value.clone();
        }
    }
}

fn write_body(shape: Shape, body: &Value) -> Result<Value> {
    let allowed: &[&str] = match shape {
        Shape::File => &["name", "description", "mimeType", "parents", "trashed"],
        Shape::Calendar => &["summary"],
        Shape::Event => &[
            "summary",
            "description",
            "location",
            "start",
            "end",
            "attendees",
            "transparency",
            "reminders",
            "recurrence",
        ],
        Shape::TaskList => &["title"],
        Shape::Task => &[
            "title",
            "notes",
            "status",
            "due",
            "completed",
            "parent",
            "position",
        ],
        Shape::Contact => &[
            "names",
            "emailAddresses",
            "phoneNumbers",
            "organizations",
            "biographies",
            "etag",
            "resourceName",
            "metadata",
        ],
        Shape::Label => &["name"],
        Shape::Range => &["values", "range", "majorDimension"],
        _ => &[],
    };
    if !allowed.is_empty() {
        for key in body
            .as_object()
            .context("request body must be an object")?
            .keys()
        {
            ensure!(
                allowed.contains(&key.as_str()),
                "{key} has no implemented Microsoft field mapping"
            );
        }
    }
    let mut out = json!({});
    match shape {
        Shape::File => {
            copy(
                body,
                &mut out,
                &[("name", "name"), ("description", "description")],
            );
            if body["mimeType"] == "inode/directory" {
                out["folder"] = json!({});
            }
            if let Some(parent) = body["parents"].as_array().and_then(|a| a.first()) {
                out["parentReference"] = json!({"id": parent});
            }
            ensure!(
                body.get("trashed").is_none(),
                "use files.delete to move a OneDrive item to the recycle bin; restoring items is unavailable through this command"
            );
        }
        Shape::Calendar => {
            copy(body, &mut out, &[("summary", "name")]);
        }
        Shape::Event => {
            copy(
                body,
                &mut out,
                &[("summary", "subject"), ("transparency", "showAs")],
            );
            if let Some(description) = body.get("description") {
                out["body"] = json!({"contentType":"text","content":description});
            }
            if let Some(location) = body.get("location") {
                out["location"] = json!({"displayName":location});
            }
            for key in ["start", "end"] {
                if let Some(value) = body.get(key) {
                    let date = value["dateTime"]
                        .as_str()
                        .or_else(|| value["date"].as_str())
                        .context("event dates require date or dateTime")?;
                    let all_day = value.get("date").is_some();
                    out[key] = json!({"dateTime":if all_day {format!("{date}T00:00:00")} else {date.into()}, "timeZone":value["timeZone"].as_str().unwrap_or("UTC")});
                    if all_day {
                        out["isAllDay"] = json!(true);
                    }
                }
            }
            if let Some(attendees) = body["attendees"].as_array() {
                out["attendees"] = json!(attendees.iter().map(|a| json!({"emailAddress":{"address":a["email"],"name":a["displayName"]},"type":if a["optional"] == true {"optional"} else {"required"}})).collect::<Vec<_>>());
            }
            ensure!(
                body.get("recurrence").is_none(),
                "RFC5545 recurrence cannot be translated losslessly to Microsoft Graph; edit recurrence in Outlook"
            );
            if body.get("transparency").is_some() {
                out["showAs"] = json!(if body["transparency"] == "transparent" {
                    "free"
                } else {
                    "busy"
                });
            }
            if let Some(reminders) = body.get("reminders") {
                if let Some(minutes) = reminders["overrides"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|a| a.get("minutes"))
                {
                    out["isReminderOn"] = json!(true);
                    out["reminderMinutesBeforeStart"] = minutes.clone();
                } else if reminders["useDefault"] == false {
                    out["isReminderOn"] = json!(false);
                }
            }
        }
        Shape::TaskList => copy(body, &mut out, &[("title", "displayName")]),
        Shape::Task => {
            copy(body, &mut out, &[("title", "title")]);
            if let Some(value) = body.get("notes") {
                out["body"] = json!({"contentType":"text","content":value});
            }
            if let Some(value) = body.get("status") {
                out["status"] = json!(if value == "completed" {
                    "completed"
                } else {
                    "notStarted"
                });
            }
            for (from, to) in [("due", "dueDateTime"), ("completed", "completedDateTime")] {
                if let Some(value) = body.get(from) {
                    out[to] = if value.is_null() {
                        Value::Null
                    } else {
                        json!({"dateTime":value,"timeZone":"UTC"})
                    };
                }
            }
            ensure!(
                body.get("parent").is_none() && body.get("position").is_none(),
                "Microsoft To Do does not support task hierarchy or positions"
            );
        }
        Shape::Contact => {
            if let Some(name) = body["names"].as_array().and_then(|a| a.first()) {
                if let Some(name) = name.get("unstructuredName") {
                    out["displayName"] = name.clone();
                }
                copy(
                    name,
                    &mut out,
                    &[
                        ("givenName", "givenName"),
                        ("familyName", "surname"),
                        ("displayName", "displayName"),
                    ],
                );
            }
            if let Some(emails) = body["emailAddresses"].as_array() {
                out["emailAddresses"] = json!(
                    emails
                        .iter()
                        .map(|e| json!({"address":e["value"]}))
                        .collect::<Vec<_>>()
                );
            }
            if let Some(phones) = body["phoneNumbers"].as_array() {
                out["businessPhones"] = json!(
                    phones
                        .iter()
                        .map(|p| p["value"].clone())
                        .collect::<Vec<_>>()
                );
            }
            if let Some(org) = body["organizations"].as_array().and_then(|a| a.first()) {
                copy(
                    org,
                    &mut out,
                    &[
                        ("name", "companyName"),
                        ("title", "jobTitle"),
                        ("department", "department"),
                    ],
                );
            }
            if let Some(note) = body["biographies"].as_array().and_then(|a| a.first()) {
                out["personalNotes"] = note["value"].clone();
            }
        }
        Shape::Label => copy(body, &mut out, &[("name", "displayName")]),
        Shape::Range => {
            ensure!(
                body["majorDimension"]
                    .as_str()
                    .is_none_or(|value| value == "ROWS"),
                "Microsoft range writes require majorDimension ROWS"
            );
            copy(body, &mut out, &[("values", "values")]);
            if let Some(rows) = out["values"].as_array_mut() {
                let width = rows
                    .iter()
                    .filter_map(Value::as_array)
                    .map(Vec::len)
                    .max()
                    .unwrap_or(0);
                ensure!(
                    rows.len()
                        .checked_mul(width)
                        .is_some_and(|cells| cells <= 16 * 1024 * 1024 / 5),
                    "Excel grid is too large"
                );
                for row in rows {
                    if let Some(row) = row.as_array_mut() {
                        row.resize(width, Value::Null);
                    }
                }
            }
        }
        _ => out = body.clone(),
    }
    Ok(out)
}

fn items(value: &Value) -> &[Value] {
    value["value"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn response(shape: Shape, value: &Value) -> Value {
    if value.is_null() {
        return Value::Null;
    }
    let mut out = json!({"id":value["id"]});
    match shape {
        Shape::File => {
            copy(
                value,
                &mut out,
                &[
                    ("name", "name"),
                    ("size", "size"),
                    ("webUrl", "webViewLink"),
                    ("createdDateTime", "createdTime"),
                    ("lastModifiedDateTime", "modifiedTime"),
                    ("description", "description"),
                ],
            );
            out["mimeType"] = if value.get("folder").is_some() {
                json!("inode/directory")
            } else {
                value["file"]["mimeType"].clone()
            };
            if let Some(parent) = value["parentReference"].get("id") {
                out["parents"] = json!([parent]);
            }
            out["trashed"] = json!(value.get("deleted").is_some());
        }
        Shape::Calendar => {
            copy(
                value,
                &mut out,
                &[("name", "summary"), ("canEdit", "canEdit")],
            );
            out["primary"] = value["isDefaultCalendar"].clone();
        }
        Shape::Event => {
            copy(
                value,
                &mut out,
                &[
                    ("subject", "summary"),
                    ("webLink", "htmlLink"),
                    ("createdDateTime", "created"),
                    ("lastModifiedDateTime", "updated"),
                    ("iCalUId", "iCalUID"),
                ],
            );
            out["description"] = value["body"]["content"].clone();
            out["location"] = value["location"]["displayName"].clone();
            out["status"] = json!(if value["isCancelled"] == true {
                "cancelled"
            } else {
                "confirmed"
            });
            for key in ["start", "end"] {
                let date = value[key]["dateTime"].as_str().unwrap_or_default();
                out[key] = if value["isAllDay"] == true {
                    json!({"date":date.get(..10).unwrap_or(date)})
                } else {
                    json!({"dateTime": if value[key]["timeZone"] == "UTC" && !date.ends_with('Z') { format!("{date}Z") } else { date.to_owned() },"timeZone":value[key]["timeZone"]})
                };
            }
            out["attendees"] = json!(value["attendees"].as_array().into_iter().flatten().map(|a| json!({"email":a["emailAddress"]["address"],"displayName":a["emailAddress"]["name"],"optional":a["type"] == "optional","responseStatus":match a["status"]["response"].as_str() {Some("accepted")=>"accepted",Some("declined")=>"declined",Some("tentativelyAccepted")=>"tentative",_=>"needsAction"}})).collect::<Vec<_>>());
        }
        Shape::TaskList => {
            out["title"] = value["displayName"].clone();
        }
        Shape::Task => {
            copy(
                value,
                &mut out,
                &[("title", "title"), ("lastModifiedDateTime", "updated")],
            );
            out["notes"] = value["body"]["content"].clone();
            out["status"] = json!(if value["status"] == "completed" {
                "completed"
            } else {
                "needsAction"
            });
            out["due"] = value["dueDateTime"]["dateTime"].clone();
            out["completed"] = value["completedDateTime"]["dateTime"].clone();
        }
        Shape::Contact => {
            out["resourceName"] = json!(format!(
                "people/{}",
                value["id"].as_str().unwrap_or_default()
            ));
            out["etag"] = value["@odata.etag"].clone();
            out["metadata"] = json!({"sources":[{"id":value["id"],"type":"CONTACT","etag":value["@odata.etag"]}]});
            out["names"] = json!([{"givenName":value["givenName"],"familyName":value["surname"],"displayName":value["displayName"]}]);
            out["emailAddresses"] = json!(
                value["emailAddresses"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(|e| json!({"value":e["address"]}))
                    .collect::<Vec<_>>()
            );
            let mut phones = value["businessPhones"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            if let Some(phone) = value.get("mobilePhone").filter(|v| !v.is_null()) {
                phones.push(phone.clone());
            }
            out["phoneNumbers"] = json!(
                phones
                    .iter()
                    .map(|p| json!({"value":p}))
                    .collect::<Vec<_>>()
            );
            out["organizations"] = json!([{"name":value["companyName"],"title":value["jobTitle"],"department":value["department"]}]);
            out["biographies"] = json!([{"value":value["personalNotes"]}]);
        }
        Shape::Message | Shape::Draft => {
            copy(
                value,
                &mut out,
                &[("conversationId", "threadId"), ("bodyPreview", "snippet")],
            );
            let mut labels = Vec::new();
            if value["isRead"] == false {
                labels.push(json!("UNREAD"));
            }
            if value["flag"]["flagStatus"] == "flagged" {
                labels.push(json!("STARRED"));
            }
            if value["isDraft"] == true {
                labels.push(json!("DRAFT"));
            }
            for category in value["categories"].as_array().into_iter().flatten() {
                labels.push(category.clone());
            }
            out["labelIds"] = json!(labels);
            let address = |v: &Value| {
                v["emailAddress"]["address"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned()
            };
            let addresses = |key: &str| {
                value[key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .map(address)
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let mut headers = vec![
                json!({"name":"From","value":address(&value["from"])}),
                json!({"name":"Subject","value":value["subject"]}),
                json!({"name":"Date","value":value["receivedDateTime"]}),
            ];
            for (key, name) in [
                ("toRecipients", "To"),
                ("ccRecipients", "Cc"),
                ("bccRecipients", "Bcc"),
            ] {
                headers.push(json!({"name":name,"value":addresses(key)}));
            }
            let content = value["body"]["content"].as_str().unwrap_or_default();
            let mime = if value["body"]["contentType"]
                .as_str()
                .is_some_and(|s| s.eq_ignore_ascii_case("html"))
            {
                "text/html"
            } else {
                "text/plain"
            };
            let mut parts = vec![
                json!({"mimeType":mime,"body":{"data":URL_SAFE_NO_PAD.encode(content),"size":content.len()}}),
            ];
            for attachment in value["attachments"].as_array().into_iter().flatten() {
                parts.push(json!({"filename":attachment["name"],"mimeType":attachment["contentType"],"body":{"attachmentId":attachment["id"],"size":attachment["size"]}}));
            }
            out["payload"] = json!({"mimeType":"multipart/mixed","headers":headers,"parts":parts});
            if shape == Shape::Draft {
                out = json!({"id":value["id"],"message":out});
            }
        }
        Shape::Label => {
            copy(
                value,
                &mut out,
                &[
                    ("displayName", "name"),
                    ("totalItemCount", "messagesTotal"),
                    ("unreadItemCount", "messagesUnread"),
                ],
            );
        }
        Shape::Permission => {
            out = value.clone();
            out["role"] = json!(if value["roles"]
                .as_array()
                .is_some_and(|a| a.iter().any(|v| v == "write"))
            {
                "writer"
            } else {
                "reader"
            });
        }
        Shape::Revision => {
            out = value.clone();
            out["modifiedTime"] = value["lastModifiedDateTime"].clone();
        }
        Shape::Identity => out = value.clone(),
        Shape::Workbook => {
            out = json!({"sheets":items(value).iter().map(|sheet|json!({"properties":{"sheetId":sheet["id"],"title":sheet["name"],"index":sheet["position"]}})).collect::<Vec<_>>()});
        }
        Shape::Range => {
            out = json!({"range":value["address"],"majorDimension":"ROWS","values":value["values"]})
        }
    }
    out
}

fn query(request: &mut Request, name: &str, value: impl AsRef<str>) {
    const COMPONENT: &percent_encoding::AsciiSet = &percent_encoding::NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    let mut pairs = request.url.query_pairs().into_owned().collect::<Vec<_>>();
    pairs.push((name.to_owned(), value.as_ref().to_owned()));
    let query = pairs
        .into_iter()
        .map(|(name, value)| {
            let name = match name.strip_prefix('$') {
                Some(name) => format!(
                    "${}",
                    percent_encoding::utf8_percent_encode(name, COMPONENT)
                ),
                None => percent_encoding::utf8_percent_encode(&name, COMPONENT).to_string(),
            };
            format!(
                "{name}={}",
                percent_encoding::utf8_percent_encode(&value, COMPONENT)
            )
        })
        .collect::<Vec<_>>()
        .join("&");
    request.url.set_query(Some(&query));
}

#[derive(Default, Debug)]
struct Selection(std::collections::BTreeMap<String, Selection>);

fn field_selection(fields: &str) -> Result<Selection> {
    fn parse(chars: &[u8], cursor: &mut usize, depth: usize) -> Result<Selection> {
        ensure!(depth <= 32, "field selection is too deeply nested");
        let mut selection = Selection::default();
        while *cursor < chars.len() && chars[*cursor] != b')' {
            let start = *cursor;
            while *cursor < chars.len() && !b",()".contains(&chars[*cursor]) {
                *cursor += 1;
            }
            let name = std::str::from_utf8(&chars[start..*cursor])?.trim();
            ensure!(!name.is_empty(), "invalid field selection");
            let children = if *cursor < chars.len() && chars[*cursor] == b'(' {
                *cursor += 1;
                let children = parse(chars, cursor, depth + 1)?;
                ensure!(
                    *cursor < chars.len() && chars[*cursor] == b')',
                    "unclosed field selection"
                );
                *cursor += 1;
                children
            } else {
                Selection::default()
            };
            let mut target = &mut selection;
            for part in name.split(['/', '.']) {
                ensure!(!part.is_empty(), "invalid field selection path");
                target = target.0.entry(part.to_owned()).or_default();
            }
            target.0.extend(children.0);
            if *cursor < chars.len() && chars[*cursor] != b')' {
                ensure!(chars[*cursor] == b',', "invalid field selection separator");
                *cursor += 1;
                ensure!(
                    *cursor < chars.len() && chars[*cursor] != b')',
                    "empty field selection"
                );
            }
        }
        Ok(selection)
    }
    ensure!(!fields.trim().is_empty(), "field selection cannot be empty");
    let mut cursor = 0;
    let result = parse(fields.as_bytes(), &mut cursor, 0)?;
    ensure!(
        cursor == fields.len(),
        "unexpected field selection terminator"
    );
    Ok(result)
}

fn project(value: &Value, selection: &Selection) -> Value {
    if selection.0.is_empty() || selection.0.contains_key("*") {
        return value.clone();
    }
    match value {
        Value::Array(values) => json!(
            values
                .iter()
                .map(|v| project(v, selection))
                .collect::<Vec<_>>()
        ),
        Value::Object(object) => Value::Object(
            selection
                .0
                .iter()
                .filter_map(|(key, children)| {
                    object
                        .get(key)
                        .map(|value| (key.clone(), project(value, children)))
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// Resolves and validates the entire operation before vault access.
pub fn plan(
    spec: &MethodSpec,
    params: &Map<String, Value>,
    body: Option<&Value>,
) -> Result<Request> {
    if let Some(fields) = text(params, "fields") {
        field_selection(fields)?;
    }
    let id = spec.id.as_str();
    let empty = json!({});
    let body = body.unwrap_or(&empty);
    let original = Method::from_bytes(spec.http_method.as_bytes())?;
    let method = if original == Method::PUT {
        Method::PATCH
    } else {
        original
    };
    let mut r = if let Some(action) = id.strip_prefix("mail.") {
        mail_plan(action, params, body, method)?
    } else if let Some(action) = id.strip_prefix("drive.") {
        drive_plan(action, params, body, method)?
    } else if let Some(action) = id.strip_prefix("calendar.") {
        calendar_plan(action, params, body, method)?
    } else if let Some(action) = id.strip_prefix("tasks.") {
        tasks_plan(action, params, body, method)?
    } else if let Some(action) = id.strip_prefix("contacts.") {
        contacts_plan(action, params, body, method)?
    } else if let Some(action) = id.strip_prefix("sheets.") {
        excel_plan(action, params, body)?
    } else {
        return Err(unsupported(id));
    };
    if let Some(size) = params.get("pageSize").or_else(|| params.get("maxResults")) {
        query(
            &mut r,
            "$top",
            size.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| size.to_string()),
        );
    }
    if let Some(next) = text(params, "pageToken") {
        ensure!(
            r.method == Method::GET && r.collection.is_some(),
            "pageToken requires a collection read"
        );
        r.url = collection_page(&r.url, next)?;
    }
    if id == "contacts.people.update-contact" {
        r.etag = body.get("etag").and_then(Value::as_str).map(str::to_owned);
    }
    Ok(r)
}

fn mail_plan(action: &str, p: &Map<String, Value>, b: &Value, method: Method) -> Result<Request> {
    let make =
        |path: &str, shape, collection| Request::new(method.clone(), path, None, shape, collection);
    match action {
        "profile" => make("me", Shape::Identity, None),
        "messages.send" | "drafts.create" => {
            ensure!(
                b.get("threadId").is_none() && b["message"].get("threadId").is_none(),
                "reply threading requires Outlook's reply operation and has no raw-MIME equivalent"
            );
            let raw = b["raw"]
                .as_str()
                .or_else(|| b["message"]["raw"].as_str())
                .context("missing MIME message")?;
            let bytes = URL_SAFE_NO_PAD.decode(raw.trim_end_matches('='))?;
            let mut r = Request::new(
                Method::POST,
                if action == "messages.send" {
                    "me/sendMail"
                } else {
                    "me/messages"
                },
                None,
                if action == "messages.send" {
                    Shape::Message
                } else {
                    Shape::Draft
                },
                None,
            )?;
            r.mime = Some(STANDARD.encode(bytes));
            Ok(r)
        }
        "drafts.send" => Request::new(
            Method::POST,
            &format!(
                "me/messages/{}/send",
                segment(b["id"].as_str().context("missing draft id")?)?
            ),
            None,
            Shape::Message,
            None,
        ),
        "messages.list" | "drafts.list" => {
            let draft = action == "drafts.list";
            let mut r = make(
                if draft {
                    "me/mailFolders/drafts/messages"
                } else {
                    "me/messages"
                },
                if draft { Shape::Draft } else { Shape::Message },
                Some(if draft { "drafts" } else { "messages" }),
            )?;
            let mut filters = Vec::new();
            let mut search = Vec::new();
            if let Some(q) = text(p, "q") {
                for term in q.split_whitespace() {
                    match term {
                        "in:inbox" => {
                            r.url = Url::parse(&format!("{ROOT}/me/mailFolders/inbox/messages"))?
                        }
                        "in:sent" => {
                            r.url =
                                Url::parse(&format!("{ROOT}/me/mailFolders/sentitems/messages"))?
                        }
                        "in:trash" => {
                            r.url =
                                Url::parse(&format!("{ROOT}/me/mailFolders/deleteditems/messages"))?
                        }
                        "is:unread" => filters.push("isRead eq false".to_owned()),
                        "is:read" => filters.push("isRead eq true".to_owned()),
                        "is:starred" => filters.push("flag/flagStatus eq 'flagged'".to_owned()),
                        "has:attachment" => filters.push("hasAttachments eq true".to_owned()),
                        s if s.starts_with("from:") => filters.push(format!(
                            "from/emailAddress/address eq '{}'",
                            s[5..].replace('\'', "''")
                        )),
                        s if s.starts_with("after:") || s.starts_with("before:") => {
                            let (kind, date) = s.split_once(':').unwrap();
                            let date = date.replace('/', "-");
                            ensure!(
                                date.len() == 10
                                    && date.chars().all(|c| c.is_ascii_digit() || c == '-'),
                                "mail date must use YYYY-MM-DD"
                            );
                            filters.push(format!(
                                "receivedDateTime {} {date}T00:00:00Z",
                                if kind == "after" { "ge" } else { "lt" }
                            ));
                        }
                        s => search.push(s),
                    }
                }
            }
            for label in p
                .get("labelIds")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                match label {
                    "UNREAD" => filters.push("isRead eq false".into()),
                    "INBOX" => {
                        r.url = Url::parse(&format!("{ROOT}/me/mailFolders/inbox/messages"))?
                    }
                    _ => bail!("labelIds {label} has no Microsoft list filter; use q"),
                }
            }
            ensure!(
                filters.is_empty() || search.is_empty(),
                "Outlook cannot combine full-text search with these filters; run them separately"
            );
            if !filters.is_empty() {
                query(&mut r, "$filter", filters.join(" and "));
            }
            if !search.is_empty() {
                query(
                    &mut r,
                    "$search",
                    format!("\"{}\"", search.join(" ").replace('"', "\\\"")),
                );
            }
            Ok(r)
        }
        "messages.get" | "messages.delete" | "drafts.get" | "drafts.delete" => {
            ensure!(
                text(p, "format") != Some("raw"),
                "raw MIME retrieval is not available in this compatibility command"
            );
            let mut r = make(
                &format!("me/messages/{}", required(p, "id")?),
                if action.starts_with("drafts") {
                    Shape::Draft
                } else {
                    Shape::Message
                },
                None,
            )?;
            if action == "messages.delete" {
                r.method = Method::POST;
                r.url.set_path(&format!("{}/permanentDelete", r.url.path()));
            }
            if r.method == Method::GET {
                query(&mut r, "$expand", "attachments");
            }
            Ok(r)
        }
        "messages.attachments.get" => make(
            &format!(
                "me/messages/{}/attachments/{}",
                required(p, "messageId")?,
                required(p, "id")?
            ),
            Shape::Identity,
            None,
        ),
        "messages.trash" | "messages.untrash" => Request::new(
            Method::POST,
            &format!("me/messages/{}/move", required(p, "id")?),
            Some(
                json!({"destinationId":if action.ends_with("untrash") {"inbox"} else {"deleteditems"}}),
            ),
            Shape::Message,
            None,
        ),
        "messages.modify" => {
            let mut body = json!({});
            for (key, add) in [("addLabelIds", true), ("removeLabelIds", false)] {
                for label in b[key]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                {
                    match label {
                        "UNREAD" => body["isRead"] = json!(!add),
                        "STARRED" => {
                            body["flag"] =
                                json!({"flagStatus":if add {"flagged"} else {"notFlagged"}})
                        }
                        _ => bail!("label {label} has no lossless Outlook equivalent"),
                    }
                }
            }
            Request::new(
                Method::PATCH,
                &format!("me/messages/{}", required(p, "id")?),
                Some(body),
                Shape::Message,
                None,
            )
        }
        "labels.list" => make("me/mailFolders", Shape::Label, Some("labels")),
        "labels.get" | "labels.delete" => make(
            &format!("me/mailFolders/{}", required(p, "id")?),
            Shape::Label,
            None,
        ),
        "labels.create" => Request::new(
            Method::POST,
            "me/mailFolders",
            Some(write_body(Shape::Label, b)?),
            Shape::Label,
            None,
        ),
        "labels.patch" | "labels.update" => Request::new(
            Method::PATCH,
            &format!("me/mailFolders/{}", required(p, "id")?),
            Some(write_body(Shape::Label, b)?),
            Shape::Label,
            None,
        ),
        _ => Err(unsupported(&format!("mail {action}"))),
    }
}

fn drive_plan(action: &str, p: &Map<String, Value>, b: &Value, method: Method) -> Result<Request> {
    match action {
        "files.copy" => {
            for key in b.as_object().context("copy body must be an object")?.keys() {
                ensure!(
                    ["name", "parents"].contains(&key.as_str()),
                    "{key} is not supported when copying a OneDrive file"
                );
            }
            Request::new(
                Method::POST,
                &format!("me/drive/items/{}/copy", required(p, "fileId")?),
                Some(b.clone()),
                Shape::File,
                None,
            )
        }
        "files.list" => {
            let mut parent = "root".to_owned();
            let mut search = None;
            let mut filters = Vec::new();
            if let Some(q) = text(p, "q") {
                for clause in q.split(" and ").map(str::trim) {
                    if matches!(clause, "trashed = false" | "trashed=false") {
                        continue;
                    }
                    if let Some(value) = clause
                        .strip_suffix(" in parents")
                        .and_then(|s| s.trim().strip_prefix('\''))
                        .and_then(|s| s.strip_suffix('\''))
                    {
                        parent = value.into();
                    } else if let Some(value) = clause
                        .strip_prefix("name contains '")
                        .and_then(|s| s.strip_suffix('\''))
                    {
                        search = Some(value.to_owned());
                        filters.push(("name".into(), value.to_owned()));
                    } else if let Some(value) = clause
                        .strip_prefix("fullText contains '")
                        .and_then(|s| s.strip_suffix('\''))
                    {
                        search = Some(value.to_owned());
                    } else if let Some(value) = clause
                        .strip_prefix("mimeType = '")
                        .and_then(|s| s.strip_suffix('\''))
                    {
                        filters.push(("mimeType".into(), value.to_owned()));
                    } else if let Some(value) = clause
                        .strip_prefix("mimeType contains '")
                        .and_then(|s| s.strip_suffix('\''))
                    {
                        filters.push(("mimePrefix".into(), value.to_owned()));
                    } else {
                        bail!(
                            "unsupported OneDrive query clause {clause:?}; use a parent or name contains filter"
                        );
                    }
                }
            }
            let path = if parent == "root" {
                "me/drive/root".into()
            } else {
                format!("me/drive/items/{}", segment(&parent)?)
            };
            let path = if let Some(search) = search {
                format!(
                    "{path}/search(q='{}')",
                    segment(&search.replace('\'', "''"))?
                )
            } else {
                format!("{path}/children")
            };
            let mut r = Request::new(Method::GET, &path, None, Shape::File, Some("files"))?;
            r.filters = filters;
            if let Some(order) = text(p, "orderBy") {
                query(
                    &mut r,
                    "$orderby",
                    order
                        .replace("modifiedTime", "lastModifiedDateTime")
                        .replace("createdTime", "createdDateTime"),
                );
            }
            Ok(r)
        }
        "files.get" | "files.delete" | "files.update" => {
            let trash = b.get("trashed") == Some(&json!(true));
            let mut body = b.clone();
            if trash {
                ensure!(
                    body.as_object().is_some_and(|o| o.len() == 1),
                    "move to the recycle bin separately from metadata edits"
                );
                body = json!({});
            }
            let mut body = if body.as_object().is_some_and(|o| !o.is_empty()) {
                Some(write_body(Shape::File, &body)?)
            } else {
                None
            };
            if let Some(parent) = text(p, "addParents") {
                ensure!(
                    !parent.contains(','),
                    "a OneDrive item has exactly one parent"
                );
                body.get_or_insert_with(|| json!({}))["parentReference"] = json!({"id":parent});
            }
            ensure!(
                p.get("removeParents").is_none() || p.get("addParents").is_some(),
                "moving a OneDrive item requires a destination parent"
            );
            Request::new(
                if trash { Method::DELETE } else { method },
                &format!("me/drive/items/{}", required(p, "fileId")?),
                body,
                Shape::File,
                None,
            )
        }
        "files.create" => {
            ensure!(
                b["mimeType"] == "inode/directory",
                "use drive upload to create a OneDrive file"
            );
            let parent = b["parents"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(Value::as_str)
                .unwrap_or("root");
            let path = if parent == "root" {
                "me/drive/root/children".into()
            } else {
                format!("me/drive/items/{}/children", segment(parent)?)
            };
            let mut body = write_body(Shape::File, b)?;
            body.as_object_mut().unwrap().remove("parentReference");
            Request::new(Method::POST, &path, Some(body), Shape::File, None)
        }
        "permissions.list" | "permissions.get" | "permissions.delete" => {
            let mut path = format!("me/drive/items/{}/permissions", required(p, "fileId")?);
            if action != "permissions.list" {
                path.push('/');
                path.push_str(&required(p, "permissionId")?);
            }
            Request::new(
                method,
                &path,
                None,
                Shape::Permission,
                if action == "permissions.list" {
                    Some("permissions")
                } else {
                    None
                },
            )
        }
        "permissions.create" => {
            ensure!(
                b["type"] == "user",
                "OneDrive invitations require a user email address"
            );
            let role = match b["role"].as_str() {
                Some("reader") => "read",
                Some("writer") => "write",
                _ => bail!("OneDrive role must be reader or writer"),
            };
            Request::new(
                Method::POST,
                &format!("me/drive/items/{}/invite", required(p, "fileId")?),
                Some(
                    json!({"recipients":[{"email":b["emailAddress"]}],"requireSignIn":true,"sendInvitation":p.get("sendNotificationEmail").cloned().unwrap_or(json!(true)),"roles":[role]}),
                ),
                Shape::Permission,
                Some("permissions"),
            )
        }
        "revisions.list" | "revisions.get" => {
            let mut path = format!("me/drive/items/{}/versions", required(p, "fileId")?);
            if action == "revisions.get" {
                path.push('/');
                path.push_str(&required(p, "revisionId")?);
            }
            Request::new(
                Method::GET,
                &path,
                None,
                Shape::Revision,
                if action == "revisions.list" {
                    Some("revisions")
                } else {
                    None
                },
            )
        }
        "drives.list" => Request::new(
            Method::GET,
            "me/drives",
            None,
            Shape::Identity,
            Some("drives"),
        ),
        "drives.get" => Request::new(
            Method::GET,
            &format!("drives/{}", required(p, "driveId")?),
            None,
            Shape::Identity,
            None,
        ),
        _ => Err(unsupported(&format!("drive {action}"))),
    }
}

fn calendar_plan(
    action: &str,
    p: &Map<String, Value>,
    b: &Value,
    method: Method,
) -> Result<Request> {
    if action.starts_with("events.")
        && matches!(method, Method::POST | Method::PATCH | Method::DELETE)
    {
        ensure!(
            text(p, "sendUpdates").is_none_or(|v| v == "all"),
            "Outlook sends meeting updates automatically; use sendUpdates all or notify:true"
        );
    }
    let calendar = || -> Result<String> {
        Ok(match text(p, "calendarId").unwrap_or("primary") {
            "primary" => "me/calendar".into(),
            id => format!("me/calendars/{}", segment(id)?),
        })
    };
    match action {
        "calendars.list" => Request::new(
            Method::GET,
            "me/calendars",
            None,
            Shape::Calendar,
            Some("items"),
        ),
        "calendar-list.get" | "calendars.get" | "calendars.delete" | "calendars.patch"
        | "calendars.update" => Request::new(
            method,
            &calendar()?,
            if b.as_object().is_some_and(|o| !o.is_empty()) {
                Some(write_body(Shape::Calendar, b)?)
            } else {
                None
            },
            Shape::Calendar,
            None,
        ),
        "calendars.insert" => Request::new(
            Method::POST,
            "me/calendars",
            Some(write_body(Shape::Calendar, b)?),
            Shape::Calendar,
            None,
        ),
        "events.list" => {
            let base = calendar()?;
            let mut r = Request::new(
                Method::GET,
                &format!("{base}/events"),
                None,
                Shape::Event,
                Some("items"),
            )?;
            match (text(p, "timeMin"), text(p, "timeMax")) {
                (Some(start), Some(end)) => {
                    r.url = Url::parse(&format!("{ROOT}/{base}/calendarView"))?;
                    query(&mut r, "startDateTime", start);
                    query(&mut r, "endDateTime", end);
                }
                (None, None) => {}
                (start, end) => {
                    let mut filters = Vec::new();
                    if let Some(start) = start {
                        filters.push(format!("end/dateTime ge '{}'", start.replace('\'', "''")));
                    }
                    if let Some(end) = end {
                        filters.push(format!("start/dateTime lt '{}'", end.replace('\'', "''")));
                    }
                    query(&mut r, "$filter", filters.join(" and "));
                }
            }
            if let Some(q) = text(p, "q") {
                query(
                    &mut r,
                    "$filter",
                    format!("contains(subject,'{}')", q.replace('\'', "''")),
                );
            }
            if text(p, "orderBy") == Some("startTime") {
                query(&mut r, "$orderby", "start/dateTime");
            }
            Ok(r)
        }
        "events.insert" | "events.patch" | "events.update" | "events.get" | "events.delete" => {
            let mut path = format!("{}/events", calendar()?);
            if action != "events.insert" {
                path.push('/');
                path.push_str(&required(p, "eventId")?);
            }
            Request::new(
                method,
                &path,
                if matches!(action, "events.insert" | "events.patch" | "events.update") {
                    Some(write_body(Shape::Event, b)?)
                } else {
                    None
                },
                Shape::Event,
                None,
            )
        }
        "events.instances" => {
            let mut r = Request::new(
                Method::GET,
                &format!(
                    "{}/events/{}/instances",
                    calendar()?,
                    required(p, "eventId")?
                ),
                None,
                Shape::Event,
                Some("items"),
            )?;
            query(
                &mut r,
                "startDateTime",
                text(p, "timeMin").context("instances requires timeMin")?,
            );
            query(
                &mut r,
                "endDateTime",
                text(p, "timeMax").context("instances requires timeMax")?,
            );
            Ok(r)
        }
        _ => Err(unsupported(&format!("calendar {action}"))),
    }
}

fn tasks_plan(action: &str, p: &Map<String, Value>, b: &Value, method: Method) -> Result<Request> {
    if matches!(action, "tasks.clear" | "tasks.move") {
        return Err(unsupported(&format!("tasks {action}")));
    }
    let lists = action.starts_with("tasklists.");
    let mut path = "me/todo/lists".to_owned();
    if !lists || !matches!(action, "tasklists.list" | "tasklists.insert") {
        path.push('/');
        path.push_str(&required(p, "tasklist")?);
    }
    if !lists {
        path.push_str("/tasks");
        if !matches!(action, "tasks.list" | "tasks.insert") {
            path.push('/');
            path.push_str(&required(p, "task")?);
        }
    }
    ensure!(
        p.get("parent").is_none() && p.get("previous").is_none(),
        "Microsoft To Do does not support task hierarchy or positions"
    );
    let shape = if lists { Shape::TaskList } else { Shape::Task };
    let mut r = Request::new(
        method.clone(),
        &path,
        if matches!(method, Method::POST | Method::PATCH) {
            Some(write_body(shape, b)?)
        } else {
            None
        },
        shape,
        if action.ends_with(".list") {
            Some("items")
        } else {
            None
        },
    )?;
    if !lists && action == "tasks.list" {
        let mut filters = Vec::new();
        if p.get("showCompleted") == Some(&json!(false)) {
            filters.push("status ne 'completed'".into());
        }
        for (from, op) in [("dueMin", "ge"), ("dueMax", "le")] {
            if let Some(date) = text(p, from) {
                filters.push(format!(
                    "dueDateTime/dateTime {op} '{}'",
                    date.replace('\'', "''")
                ));
            }
        }
        if !filters.is_empty() {
            query(&mut r, "$filter", filters.join(" and "));
        }
    }
    Ok(r)
}

fn contacts_plan(
    action: &str,
    p: &Map<String, Value>,
    b: &Value,
    method: Method,
) -> Result<Request> {
    let contact = || -> Result<String> {
        let name = text(p, "resourceName").context("missing resourceName")?;
        segment(name.strip_prefix("people/").unwrap_or(name))
    };
    match action {
        "people.connections.list" | "people.search-contacts" => {
            let mut r = Request::new(
                Method::GET,
                "me/contacts",
                None,
                Shape::Contact,
                Some(if action.ends_with("search-contacts") {
                    "results"
                } else {
                    "connections"
                }),
            )?;
            if let Some(q) = text(p, "query") {
                let q = q.replace('\'', "''");
                query(
                    &mut r,
                    "$filter",
                    format!(
                        "startswith(displayName,'{q}') or startswith(givenName,'{q}') or startswith(surname,'{q}')"
                    ),
                );
            }
            Ok(r)
        }
        "people.get" | "people.delete-contact" | "people.update-contact" => Request::new(
            method.clone(),
            &format!("me/contacts/{}", contact()?),
            if method == Method::PATCH {
                Some(write_body(Shape::Contact, b)?)
            } else {
                None
            },
            Shape::Contact,
            None,
        ),
        "people.create-contact" => Request::new(
            Method::POST,
            "me/contacts",
            Some(write_body(Shape::Contact, b)?),
            Shape::Contact,
            None,
        ),
        _ => Err(unsupported(&format!("contacts {action}"))),
    }
}

fn excel_write_address(address: &str, body: &Value) -> Result<String> {
    if address.contains(':') {
        return Ok(address.into());
    }
    let cell = address.strip_prefix('$').unwrap_or(address);
    let letters = cell.bytes().take_while(u8::is_ascii_alphabetic).count();
    let (column, row) = cell.split_at(letters);
    let row = row.strip_prefix('$').unwrap_or(row);
    if column.is_empty() || row.is_empty() || !row.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(address.into());
    }
    let rows = body["values"]
        .as_array()
        .context("Excel values must be a row grid")?;
    let width = rows
        .iter()
        .filter_map(Value::as_array)
        .map(Vec::len)
        .max()
        .unwrap_or(0);
    ensure!(
        !rows.is_empty() && width > 0,
        "Excel values must contain at least one cell"
    );
    let start_column = column
        .bytes()
        .try_fold(0_usize, |value, byte| {
            value
                .checked_mul(26)?
                .checked_add(usize::from(byte.to_ascii_uppercase() - b'A') + 1)
        })
        .context("Excel column is out of range")?;
    let start_row = row.parse::<usize>().context("Excel row is out of range")?;
    let end_column = start_column
        .checked_add(width - 1)
        .context("Excel column is out of range")?;
    let end_row = start_row
        .checked_add(rows.len() - 1)
        .context("Excel row is out of range")?;
    ensure!(
        start_row > 0 && end_row <= 1_048_576 && end_column <= 16_384,
        "Excel write exceeds worksheet bounds"
    );
    if rows.len() == 1 && width == 1 {
        return Ok(address.into());
    }
    let mut number = end_column;
    let mut name = Vec::new();
    while number > 0 {
        number -= 1;
        name.push(char::from(b'A' + (number % 26) as u8));
        number /= 26;
    }
    let name: String = name.into_iter().rev().collect();
    Ok(format!(
        "{}{}:{name}{end_row}",
        column.to_ascii_uppercase(),
        start_row
    ))
}

fn excel_plan(action: &str, p: &Map<String, Value>, body: &Value) -> Result<Request> {
    if !["get", "values.get", "values.update", "values.clear"].contains(&action) {
        return Err(unsupported(&format!("sheets {action}")));
    }
    let base = format!("me/drive/items/{}/workbook", required(p, "spreadsheetId")?);
    if action == "get" {
        return Request::new(
            Method::GET,
            &format!("{base}/worksheets"),
            None,
            Shape::Workbook,
            None,
        );
    }
    ensure!(
        ["values.get", "values.update", "values.clear"].contains(&action),
        "Excel {action} has no implemented Microsoft API mapping"
    );
    let range = text(p, "range").context("missing range")?;
    let (sheet, address) = if let Some(parts) = range.rsplit_once('!') {
        parts
    } else {
        ensure!(
            action == "values.get" && range.starts_with('\'') && range.ends_with('\''),
            "Excel ranges require a worksheet and address, such as Sheet1!A1:B4"
        );
        let sheet = range.trim_matches('\'').replace("''", "'");
        return Request::new(
            Method::GET,
            &format!(
                "{base}/worksheets/{}/usedRange(valuesOnly=true)",
                segment(&sheet)?
            ),
            None,
            Shape::Range,
            None,
        );
    };
    let sheet = sheet.trim_matches('\'').replace("''", "'");
    let address = if action == "values.update" {
        excel_write_address(address, body)?
    } else {
        address.into()
    };
    let path = format!(
        "{base}/worksheets/{}/range(address='{}')",
        segment(&sheet)?,
        segment(&address.replace('\'', "''"))?
    );
    match action {
        "values.get" => Request::new(Method::GET, &path, None, Shape::Range, None),
        "values.update" => Request::new(
            Method::PATCH,
            &path,
            Some(write_body(Shape::Range, body)?),
            Shape::Range,
            None,
        ),
        "values.clear" => Request::new(
            Method::POST,
            &format!("{path}/clear"),
            Some(json!({"applyTo":"Contents"})),
            Shape::Identity,
            None,
        ),
        _ => unreachable!(),
    }
}

pub fn json(
    http: &Http,
    spec: &MethodSpec,
    p: &Map<String, Value>,
    body: Option<&Value>,
    token: &str,
) -> Result<Value> {
    if spec.id == "drive.drives.list" {
        let drive = http.json(
            &Method::GET,
            &Url::parse(&format!("{ROOT}/me/drive"))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        if let Some(mut result) = personal_drive_listing(&drive) {
            ensure!(
                p.get("pageToken").is_none(),
                "personal OneDrive exposes one default drive and has no drive-list page tokens"
            );
            if let Some(fields) = text(p, "fields") {
                result = project(&result, &field_selection(fields)?);
            }
            return Ok(result);
        }
    }
    #[cfg(feature = "live-test-diagnostics")]
    if spec.id == "drive.files.delete"
        && http::live_session_only()
        && let Ok(marker) = std::env::var("JOOM_LIVE_CLEANUP_MARKER")
    {
        ensure!(
            marker.starts_with("joom-test-")
                && marker
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "invalid cleanup fixture marker"
        );
        let request = plan(spec, p, body)?;
        let metadata = http.json(
            &Method::GET,
            &Url::parse(&format!(
                "{ROOT}/me/drive/items/{}?$select=id,name,eTag",
                required(p, "fileId")?
            ))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        ensure!(
            metadata["name"]
                .as_str()
                .is_some_and(|name| name.starts_with(&marker)),
            "refusing to bypass a lock on a non-fixture item"
        );
        let mut deletion = http
            .client
            .delete(request.url.clone())
            .bearer_auth(token)
            .header("Prefer", "bypass-shared-lock");
        if let Some(etag) = metadata["eTag"].as_str() {
            deletion = deletion.header(reqwest::header::IF_MATCH, etag);
        }
        let response = deletion.send().map_err(|error| error.without_url())?;
        http::check_response(response, &Method::DELETE, &request.url)?;
        return Ok(Value::Null);
    }
    if spec.id == "drive.files.copy" {
        plan(spec, p, body)?;
        let metadata = http.json(
            &Method::GET,
            &Url::parse(&format!("{ROOT}/me/drive/items/{}", required(p, "fileId")?))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        ensure!(
            metadata.get("folder").is_none(),
            "folder copying requires an asynchronous OneDrive operation and is not implemented"
        );
        let mut copied = body.cloned().unwrap_or_else(|| json!({}));
        if copied.get("name").is_none() {
            copied["name"] = json!(format!(
                "Copy of {}",
                metadata["name"]
                    .as_str()
                    .context("missing OneDrive filename")?
            ));
        }
        let directory = tempfile::tempdir()?;
        let file = directory.path().join("copy");
        let name = file.to_str().context("invalid temporary path")?;
        download(
            http,
            crate::schema_commands::contract("drive")
                .document
                .method("files.get")?,
            p,
            token,
            name,
        )?;
        return upload(
            http,
            crate::schema_commands::contract("drive")
                .document
                .method("files.create")?,
            &Map::new(),
            Some(&copied),
            token,
            name,
            metadata["file"]["mimeType"]
                .as_str()
                .unwrap_or("application/octet-stream"),
        );
    }
    // To Do exposes its default list through wellknownListName, rather than an ID alias.
    let mut resolved = p.clone();
    if text(p, "tasklist") == Some("@default") && spec.id.starts_with("tasks.") {
        let lists = http.json(
            &Method::GET,
            &Url::parse(&format!("{ROOT}/me/todo/lists"))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        let default = items(&lists)
            .iter()
            .find(|list| list["wellknownListName"] == "defaultList")
            .and_then(|list| list["id"].as_str())
            .context("Microsoft did not return the default To Do list; select a list explicitly")?;
        resolved.insert("tasklist".into(), json!(default));
    }
    let p = &resolved;
    let r = plan(spec, p, body)?;
    let value = if let Some(mime) = &r.mime {
        let result = http
            .client
            .request(r.method.clone(), r.url.clone())
            .bearer_auth(token)
            .header("Content-Type", "text/plain")
            .body(mime.clone())
            .send()
            .map_err(|e| e.without_url())?;
        let result = http::check_response(result, &r.method, &r.url)?;
        let bytes = http::limited(result, http::JSON_LIMIT)?;
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)?
        }
    } else {
        http.json_conditional(
            &r.method,
            &r.url,
            r.body.as_ref(),
            Some(token),
            http::JSON_LIMIT,
            r.etag.as_deref(),
        )?
    };
    let mut result = if let Some(collection) = r.collection {
        let values = items(&value)
            .iter()
            .filter(|v| {
                r.filters.iter().all(|(key, expected)| match key.as_str() {
                    "mimeType" => {
                        v["file"]["mimeType"].as_str() == Some(expected)
                            || (expected == "inode/directory" && v.get("folder").is_some())
                    }
                    "mimePrefix" => v["file"]["mimeType"]
                        .as_str()
                        .is_some_and(|mime| mime.starts_with(expected)),
                    "name" => v["name"]
                        .as_str()
                        .is_some_and(|name| name.to_lowercase().contains(&expected.to_lowercase())),
                    _ => false,
                })
            })
            .map(|v| {
                let v = response(r.shape, v);
                if collection == "results" {
                    json!({"person":v})
                } else {
                    v
                }
            })
            .collect::<Vec<_>>();
        let mut out = json!({collection:values});
        if let Some(next) = value["@odata.nextLink"].as_str() {
            let next = collection_page(&r.url, next)
                .context("Microsoft returned an invalid collection page")?;
            out["nextPageToken"] = json!(next.as_str());
        }
        out
    } else {
        response(r.shape, &value)
    };
    if spec.id == "mail.messages.attachments.get" {
        let content = value["contentBytes"]
            .as_str()
            .context("Microsoft did not return file attachment content")?;
        let bytes = STANDARD.decode(content)?;
        result = json!({"data":URL_SAFE_NO_PAD.encode(&bytes),"size":bytes.len()});
    }
    if spec.id == "sheets.get" {
        let metadata = http.json(
            &Method::GET,
            &Url::parse(&format!(
                "{ROOT}/me/drive/items/{}",
                required(p, "spreadsheetId")?
            ))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        result["spreadsheetId"] = p["spreadsheetId"].clone();
        result["properties"] = json!({"title":metadata["name"]});
        result["spreadsheetUrl"] = metadata["webUrl"].clone();
    }
    if spec.id == "sheets.values.get" {
        let property = match text(p, "valueRenderOption") {
            Some("FORMULA") => "formulas",
            Some("UNFORMATTED_VALUE") => "values",
            _ => "text",
        };
        result["values"] = value
            .get(property)
            .cloned()
            .unwrap_or_else(|| value["values"].clone());
    }
    if spec.id == "sheets.values.clear" {
        result = json!({"spreadsheetId":p["spreadsheetId"],"clearedRange":p["range"]});
    }
    if spec.id == "mail.profile" {
        result = json!({"emailAddress":value["mail"].as_str().or_else(||value["userPrincipalName"].as_str())});
    }
    if matches!(
        spec.id.as_str(),
        "calendar.calendars.get" | "calendar.calendar-list.get" | "calendar.calendars.list"
    ) {
        let settings = http.json(
            &Method::GET,
            &Url::parse(&format!("{ROOT}/me/mailboxSettings"))?,
            None,
            Some(token),
            http::JSON_LIMIT,
        )?;
        if let Ok(zone) = mailbox_zone(&settings) {
            if let Some(calendars) = result["items"].as_array_mut() {
                for calendar in calendars {
                    calendar["timeZone"] = json!(iana_zone(zone));
                }
            } else {
                result["timeZone"] = json!(iana_zone(zone));
            }
        }
    }
    if let Some(fields) = text(p, "fields") {
        result = project(&result, &field_selection(fields)?);
    }
    Ok(result)
}

fn personal_drive_listing(drive: &Value) -> Option<Value> {
    (drive["driveType"] == "personal" && drive["id"].as_str().is_some_and(|id| !id.is_empty()))
        .then(|| json!({"drives":[drive]}))
}

pub fn upload(
    http: &Http,
    spec: &MethodSpec,
    p: &Map<String, Value>,
    body: Option<&Value>,
    token: &str,
    path: &str,
    content_type: &str,
) -> Result<Value> {
    ensure!(
        matches!(
            spec.id.as_str(),
            "drive.files.create" | "drive.files.update"
        ),
        "this Microsoft operation does not support file uploads"
    );
    let file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    ensure!(
        size <= 250 * 1024 * 1024,
        "OneDrive simple uploads are limited to 250 MiB"
    );
    let path = if spec.id == "drive.files.update" {
        format!("me/drive/items/{}/content", required(p, "fileId")?)
    } else {
        let name = body
            .and_then(|b| b["name"].as_str())
            .or_else(|| {
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
            })
            .context("missing upload filename")?;
        let parent = body
            .and_then(|b| b["parents"].as_array())
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("root");
        let base = if parent == "root" {
            "me/drive/root".into()
        } else {
            format!("me/drive/items/{}", segment(parent)?)
        };
        format!("{base}:/{}:/content", segment(name)?)
    };
    let mut url = Url::parse(&format!("{ROOT}/{path}"))?;
    graph_url(&url)?;
    if spec.id == "drive.files.create" {
        url.query_pairs_mut()
            .append_pair("@microsoft.graph.conflictBehavior", "fail");
    }
    let mut request = http
        .client
        .put(url.clone())
        .bearer_auth(token)
        .header("Content-Type", content_type)
        .header("Content-Length", size)
        .body(reqwest::blocking::Body::new(file));
    if spec.id == "drive.files.create" {
        request = request.header(reqwest::header::IF_NONE_MATCH, "*");
    }
    let result = request.send().map_err(|e| e.without_url())?;
    let bytes = http::limited(
        http::check_response(result, &Method::PUT, &url)?,
        http::JSON_LIMIT,
    )?;
    Ok(response(Shape::File, &serde_json::from_slice(&bytes)?))
}

pub fn download(
    http: &Http,
    spec: &MethodSpec,
    p: &Map<String, Value>,
    token: &str,
    path: &str,
) -> Result<Value> {
    ensure!(
        matches!(
            spec.id.as_str(),
            "drive.files.get" | "drive.files.export" | "drive.revisions.get"
        ),
        "this Microsoft operation does not support file downloads"
    );
    let mut address = format!("{ROOT}/me/drive/items/{}/content", required(p, "fileId")?);
    if spec.id == "drive.revisions.get" {
        address = format!(
            "{ROOT}/me/drive/items/{}/versions/{}/content",
            required(p, "fileId")?,
            required(p, "revisionId")?
        );
    }
    let mut url = Url::parse(&address)?;
    graph_url(&url)?;
    if spec.id == "drive.files.export" {
        ensure!(
            text(p, "mimeType") == Some("application/pdf"),
            "OneDrive export currently supports application/pdf"
        );
        url.query_pairs_mut().append_pair("format", "pdf");
    }
    let mut response = http
        .client
        .get(url.clone())
        .bearer_auth(token)
        .send()
        .map_err(|e| e.without_url())?;
    // OneDrive supplies a short-lived download URL. Never forward the Graph bearer token.
    for _ in 0..5 {
        if !response.status().is_redirection() {
            break;
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .context("missing OneDrive download redirect")?
            .to_str()?;
        let next = url.join(location)?;
        ensure!(
            next.scheme() == "https"
                && next.username().is_empty()
                && next.password().is_none()
                && next.fragment().is_none()
                && next.port_or_known_default() == Some(443),
            "invalid OneDrive download redirect"
        );
        response = http
            .client
            .get(next.clone())
            .send()
            .map_err(|e| e.without_url())?;
        url = next;
    }
    http::check_status(response.status())?;
    crate::save_content(response, path)
}

/// Validate the compatibility URI without treating it as a network destination.
pub fn validate(
    doc: &Document,
    spec: &MethodSpec,
    p: &Map<String, Value>,
    body: Option<&Value>,
    upload: bool,
    download: bool,
) -> Result<()> {
    doc.url_with_body(spec, p, body)?;
    if upload {
        ensure!(
            matches!(
                spec.id.as_str(),
                "drive.files.create" | "drive.files.update"
            ),
            "this Microsoft operation does not support uploads"
        );
    } else if download {
        ensure!(
            matches!(
                spec.id.as_str(),
                "drive.files.get"
                    | "drive.files.export"
                    | "drive.revisions.get"
                    | "mail.messages.attachments.get"
            ),
            "this Microsoft operation does not support downloads"
        );
    } else {
        plan(spec, p, body)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/microsoft.rs"]
mod tests;
