use super::*;

#[test]
fn personal_drive_listing_keeps_the_canonical_personal_identifier() {
    let drive = json!({"id":"123ABC","driveType":"personal","webUrl":"https://onedrive.live.com/"});
    let list = personal_drive_listing(&drive).unwrap();
    assert_eq!(list["drives"][0], drive);
    assert!(personal_drive_listing(&json!({"id":"b!business","driveType":"business"})).is_none());
}

#[test]
fn graph_search_query_uses_odata_names_and_percent_encoded_spaces() {
    let r = request(
        "drive",
        "files.list",
        json!({"q":"name contains 'fixture'", "orderBy":"modifiedTime desc"}),
        None,
    )
    .unwrap();
    assert!(
        r.url
            .as_str()
            .contains("?$orderby=lastModifiedDateTime%20desc")
    );
    let mut r = r;
    query(&mut r, "$search", "a+b c");
    assert!(r.url.as_str().contains("$search=a%2Bb%20c"));
}

#[test]
fn mailbox_timezone_accepts_property_and_primitive_graph_responses() {
    for value in [
        json!({"timeZone":"Romance Standard Time"}),
        json!({"value":"Romance Standard Time"}),
        json!("Romance Standard Time"),
    ] {
        assert_eq!(iana_zone(mailbox_zone(&value).unwrap()), "Europe/Paris");
    }
    for value in [
        Value::Null,
        json!({}),
        json!({"timeZone":null}),
        json!({"timeZone":""}),
    ] {
        assert!(mailbox_zone(&value).is_err());
    }
}

#[test]
fn every_exposed_command_has_an_audited_microsoft_route() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/microsoft-routes.json"
    )))
    .unwrap();
    let native: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/native-requests.json"
    )))
    .unwrap();
    let mut report = Vec::new();
    let mut exercised = std::collections::BTreeSet::new();
    for service in crate::services::SERVICES {
        let contract = crate::schema_commands::contract(service.cli);
        for command in contract.inputs.keys() {
            let spec = contract
                .document
                .method(command)
                .or_else(|_| {
                    let method = native[service.cli]["x-expected-methods"][command]
                        .as_str()
                        .unwrap();
                    contract.document.method(method)
                })
                .unwrap();
            if let Some(fixture) = fixtures.iter().find(|f| f["id"] == spec.id) {
                let p = fixture["params"].as_object().unwrap();
                let body = fixture.get("body").filter(|b| !b.is_null());
                let mode = fixture["mode"].as_str().unwrap();
                if mode == "download" {
                    validate(&contract.document, spec, p, body, false, true).unwrap();
                } else {
                    let r = plan(spec, p, body)
                        .unwrap_or_else(|error| panic!("{} {command}: {error:#}", service.cli));
                    assert_eq!(
                        r.method.as_str(),
                        fixture["method"].as_str().unwrap(),
                        "{} {command}",
                        service.cli
                    );
                    assert_eq!(
                        r.url.path(),
                        fixture["path"].as_str().unwrap(),
                        "{} {command}",
                        service.cli
                    );
                    graph_url(&r.url).unwrap();
                }
                exercised.insert(spec.id.clone());
                report.push(json!({"service":service.cli,"command":command,"providerMethod":spec.id,"status":"mapped","verification":"local request planning; not live"}));
            } else {
                panic!("{} {command} has no Microsoft route fixture", service.cli);
            }
        }
    }
    assert_eq!(report.len(), 77);
    assert_eq!(
        exercised.len(),
        fixtures.len(),
        "route fixture is absent from the CLI catalog"
    );
    if let Ok(directory) = std::env::var("JOOM_AUDIT_DIRECTORY") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join("commands.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
}

fn request(service: &str, command: &str, params: Value, body: Option<Value>) -> Result<Request> {
    let document = &crate::schema_commands::contract(service).document;
    plan(
        document.method(command)?,
        params.as_object().unwrap(),
        body.as_ref(),
    )
}

