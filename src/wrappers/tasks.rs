use super::{Request, backend::Backend, values};
use anyhow::{Context, Result, bail, ensure};
use jiff::{SignedDuration, Timestamp, ToSpan, ZonedDateTime, tz::TimeZone};
use serde_json::{Map, Value, json};

mod mail;

fn parse_instant(text: &str) -> Result<Timestamp> {
    text.parse()
        .context("Microsoft returned an invalid timestamp")
}

fn rows<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value[key].as_array().map(Vec::as_slice).unwrap_or_default()
}
fn id<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .with_context(|| format!("Microsoft did not return {field}"))
}
fn view(service: &str, command: &str, value: Value) -> Result<Value> {
    crate::presentation::plain_value(
        service,
        crate::presentation::method_action(service, command),
        &value,
    )
}
fn changed(value: Value, message: &str) -> Value {
    if value.is_null() {
        json!({"status":message})
    } else {
        let mut value = value;
        if let Some(object) = value.as_object_mut() {
            object.insert("confirmation".into(), json!(message));
        }
        value
    }
}
fn escaped(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\'', "\\'")
}
fn file_type(text: &str) -> &str {
    match text {
        "folder" => "inode/directory",
        "document" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "spreadsheet" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "presentation" => {
            "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        }
        "pdf" => "application/pdf",
        other => other,
    }
}
fn list_files(r: &Request, api: &mut impl Backend, mime: Option<&str>) -> Result<Value> {
    let mut terms = vec!["trashed = false".to_owned()];
    if let Some(folder) = r
        .text("folder")
        .or((r.wrapper == "jmdrive" && r.command == "ls").then_some("root"))
    {
        terms.push(format!("'{}' in parents", escaped(folder)));
    }
    if let Some(mime) = mime.or_else(|| r.text("type").map(file_type)) {
        terms.push(if mime == "image" {
            "mimeType contains 'image/'".into()
        } else {
            format!("mimeType = '{}'", escaped(mime))
        });
    }
    if let Some(query) = r.text("query") {
        terms.push(format!("fullText contains '{}'", escaped(query)));
    }
    if let Some(name) = r.text("name") {
        terms.push(format!("name contains '{}'", escaped(name)));
    }
    let value = api.call("drive", "files.list", r.list_params(json!({"q":terms.join(" and "),"fields":"nextPageToken,files(id,name,mimeType,modifiedTime,webViewLink,parents)","orderBy":"modifiedTime desc"})))?;
    view("drive", "files.list", value)
}

fn place(api: &mut impl Backend, target: &str, folder: &str, name: Option<&str>) -> Result<Value> {
    let original = api.call(
        "drive",
        "files.get",
        json!({"fileId":target,"fields":"id,parents"}),
    )?;
    let parents = rows(&original, "parents")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let mut params = json!({"fileId":target});
    if !parents.contains(&folder) {
        params["addParents"] = json!(folder);
        if !parents.is_empty() {
            params["removeParents"] = json!(parents.join(","));
        }
    }
    if let Some(name) = name {
        params["name"] = json!(name);
    }
    // Moving to the current parent with no rename needs no mutation.
    if params.as_object().unwrap().len() == 1 {
        return Ok(changed(original, "File is already in this folder."));
    }
    params["body"] = json!({});
    api.call("drive", "files.update", params)
}

fn export(r: &Request, api: &mut impl Backend) -> Result<Value> {
    api.call("drive", "files.export", json!({"fileId":r.target(),"mimeType":values::mime(r.text("format").unwrap_or("pdf"))?,"output":r.text("out").unwrap()}))
}

fn share(r: &Request, api: &mut impl Backend) -> Result<Value> {
    api.call("drive", "permissions.create", json!({"fileId":r.target(),"type":"user","role":r.text("role").unwrap_or("reader"),"emailAddress":r.text("to").unwrap(),"sendNotificationEmail":r.params.get("notify").and_then(Value::as_bool).unwrap_or(true)}))
}

pub fn execute(r: &Request, api: &mut impl Backend) -> Result<Value> {
    match r.wrapper.as_str() {
        "jmmail" => mail::execute(r, api),
        "jmdrive" => drive(r, api),
        "jmcal" => calendar(r, api),
        "jmsheets" => sheets(r, api),
        "jmdocs" => docs(r, api),
        "jmslides" => slides(r, api),
        "jmtasks" => tasks(r, api),
        "jmcontacts" => contacts(r, api),
        _ => bail!("unknown wrapper"),
    }
}

fn drive(r: &Request, api: &mut impl Backend) -> Result<Value> {
    Ok(match r.command.as_str() {
        "ls" | "find" => return list_files(r, api, None),
        "info" => api.call("drive", "files.get", json!({"fileId":r.target(),"fields":"id,name,mimeType,size,parents,owners,permissions,webViewLink"}))?,
        "get" => {
            api.call("drive", "files.get", json!({"fileId":r.target(),"output":r.text("out").unwrap()}))?
        }
        "put" => {
            let path = r.text("path").unwrap();
            let name = r.text("name").map(str::to_owned).unwrap_or_else(|| std::path::Path::new(path).file_name().unwrap().to_string_lossy().into_owned());
            let mut params = json!({"name":name,"upload":path,"contentType":values::upload_mime(path)});
            if let Some(folder) = r.text("folder") { params["parents"] = json!([folder]); }
            changed(api.call("drive", "files.create", params)?, "File uploaded.")
        }
        "mkdir" => {
            let mut params = json!({"name":r.text("name").unwrap(),"mimeType":"inode/directory"});
            if let Some(folder) = r.text("folder") { params["parents"] = json!([folder]); }
            changed(api.call("drive", "files.create", params)?, "Folder created.")
        }
        "move" => {
            let value = if let Some(folder) = r.text("to") { place(api, r.target(), folder, r.text("name"))? }
            else { api.call("drive", "files.update", json!({"fileId":r.target(),"name":r.text("name").unwrap()}))? };
            changed(value, "File updated.")
        }
        "copy" => {
            let mut params = json!({"fileId":r.target(),"body":{}});
            if let Some(folder) = r.text("to") { params["parents"] = json!([folder]); }
            if let Some(name) = r.text("name") { params["name"] = json!(name); }
            changed(api.call("drive", "files.copy", params)?, "File copied.")
        }
        "share" => changed(share(r, api)?, "Access granted."),
        "trash" | "restore" => changed(api.call("drive", "files.update", json!({"fileId":r.target(),"trashed":r.command == "trash"}))?, if r.command == "trash" { "File moved to Trash." } else { "File restored." }),
        _ => bail!("unknown Drive command"),
    })
}

fn calendar_zone(r: &Request, api: &mut impl Backend) -> Result<TimeZone> {
    if let Some(zone) = r.text("timezone") {
        return TimeZone::get(zone).context("unknown timezone");
    }
    let calendar = api.call(
        "calendar",
        "calendars.get",
        json!({"calendarId":r.text("calendar").unwrap_or("primary")}),
    )?;
    let Some(name) = calendar["timeZone"]
        .as_str()
        .filter(|name| !name.is_empty())
    else {
        return TimeZone::try_system().context(
            "Microsoft returned no mailbox timezone and the computer timezone could not be determined; supply timezone:AREA/CITY",
        );
    };
    TimeZone::get(name).context("Microsoft returned an unknown calendar timezone; supply timezone:")
}

fn calendar(r: &Request, api: &mut impl Backend) -> Result<Value> {
    let calendar = r.text("calendar").unwrap_or("primary");
    if r.command == "calendars" {
        let value = api.call("calendar", "calendars.list", r.list_params(json!({})))?;
        return view("calendar", "calendars.list", value);
    }
    if r.command == "show" {
        let value = api.call(
            "calendar",
            "events.get",
            json!({"calendarId":calendar,"eventId":r.target()}),
        )?;
        return view("calendar", "events.get", value);
    }
    if r.command == "cancel" {
        return Ok(changed(api.call("calendar", "events.delete", json!({"calendarId":calendar,"eventId":r.target(),"sendUpdates":if r.flag("notify") { "all" } else { "none" }}))?, "Event cancelled."));
    }
    let zone = calendar_zone(r, api)?;
    let now = Timestamp::now();
    let today = now.to_zoned_datetime(zone.clone()).date();
    let zone_name = zone
        .iana_name()
        .context("calendar timezone has no IANA name")?;
    match r.command.as_str() {
        "today" | "week" | "find" => {
            let mut params = json!({"calendarId":calendar,"orderBy":"startTime"});
            if r.command != "find" {
                let start = if r.command == "week" {
                    today.checked_sub(i64::from(today.weekday().to_monday_zero_offset()).days())?
                } else {
                    today
                };
                let end = start.checked_add(if r.command == "week" {
                    7.days()
                } else {
                    1.day()
                })?;
                params["timeMin"] = json!(values::midnight(start, &zone)?.to_string());
                params["timeMax"] = json!(values::midnight(end, &zone)?.to_string());
            } else {
                params["q"] = json!(r.text("query").unwrap());
                if let Some(from) = r.text("from") {
                    params["timeMin"] =
                        json!(values::midnight(values::date(from, today)?, &zone)?.to_string());
                }
                if let Some(until) = r.text("until") {
                    params["timeMax"] = json!(
                        values::midnight(values::date(until, today)?.tomorrow()?, &zone)?
                            .to_string()
                    );
                }
                if let (Some(from), Some(until)) =
                    (params["timeMin"].as_str(), params["timeMax"].as_str())
                {
                    ensure!(
                        parse_instant(from)? < parse_instant(until)?,
                        "until: must be on or after from: in the calendar timezone"
                    );
                }
                params = r.list_params(params);
            }
            view(
                "calendar",
                "events.list",
                api.call("calendar", "events.list", params)?,
            )
        }
        "add" => {
            let mut body = json!({"summary":r.text("title").unwrap()});
            if let Some(on) = r.text("on") {
                let day = values::date(on, today)?;
                body["start"] = json!({"date":day.to_string()});
                body["end"] = json!({"date":day.tomorrow()?.to_string()});
            } else {
                let start = values::timestamp(r.text("at").unwrap(), &zone, now)?;
                let end = start
                    .checked_add(values::duration(
                        r.text("for").expect("timed event duration default"),
                    )?)
                    .context("event end is outside its allowed range")?;
                body["start"] = json!({"dateTime":start.to_string(),"timeZone":zone_name});
                body["end"] = json!({"dateTime":end.to_string(),"timeZone":zone_name});
            }
            if let Some(where_) = r.text("where") {
                body["location"] = json!(where_);
            }
            let attendees = r.strings("with");
            if !attendees.is_empty() {
                body["attendees"] = json!(
                    attendees
                        .iter()
                        .map(|email| json!({"email":email}))
                        .collect::<Vec<_>>()
                );
            }
            Ok(changed(
                api.call(
                    "calendar",
                    "events.insert",
                    json!({"calendarId":calendar,"sendUpdates":"all","body":body}),
                )?,
                "Event created.",
            ))
        }
        "move" => {
            let original = api.call(
                "calendar",
                "events.get",
                json!({"calendarId":calendar,"eventId":r.target()}),
            )?;
            ensure!(
                original.pointer("/start/date").is_none(),
                "use an all-day event's date fields with joom to reschedule it as all-day; at: moves timed events"
            );
            let duration = if let Some(duration) = r.text("for") {
                values::duration(duration)?
            } else {
                parse_instant(
                    original
                        .pointer("/end/dateTime")
                        .and_then(Value::as_str)
                        .context("event has no end time")?,
                )? - parse_instant(
                    original
                        .pointer("/start/dateTime")
                        .and_then(Value::as_str)
                        .context("event has no start time")?,
                )?
            };
            let start = values::timestamp(r.text("at").unwrap(), &zone, now)?;
            let end = start
                .checked_add(duration)
                .context("event end is outside its allowed range")?;
            Ok(changed(api.call("calendar", "events.patch", json!({"calendarId":calendar,"eventId":r.target(),"sendUpdates":"all","body":{"start":{"dateTime":start.to_string(),"timeZone":zone_name},"end":{"dateTime":end.to_string(),"timeZone":zone_name}}}))?, "Event rescheduled."))
        }
        "free" => {
            let day = values::date(r.text("on").unwrap(), today)?;
            let start = zone
                .to_ambiguous_zoned_datetime(day.at(9, 0, 0, 0))
                .unambiguous()
                .context("working hours are ambiguous in this timezone")?
                .timestamp();
            let end = zone
                .to_ambiguous_zoned_datetime(day.at(18, 0, 0, 0))
                .unambiguous()
                .context("working hours are ambiguous in this timezone")?
                .timestamp();
            let duration = values::duration(r.text("for").unwrap())?;
            let mut calendars = r.strings("with");
            calendars.push(calendar.into());
            calendars.sort();
            calendars.dedup();
            let result = api.call("calendar", "freebusy.query", json!({"body":{"timeMin":start.to_string(),"timeMax":end.to_string(),"items":calendars.iter().map(|id| json!({"id":id})).collect::<Vec<_>>()}}))?;
            let mut busy = Vec::new();
            for calendar in &calendars {
                let item = result["calendars"]
                    .get(calendar)
                    .context("Microsoft omitted availability for a requested calendar")?;
                ensure!(
                    rows(item, "errors").is_empty(),
                    "availability is unavailable for {calendar}; check calendar sharing"
                );
                for slot in rows(item, "busy") {
                    busy.push((
                        parse_instant(id(slot, "start")?)?,
                        parse_instant(id(slot, "end")?)?,
                    ));
                }
            }
            let mut slots = Vec::new();
            let mut cursor = start;
            while let Ok(finish) = cursor.checked_add(duration) {
                if finish > end {
                    break;
                }
                if busy
                    .iter()
                    .all(|(start, end)| finish <= *start || cursor >= *end)
                {
                    slots.push(json!({"start":values::rfc3339(cursor, &zone),"end":values::rfc3339(finish, &zone)}));
                }
                cursor = cursor.checked_add(SignedDuration::from_secs(900))?;
            }
            Ok(json!(slots))
        }
        _ => bail!("unknown Calendar command"),
    }
}

fn range(r: &Request, api: &mut impl Backend, selected: &str) -> Result<String> {
    if selected.contains('!') {
        ensure!(
            r.text("tab").is_none(),
            "select a tab with tab: or inside the A1 range, choosing one"
        );
        return Ok(selected.into());
    }
    let tab = if let Some(tab) = r.text("tab") {
        tab.to_owned()
    } else {
        let spreadsheet = api.call(
            "sheets",
            "get",
            json!({"spreadsheetId":r.target(),"fields":"sheets.properties"}),
        )?;
        spreadsheet
            .pointer("/sheets/0/properties/title")
            .and_then(Value::as_str)
            .context("spreadsheet has no tabs")?
            .into()
    };
    let tab = format!("'{}'", tab.replace('\'', "''"));
    Ok(if selected.is_empty() {
        tab
    } else {
        format!("{tab}!{selected}")
    })
}

fn sheets(r: &Request, api: &mut impl Backend) -> Result<Value> {
    if r.command == "ls" {
        return list_files(r, api, Some(file_type("spreadsheet")));
    }
    if r.command == "new" {
        let mut params = json!({"title":r.text("title").unwrap()});
        if let Some(tab) = r.text("tab") {
            params["sheets"] = json!([{"properties":{"title":tab}}]);
        }
        return Ok(changed(
            api.call("sheets", "create", params)?,
            "Spreadsheet created.",
        ));
    }
    if r.command == "export" && r.text("format") != Some("csv") {
        return export(r, api);
    }
    let selected = r
        .text("range")
        .or_else(|| r.text("cell"))
        .or_else(|| r.text("at"))
        .unwrap_or(if matches!(r.command.as_str(), "export" | "append") {
            ""
        } else {
            "A1:Z20"
        });
    let selected = range(r, api, selected)?;
    let base = json!({"spreadsheetId":r.target(),"range":selected});
    match r.command.as_str() {
        "read" => view(
            "sheets",
            "values.get",
            api.call("sheets", "values.get", base)?,
        ),
        "export" => {
            let value = api.call("sheets", "values.get", base)?;
            let rows = rows(&value, "values")
                .iter()
                .map(|row| {
                    row.as_array()
                        .into_iter()
                        .flatten()
                        .map(|cell| {
                            cell.as_str()
                                .map(str::to_owned)
                                .unwrap_or_else(|| cell.to_string())
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            values::save(&values::csv_bytes(&rows)?, r.text("out").unwrap())
        }
        "clear" => Ok(changed(
            api.call("sheets", "values.clear", base)?,
            "Cells cleared.",
        )),
        "set" | "append" | "load" => {
            let mut params = base;
            let data = if r.command == "set" {
                vec![vec![r.text("value").unwrap().to_owned()]]
            } else {
                values::csv_rows(&r.params)?
            };
            params["values"] = json!(data);
            if r.command == "append" {
                params["insertDataOption"] = json!("INSERT_ROWS");
            }
            let method = if r.command == "append" {
                "values.append"
            } else {
                "values.update"
            };
            Ok(changed(
                api.call("sheets", method, params)?,
                "Cells updated.",
            ))
        }
        _ => bail!("unknown Sheets command"),
    }
}

fn docs(r: &Request, api: &mut impl Backend) -> Result<Value> {
    match r.command.as_str() {
        "ls" => list_files(r, api, Some(file_type("document"))),
        "export" => export(r, api),
        "share" => Ok(changed(share(r, api)?, "Access granted.")),
        _ => bail!("unknown Word command"),
    }
}

fn slides(r: &Request, api: &mut impl Backend) -> Result<Value> {
    match r.command.as_str() {
        "ls" => list_files(r, api, Some(file_type("presentation"))),
        "export" => export(r, api),
        _ => bail!("unknown PowerPoint command"),
    }
}

fn tasks(r: &Request, api: &mut impl Backend) -> Result<Value> {
    let list = r.text("list").unwrap_or("@default");
    let command = match r.command.as_str() {
        "ls" => "tasks.list",
        "add" => "tasks.insert",
        "show" => "tasks.get",
        "done" | "undo" | "set" => "tasks.patch",
        "delete" => "tasks.delete",
        "lists" => "tasklists.list",
        "new-list" => "tasklists.insert",
        _ => bail!("unknown Tasks command"),
    };
    let mut params = if matches!(r.command.as_str(), "lists" | "new-list") {
        json!({})
    } else {
        json!({"tasklist":list})
    };
    if let Some(target) = r.text("target") {
        params["task"] = json!(target);
    }
    if let Some(title) = r.text("title") {
        params["title"] = json!(title);
    }
    if let Some(notes) = r.text("notes") {
        params["notes"] = json!(notes);
    }
    if let Some(due) = r.text("due") {
        let due = values::date(due, ZonedDateTime::now().date())?;
        if r.command == "ls" {
            params["dueMin"] = json!(format!("{due}T00:00:00Z"));
            params["dueMax"] = json!(format!("{}T00:00:00Z", due.tomorrow()?));
        } else {
            params["due"] = json!(format!("{due}T00:00:00Z"));
        }
    }
    if r.command == "ls" {
        params["showCompleted"] = json!(r.flag("completed"));
    }
    if r.command == "done" {
        params["status"] = json!("completed");
    }
    if r.command == "undo" {
        params["status"] = json!("needsAction");
        params["completed"] = Value::Null;
    }
    if matches!(r.command.as_str(), "ls" | "lists") {
        params = r.list_params(params);
    }
    let value = api.call("tasks", command, params)?;
    if matches!(r.command.as_str(), "ls" | "lists" | "show") {
        view("tasks", command, value)
    } else {
        Ok(changed(value, "Task updated."))
    }
}

fn contacts(r: &Request, api: &mut impl Backend) -> Result<Value> {
    if r.command == "ls" {
        let value = api.call(
            "contacts",
            "people.connections.list",
            r.list_params(json!({"resourceName":"people/me"})),
        )?;
        return view("contacts", "people.connections.list", value);
    }
    if r.command == "find" {
        let value = api.call(
            "contacts",
            "people.search-contacts",
            r.list_params(json!({"query":r.text("query").unwrap()})),
        )?;
        return view("contacts", "people.search-contacts", value);
    }
    if r.command == "show" {
        let value = api.call("contacts", "people.get", json!({"resourceName":r.target()}))?;
        return view("contacts", "people.get", value);
    }
    if r.command == "delete" {
        return Ok(changed(
            api.call(
                "contacts",
                "people.delete-contact",
                json!({"resourceName":r.target()}),
            )?,
            "Contact deleted.",
        ));
    }
    let mut body = Map::new();
    if let Some(name) = r.text("name") {
        body.insert("names".into(), json!([{"unstructuredName":name}]));
    }
    for (field, wire) in [("email", "emailAddresses"), ("phone", "phoneNumbers")] {
        if r.params.contains_key(field) {
            body.insert(
                wire.into(),
                json!(
                    r.strings(field)
                        .iter()
                        .map(|value| json!({"value":value}))
                        .collect::<Vec<_>>()
                ),
            );
        }
    }
    if let Some(company) = r.text("company") {
        body.insert("organizations".into(), json!([{"name":company}]));
    }
    let value = if r.command == "new" {
        api.call("contacts", "people.create-contact", json!({"body":body}))?
    } else {
        let current = api.call("contacts", "people.get", json!({"resourceName":r.target()}))?;
        if let Some(etag) = current["etag"].as_str() {
            body.insert("etag".into(), json!(etag));
        }
        api.call(
            "contacts",
            "people.update-contact",
            json!({"resourceName":r.target(),"body":body}),
        )?
    };
    view("contacts", "people.get", changed(value, "Contact saved."))
}
