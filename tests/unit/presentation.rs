use super::*;

#[test]
fn native_lists_show_actionable_ids_and_details_without_transport_metadata() {
    let cases = [
        (
            "mail",
            "labels",
            json!({"labels":[{"id":"INBOX","name":"INBOX","type":"system","labelListVisibility":"labelShow"}]}),
            "INBOX",
        ),
        (
            "calendar",
            "events",
            json!({"kind":"calendar#events","items":[{"id":"event","summary":"Meeting","start":{"dateTime":"2026-12-01T10:00:00Z"},"end":{"dateTime":"2026-12-01T11:00:00Z"}}]}),
            "event",
        ),
        (
            "tasks",
            "list",
            json!({"etag":"transport","items":[{"id":"task","title":"Review","due":"2026-12-01T00:00:00Z","status":"needsAction"}]}),
            "task",
        ),
        (
            "contacts",
            "list",
            json!({"connections":[{"resourceName":"people/contact","etag":"transport","names":[{"displayName":"Alice"}],"emailAddresses":[{"value":"alice@example.com"}]}]}),
            "people/contact",
        ),
    ];
    for (service, action, response, id) in cases {
        let view = plain_value(service, action, &response).unwrap();
        assert_eq!(view[0]["id"], id, "{service} {action}");
        let text = serde_json::to_string(&view).unwrap();
        assert!(!text.contains("transport"), "{service} {action}: {text}");
    }
}
#[test]
fn reads_decode_content_and_keep_json_fields_untouched() {
    let mail = json!({"id":"message","snippet":"preview","payload":{"mimeType":"multipart/mixed","headers":[{"name":"Subject","value":"Hello"}],"parts":[{"mimeType":"text/plain","body":{"data":URL_SAFE_NO_PAD.encode("Hello\nworld")}}, {"mimeType":"text/plain","filename":"secret.txt","body":{"data":URL_SAFE_NO_PAD.encode("attachment")}}]}});
    let view = plain_value("mail", "get", &mail).unwrap();
    assert_eq!(view["subject"], "Hello");
    assert_eq!(view["text"], "Hello\nworld");
    assert!(mail["payload"].is_object());
    let cells = json!({"values":[["Name",1],["Second",2]],"range":"A1:B2"});
    assert_eq!(
        plain_value("sheets", "get", &cells).unwrap(),
        cells["values"]
    );
}
#[test]
fn cleared_sheet_ranges_return_an_empty_row_list() {
    let action = method_action("sheets", "values.get");
    for response in [
        json!({"range":"Fixture!A1:D3","majorDimension":"ROWS"}),
        json!({"values":[]}),
        json!({"values":[["text"]]}),
    ] {
        assert_eq!(
            plain_value("sheets", action, &response).unwrap(),
            response.get("values").cloned().unwrap_or_else(|| json!([]))
        );
    }
    assert_eq!(
        plain_value(
            "sheets",
            method_action("sheets", "get"),
            &json!({"spreadsheetId":"sheet","properties":{"title":"Title"}})
        )
        .unwrap()["title"],
        "Title"
    );
}
#[test]
fn every_write_command_has_a_human_confirmation() {
    for service in crate::services::SERVICES {
        for cmd in crate::native::commands(service.cli) {
            if !cmd
                .get_arguments()
                .any(|arg| arg.get_id() == "ttl" || arg.get_id() == "output")
            {
                assert!(
                    success(service.cli, cmd.get_name()).is_some(),
                    "{} {}",
                    service.cli,
                    cmd.get_name()
                );
            }
        }
    }
}