#[test]
fn bearer_tokens_are_restricted_to_graph_and_reject_other_origins() {
    graph_url(&Url::parse("https://graph.microsoft.com/v1.0/me/messages").unwrap()).unwrap();
    for address in [
        "https://api.example.com/v1/messages",
        "https://graph.microsoft.com.attacker.test/v1.0/me",
        "https://attacker.graph.microsoft.com/v1.0/me",
        "http://graph.microsoft.com/v1.0/me",
        "https://user@graph.microsoft.com/v1.0/me",
        "https://graph.microsoft.com:444/v1.0/me",
        "https://graph.microsoft.com/v1.0/me#fragment",
        "https://graph.microsoft.com/beta/me",
    ] {
        assert!(
            graph_url(&Url::parse(address).unwrap()).is_err(),
            "{address}"
        );
        assert!(
            Http::new(1)
                .unwrap()
                .json(
                    &Method::GET,
                    &Url::parse(address).unwrap(),
                    None,
                    Some("synthetic-token"),
                    1024
                )
                .is_err()
        );
    }
}

#[test]
fn unchanged_named_commands_resolve_to_microsoft() {
    for (args, path) in [
        (
            vec![
                "joom",
                "mail",
                "me@example.com",
                "messages.list",
                "--q",
                "in:inbox is:unread",
            ],
            "/v1.0/me/mailFolders/inbox/messages",
        ),
        (
            vec![
                "joom",
                "drive",
                "me@example.com",
                "files.get",
                "--file-id",
                "ABC!123",
            ],
            "/v1.0/me/drive/items/ABC%21123",
        ),
        (
            vec!["joom", "tasks", "me@example.com", "tasklists.list"],
            "/v1.0/me/todo/lists",
        ),
        (
            vec![
                "joom",
                "contacts",
                "me@example.com",
                "people.get",
                "--resource-name",
                "people/ABC=",
            ],
            "/v1.0/me/contacts/ABC%3D",
        ),
    ] {
        let root = crate::cli::parse_from(args).unwrap();
        let (service, context) = root.subcommand().unwrap();
        let service = crate::services::SERVICES
            .iter()
            .find(|s| s.cli == service)
            .unwrap();
        let call = crate::service_call(service, context).unwrap();
        let spec = crate::schema_commands::contract(service.cli)
            .document
            .method(&call.method)
            .unwrap();
        let r = plan(spec, &call.params, call.body.as_ref()).unwrap();
        assert_eq!(r.url.host_str(), Some("graph.microsoft.com"));
        assert_eq!(r.url.path(), path);
    }
}

#[test]
fn pagination_uses_the_entire_link_and_validates_the_collection() {
    let next = "https://graph.microsoft.com/v1.0/me/messages?$skiptoken=a%2Bb%3D&$top=2";
    let r = request("mail", "messages.list", json!({"pageToken":next}), None).unwrap();
    assert_eq!(r.url.as_str(), next);
    for next in [
        "https://attacker.test/v1.0/me/messages",
        "https://graph.microsoft.com/v1.0/me/contacts",
        "http://graph.microsoft.com/v1.0/me/messages",
    ] {
        assert!(request("mail", "messages.list", json!({"pageToken":next}), None).is_err());
    }
}

