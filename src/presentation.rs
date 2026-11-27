use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};

use crate::output;

fn mime_text(part: &Value, out: &mut String) -> Result<()> {
    if part["mimeType"] == "text/plain" && part["filename"].as_str().is_none_or(str::is_empty) {
        if let Some(data) = part.pointer("/body/data").and_then(Value::as_str) {
            let bytes = URL_SAFE_NO_PAD
                .decode(data.trim_end_matches('='))
                .context("Microsoft returned invalid message body encoding")?;
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&String::from_utf8_lossy(&bytes));
        }
    } else if let Some(parts) = part["parts"].as_array() {
        for part in parts {
            mime_text(part, out)?;
        }
    }
    Ok(())
}
fn message(value: &Value) -> Result<Value> {
    let mut result = serde_json::Map::new();
    result.insert("id".into(), value["id"].clone());
    if let Some(headers) = value.pointer("/payload/headers").and_then(Value::as_array) {
        for header in headers {
            let name = header["name"]
                .as_str()
                .unwrap_or_default()
                .to_ascii_lowercase();
            if ["date", "from", "to", "cc", "bcc", "subject"].contains(&name.as_str()) {
                result.insert(name, header["value"].clone());
            }
        }
    }
    let mut body = String::new();
    mime_text(&value["payload"], &mut body)?;
    if body.is_empty() {
        body = value["snippet"].as_str().unwrap_or_default().into();
    }
    result.insert("text".into(), json!(body));
    Ok(Value::Object(result))
}