#[test]
fn folder_pagination_accepts_odata_keys_and_preserves_collection_boundaries() {
    let first = request("mail", "messages.list", json!({"q":"in:inbox"}), None).unwrap();
    let next = "https://graph.microsoft.com/v1.0/me/mailFolders('inbox')/messages?$top=2&$skiptoken=a%2Bb%3D";
    assert_eq!(collection_page(&first.url, next).unwrap().as_str(), next);
    let continued = request(
        "mail",
        "messages.list",
        json!({"q":"in:inbox","pageToken":next}),
        None,
    )
    .unwrap();
    assert_eq!(continued.url.as_str(), next);
    let slash = "https://graph.microsoft.com/v1.0/me/mailFolders/inbox/messages?$skip=4";
    assert_eq!(
        collection_page(&continued.url, slash).unwrap().as_str(),
        slash
    );
    for next in [
        "https://graph.microsoft.com/v1.0/me/mailFolders('sentitems')/messages",
        "https://graph.microsoft.com/v1.0/me/messages",
        "https://graph.microsoft.com/v1.0/users('other-user')/mailFolders('inbox')/messages",
        "https://graph.microsoft.com/v1.0/me/mailFolders('inbox')/contacts",
        "https://graph.microsoft.com/v1.0/me/mailFolders('inbox%2Fmessages')",
        "https://graph.microsoft.com/v1.0/me/mailFolders('inbox')/messages#fragment",
        "https://attacker.test/v1.0/me/mailFolders('inbox')/messages",
    ] {
        assert!(collection_page(&first.url, next).is_err(), "{next}");
        assert!(
            request(
                "mail",
                "messages.list",
                json!({"q":"in:inbox","pageToken":next}),
                None
            )
            .is_err(),
            "{next}"
        );
    }
    let keyed =
        Url::parse("https://graph.microsoft.com/v1.0/me/mailFolders('a%2Fb''c')/messages").unwrap();
    let slash = "https://graph.microsoft.com/v1.0/me/mailFolders/a%2Fb%27c/messages";
    assert_eq!(collection_page(&keyed, slash).unwrap().as_str(), slash);
    assert!(
        collection_page(
            &keyed,
            "https://graph.microsoft.com/v1.0/me/mailFolders/a/b%27c/messages"
        )
        .is_err()
    );
}

#[test]
fn send_keeps_mime_and_attachments_and_draft_shape() {
    let mime = "To: friend@example.com\r\nSubject: Hello\r\n\r\nMessage";
    let body = json!({"raw":URL_SAFE_NO_PAD.encode(mime)});
    let send = request("mail", "messages.send", json!({}), Some(body.clone())).unwrap();
    assert_eq!(send.url.path(), "/v1.0/me/sendMail");
    assert_eq!(send.method, Method::POST);
    assert_eq!(send.mime, Some(STANDARD.encode(mime)));
    let draft = request(
        "mail",
        "drafts.create",
        json!({}),
        Some(json!({"message":body})),
    )
    .unwrap();
    assert_eq!(draft.url.path(), "/v1.0/me/messages");
    assert_eq!(draft.shape, Shape::Draft);
}

#[test]
fn unread_stars_trash_and_permanent_delete_keep_distinct_semantics() {
    let modified = request(
        "mail",
        "messages.modify",
        json!({"id":"AAMk=="}),
        Some(json!({"removeLabelIds":["UNREAD"],"addLabelIds":["STARRED"]})),
    )
    .unwrap();
    assert_eq!(modified.method, Method::PATCH);
    assert_eq!(
        modified.body,
        Some(json!({"isRead":true,"flag":{"flagStatus":"flagged"}}))
    );
    let trash = request("mail", "messages.trash", json!({"id":"AAMk=="}), None).unwrap();
    assert_eq!(trash.body.unwrap()["destinationId"], "deleteditems");
    let deleted = request("mail", "messages.delete", json!({"id":"AAMk=="}), None).unwrap();
    assert_eq!(deleted.method, Method::POST);
    assert!(deleted.url.path().ends_with("/permanentDelete"));
    assert!(
        request(
            "mail",
            "messages.modify",
            json!({"id":"x"}),
            Some(json!({"addLabelIds":["arbitrary-label"]}))
        )
        .is_err()
    );
}

#[test]
fn microsoft_messages_still_render_through_the_existing_presenter() {
    let value = response(
        Shape::Message,
        &json!({"id":"m1","conversationId":"c1","subject":"Hello","from":{"emailAddress":{"address":"sender@example.com"}},"toRecipients":[{"emailAddress":{"address":"recipient@example.com"}}],"isRead":false,"body":{"contentType":"text","content":"Body"},"attachments":[{"id":"a1","name":"report.pdf","contentType":"application/pdf","size":8}]}),
    );
    let plain = crate::presentation::plain_value("mail", "get", &value).unwrap();
    assert_eq!(plain["subject"], "Hello");
    assert_eq!(plain["from"], "sender@example.com");
    assert_eq!(plain["text"], "Body");
    assert_eq!(value["threadId"], "c1");
    assert_eq!(value["labelIds"], json!(["UNREAD"]));
    assert_eq!(value["payload"]["parts"][1]["body"]["attachmentId"], "a1");
}

#[test]
fn drive_keeps_folder_creation_move_trash_and_office_filters() {
    let folder = request(
        "drive",
        "files.create",
        json!({}),
        Some(json!({"name":"Folder","mimeType":"inode/directory"})),
    )
    .unwrap();
    assert_eq!(folder.url.path(), "/v1.0/me/drive/root/children");
    assert_eq!(folder.body.unwrap(), json!({"name":"Folder","folder":{}}));
    let moved = request(
        "drive",
        "files.update",
        json!({"fileId":"file","addParents":"folder","removeParents":"previous"}),
        Some(json!({"name":"New name"})),
    )
    .unwrap();
    assert_eq!(
        moved.body.unwrap(),
        json!({"name":"New name","parentReference":{"id":"folder"}})
    );
    let trash = request(
        "drive",
        "files.update",
        json!({"fileId":"file"}),
        Some(json!({"trashed":true})),
    )
    .unwrap();
    assert_eq!(trash.method, Method::DELETE);
    let r=request("drive","files.list",json!({"q":"trashed = false and 'root' in parents and mimeType = 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'"}),None).unwrap();
    assert_eq!(
        r.filters[0].1,
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
}

#[test]
fn events_use_calendar_view_and_keep_all_day_dates() {
    let r=request("calendar","events.list",json!({"calendarId":"primary","timeMin":"2026-11-26T00:00:00Z","timeMax":"2026-11-27T00:00:00Z"}),None).unwrap();
    assert_eq!(r.url.path(), "/v1.0/me/calendar/calendarView");
    assert!(
        r.url
            .query_pairs()
            .any(|(k, v)| k == "startDateTime" && v == "2026-11-26T00:00:00Z")
    );
    let r = request(
        "calendar",
        "events.insert",
        json!({"calendarId":"primary"}),
        Some(
            json!({"summary":"Birthday","start":{"date":"2026-11-26"},"end":{"date":"2026-11-27"}}),
        ),
    )
    .unwrap();
    assert_eq!(r.body.as_ref().unwrap()["isAllDay"], true);
    assert_eq!(r.body.unwrap()["start"]["dateTime"], "2026-11-26T00:00:00");
    let event = response(
        Shape::Event,
        &json!({"start":{"dateTime":"2026-11-26T12:00:00","timeZone":"UTC"}}),
    );
    assert_eq!(event["start"]["dateTime"], "2026-11-26T12:00:00Z");
    assert_eq!(iana_zone("Romance Standard Time"), "Europe/Paris");
    assert_eq!(iana_zone("Europe/Paris"), "Europe/Paris");
}

#[test]
fn contacts_and_todo_translate_writes_and_responses() {
    let r=request("contacts","people.create-contact",json!({}),Some(json!({"names":[{"givenName":"Alex","familyName":"Doe"}],"emailAddresses":[{"value":"alex@example.com"}]}))).unwrap();
    assert_eq!(
        r.body.unwrap(),
        json!({"givenName":"Alex","surname":"Doe","emailAddresses":[{"address":"alex@example.com"}]})
    );
    let r = request(
        "tasks",
        "tasks.patch",
        json!({"tasklist":"list","task":"task"}),
        Some(json!({"status":"needsAction","completed":null})),
    )
    .unwrap();
    assert_eq!(
        r.body.unwrap(),
        json!({"status":"notStarted","completedDateTime":null})
    );
    let task = response(
        Shape::Task,
        &json!({"id":"task","title":"Test","status":"completed","body":{"content":"Note"}}),
    );
    assert_eq!(task["status"], "completed");
    assert_eq!(task["notes"], "Note");
}

#[test]
fn unsupported_operations_and_fields_fail_before_any_network_or_vault_access() {
    for service in ["forms", "youtube", "docs", "slides", "photos"] {
        assert!(crate::cli::parse_from(["joom", service, "user@example.com", "get"]).is_err());
    }
    assert!(
        request(
            "calendar",
            "events.insert",
            json!({}),
            Some(json!({"summary":"Test","recurrence":["RRULE:FREQ=DAILY"]}))
        )
        .is_err()
    );
    assert!(write_body(Shape::File, &json!({"name":"file","unmapped":"value"})).is_err());
    assert!(
        request(
            "calendar",
            "events.delete",
            json!({"calendarId":"primary","eventId":"event","sendUpdates":"none"}),
            None
        )
        .is_err()
    );
}

#[test]
fn field_projection_preserves_nested_arrays_and_pagination() {
    let selection = field_selection("nextPageToken,files(id,name,owners(emailAddress))").unwrap();
    let out = project(
        &json!({"nextPageToken":"next","files":[{"id":"one","name":"File","size":10,"owners":[{"emailAddress":"me@example.com","displayName":"Me"}]}],"extra":true}),
        &selection,
    );
    assert_eq!(
        out,
        json!({"nextPageToken":"next","files":[{"id":"one","name":"File","owners":[{"emailAddress":"me@example.com"}]}]})
    );
    for invalid in ["files(", "files(id))", ",id", "id,", "files(,id)"] {
        assert!(field_selection(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn excel_uses_microsoft_graph_workbook_and_range_apis() {
    let r = request("sheets", "get", json!({"spreadsheetId":"file"}), None).unwrap();
    assert_eq!(
        r.url.path(),
        "/v1.0/me/drive/items/file/workbook/worksheets"
    );
    let r = request(
        "sheets",
        "values.update",
        json!({"spreadsheetId":"file","range":"Sheet1!A1:B1"}),
        Some(json!({"values":[["Hello",42]]})),
    )
    .unwrap();
    assert_eq!(r.method, Method::PATCH);
    assert!(
        r.url
            .path()
            .ends_with("/workbook/worksheets/Sheet1/range(address='A1%3AB1')")
    );
    assert_eq!(r.body.unwrap(), json!({"values":[["Hello",42]]}));
    let r = request(
        "sheets",
        "values.clear",
        json!({"spreadsheetId":"file","range":"Sheet1!A1:B1"}),
        None,
    )
    .unwrap();
    assert_eq!(r.method, Method::POST);
    assert_eq!(r.body.unwrap(), json!({"applyTo":"Contents"}));
}

#[test]
fn excel_grid_writes_expand_single_cell_anchors_and_preserve_missing_cells() {
    let r = request(
        "sheets",
        "values.update",
        json!({"spreadsheetId":"file","range":"Fixture!$Z$9"}),
        Some(json!({"values":[["Name","Count"],["Joom"]]})),
    )
    .unwrap();
    assert!(r.url.path().ends_with("range(address='Z9%3AAA10')"));
    assert_eq!(
        r.body.unwrap(),
        json!({"values":[["Name","Count"],["Joom",null]]})
    );
    assert!(excel_write_address("XFD1048576", &json!({"values":[[1,2]]})).is_err());
    assert_eq!(
        excel_write_address("A1:B2", &json!({"values":[[1,2],[3,4]]})).unwrap(),
        "A1:B2"
    );
    let mut rows = vec![json!([]); 4096];
    rows[0] = json!(vec![1; 4096]);
    assert!(write_body(Shape::Range, &json!({"values":rows})).is_err());
}

#[test]
fn wrapper_contact_updates_keep_microsoft_concurrency_metadata() {
    let current = response(
        Shape::Contact,
        &json!({"id":"contact","displayName":"Alex","@odata.etag":"W/\"version\""}),
    );
    assert_eq!(current["metadata"]["sources"][0]["type"], "CONTACT");
    let r=request("contacts","people.update-contact",json!({"resourceName":"people/contact"}),Some(json!({"names":[{"unstructuredName":"New name"}],"metadata":current["metadata"],"etag":current["etag"]}))).unwrap();
    assert_eq!(r.body.unwrap(), json!({"displayName":"New name"}));
    assert_eq!(r.etag.as_deref(), Some("W/\"version\""));
}

#[test]
fn spreadsheet_csv_export_reads_the_selected_worksheets_used_range() {
    let r = request(
        "sheets",
        "values.get",
        json!({"spreadsheetId":"workbook","range":"'First tab'"}),
        None,
    )
    .unwrap();
    assert_eq!(
        r.url.path(),
        "/v1.0/me/drive/items/workbook/workbook/worksheets/First%20tab/usedRange(valuesOnly=true)"
    );
}