fn compact(mut value: Value) -> Value {
    if let Some(fields) = value.as_object_mut() {
        fields.retain(|_, value| !value.is_null() && !value.as_array().is_some_and(Vec::is_empty));
    }
    value
}
fn rows(value: &Value, key: &str, view: impl Fn(&Value) -> Value) -> Value {
    value[key]
        .as_array()
        .map(|rows| Value::Array(rows.iter().map(view).collect()))
        .unwrap_or_else(|| value.clone())
}
fn contact(value: &Value) -> Value {
    let entries = |key: &str| {
        value[key]
            .as_array()
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry["value"].as_str())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    compact(
        json!({"id":value["resourceName"],"name":value.pointer("/names/0/displayName"),"email":entries("emailAddresses"),"phone":entries("phoneNumbers")}),
    )
}
fn event(value: &Value) -> Value {
    let endpoint = |key: &str| {
        value[key]["dateTime"]
            .as_str()
            .or_else(|| value[key]["date"].as_str())
    };
    compact(
        json!({"id":value["id"],"title":value["summary"],"start":endpoint("start"),"end":endpoint("end"),"location":value["location"],"status":value["status"],"description":value["description"],"url":value["htmlLink"]}),
    )
}
fn task(value: &Value) -> Value {
    compact(
        json!({"id":value["id"],"title":value["title"],"due":value["due"].as_str().map(|due|due.split('T').next().unwrap_or(due)),"status":value["status"],"notes":value["notes"]}),
    )
}
pub fn plain_value(service: &str, action: &str, value: &Value) -> Result<Value> {
    Ok(match (service, action) {
        ("mail", "labels") => rows(value, "labels", |label| {
            compact(json!({"id":label["id"],"name":label["name"],"type":label["type"]}))
        }),
        ("calendar", "calendars") => rows(value, "items", |calendar| {
            compact(
                json!({"id":calendar["id"],"name":calendar["summary"],"timezone":calendar["timeZone"],"access":calendar["accessRole"],"primary":calendar["primary"]}),
            )
        }),
        ("calendar", "events") => rows(value, "items", |value| {
            let mut row = event(value);
            row.as_object_mut().unwrap().remove("description");
            row
        }),
        ("calendar", "get") => event(value),
        ("tasks", "lists") => rows(value, "items", |list| {
            compact(json!({"id":list["id"],"title":list["title"]}))
        }),
        ("tasks", "list") => rows(value, "items", task),
        ("tasks", "get") => task(value),
        ("contacts", "list") => rows(value, "connections", contact),
        ("contacts", "search") => rows(value, "results", |result| contact(&result["person"])),
        ("contacts", "get") => contact(value),
        ("mail", "get") => message(value)?,
        ("mail", "thread") => {
            json!({"id":value["id"],"messages":value["messages"].as_array().map(|messages|messages.iter().map(message).collect::<Result<Vec<_>>>()).transpose()?.unwrap_or_default()})
        }
        ("sheets", "values") => value.get("values").cloned().unwrap_or_else(|| json!([])),
        ("sheets", "get") if value.get("values").is_some() => value["values"].clone(),
        ("sheets", "get") => compact(
            json!({"id":value["spreadsheetId"],"title":value.pointer("/properties/title"),"url":value["spreadsheetUrl"],"sheets":value["sheets"].as_array().map(|sheets|sheets.iter().map(|sheet|compact(json!({"id":sheet.pointer("/properties/sheetId"),"title":sheet.pointer("/properties/title"),"rows":sheet.pointer("/properties/gridProperties/rowCount"),"columns":sheet.pointer("/properties/gridProperties/columnCount")}))).collect::<Vec<_>>())}),
        ),
        _ => value.clone(),
    })
}
fn success(service: &str, action: &str) -> Option<&'static str> {
    Some(match (service, action) {
        ("mail", "send") => "Message sent.",
        ("mail", "trash") => "Message moved to Trash.",
        ("mail", "restore") => "Message restored.",
        ("mail", "read") => "Message marked as read.",
        ("mail", "unread") => "Message marked as unread.",
        ("mail", "star") => "Message starred.",
        ("mail", "unstar") => "Message unstarred.",
        ("drive", "upload") => "File uploaded.",
        ("drive", "mkdir") => "Folder created.",
        ("drive", "rename") => "File renamed.",
        ("drive", "copy") => "File copied.",
        ("drive", "trash") => "File moved to Trash.",
        ("drive", "restore") => "File restored.",
        ("calendar", "create") => "Event created.",
        ("calendar", "update") => "Event updated.",
        ("calendar", "delete") => "Event deleted.",
        ("sheets", "create") => "Spreadsheet created.",
        ("sheets", "write") => "Cells updated.",
        ("sheets", "append") => "Rows appended.",
        ("sheets", "clear") => "Cells cleared.",
        ("tasks", "create-list") => "Task list created.",
        ("tasks", "add") => "Task created.",
        ("tasks", "update") => "Task updated.",
        ("tasks", "complete") => "Task completed.",
        ("tasks", "reopen") => "Task reopened.",
        ("tasks", "delete") => "Task deleted.",
        ("contacts", "create") => "Contact created.",
        ("contacts", "delete") => "Contact deleted.",
        _ => return None,
    })
}
pub(crate) fn method_action<'a>(service: &str, action: &'a str) -> &'a str {
    match (service, action) {
        ("mail", "messages.get") => "get",
        ("mail", "threads.get") => "thread",
        ("mail", "labels.list") => "labels",
        ("mail", "messages.send" | "drafts.send") => "send",
        ("mail", "messages.trash") => "trash",
        ("mail", "messages.untrash") => "restore",
        ("calendar", "calendars.list") => "calendars",
        ("calendar", "events.list") => "events",
        ("calendar", "events.get") => "get",
        ("sheets", "values.get") => "values",
        ("sheets", "get") => "get",
        ("tasks", "tasklists.list") => "lists",
        ("tasks", "tasks.list") => "list",
        ("tasks", "tasks.get") => "get",
        ("contacts", "people.connections.list") => "list",
        ("contacts", "people.search-contacts") => "search",
        ("contacts", "people.get") => "get",
        _ => action,
    }
}

pub fn emit(
    service: &str,
    action: Option<&str>,
    value: &Value,
    json: bool,
    paginated: bool,
) -> Result<()> {
    if json {
        return output::emit(value, true, paginated);
    }
    if let Some(action) = action {
        if crate::schema_commands::contract(service)
            .document
            .root
            .methods
            .get(action)
            .is_some_and(|method| method.batch_operation.is_some())
        {
            return output::status(
                if service == "docs" {
                    "Document updated."
                } else {
                    "Presentation updated."
                },
                value,
                false,
            );
        }
        let action = method_action(service, action);
        if let Some(message) = success(service, action) {
            let id = [
                "id",
                "name",
                "documentId",
                "presentationId",
                "spreadsheetId",
                "formId",
            ]
            .iter()
            .find_map(|key| value[*key].as_str());
            let message = match id {
                Some(id) => format!("{message} ID: {id}"),
                None => message.into(),
            };
            return output::status(&message, value, false);
        }
        return output::emit(&plain_value(service, action, value)?, false, paginated);
    }
    if value.is_null() {
        output::status("Done.", value, false)
    } else {
        output::emit(value, false, paginated)
    }
}

#[cfg(test)]
#[path = "../tests/unit/presentation.rs"]
mod tests;
