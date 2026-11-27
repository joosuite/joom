use super::*;
use serde_json::json;

#[test]
fn plain_output_has_no_json_fallback_for_nested_response_shapes() {
    let mut responses = vec![
        json!({}),
        json!([]),
        json!({"unrecognizedCollection":[{"name":"entry","nested":{"enabled":true,"tags":["one","two"]}}],"nextPageToken":"cursor"}),
        json!({"first":[{"name":"one"}],"second":[{"name":"two"}],"details":{"active":true}}),
        json!([{"name":"entry"},"text",42,true,null,["cell",{"nested":"value"}]]),
        json!({"spreadsheetId":"sheet","values":[["name",1,true,null],["second",2,false,"value"]]}),
    ];
    for _depth in 0..4 {
        for response in &responses {
            for paginated in [false, true] {
                let mut bytes = Vec::new();
                emit_to(response, false, paginated, &mut bytes).unwrap();
                let text = String::from_utf8(bytes).unwrap();
                assert!(
                    !text.contains(['{', '}', '[', ']', '"']),
                    "JSON leaked into plain output: {text}"
                );
                let mut bytes = Vec::new();
                emit_to(response, true, paginated, &mut bytes).unwrap();
                assert_eq!(&serde_json::from_slice::<Value>(&bytes).unwrap(), response);
            }
        }
        responses = responses
            .into_iter()
            .map(|value| json!({"section":[{"value":value}]}))
            .collect();
    }
}

#[test]
fn mail_folders_are_rows_in_plain_output_and_unchanged_in_json() {
    let value = json!({"labels":[
        {"id":"INBOX","name":"INBOX","type":"system"},
        {"id":"CHAT","name":"CHAT","type":"system","labelListVisibility":"labelHide","messageListVisibility":"hide"},
        {"id":"Label_1","name":"Projects","type":"user","color":{"backgroundColor":"#ffffff","textColor":"#000000"}}
    ]});
    let mut bytes = Vec::new();
    emit_to(&value, false, false, &mut bytes).unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "id\tname\ttype\tcolor\tlabelListVisibility\tmessageListVisibility\nINBOX\tINBOX\tsystem\t\t\t\nCHAT\tCHAT\tsystem\t\tlabelHide\thide\nLabel_1\tProjects\tuser\tbackgroundColor: #ffffff; textColor: #000000\t\t\n"
    );
    let mut bytes = Vec::new();
    emit_to(&value, true, false, &mut bytes).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
}

#[test]
fn collections_are_detected_without_service_specific_field_names() {
    let mut bytes = Vec::new();
    emit_to(
        &json!({"notes":[{"id":"n1","title":"Checklist"}],"nextPageToken":"cursor"}),
        false,
        false,
        &mut bytes,
    )
    .unwrap();
    assert_eq!(bytes, b"id\ttitle\nn1\tChecklist\n");
}

#[test]
fn nested_records_keep_their_fields_and_never_fall_back_to_json() {
    let value = json!({"id":"m1","labelIds":["INBOX","UNREAD"],"payload":{"headers":[{"name":"From","value":"sender@example.com"}],"mimeType":"text/plain"}});
    let mut bytes = Vec::new();
    emit_to(&value, false, false, &mut bytes).unwrap();
    assert_eq!(bytes, b"id\tm1\nlabelIds:\n  INBOX\n  UNREAD\npayload:\n  headers:\n    name\tvalue\n    From\tsender@example.com\n  mimeType\ttext/plain\n");
    assert_eq!(
        escaped(&json!({"items":[{"text":"unsafe\n\u{1b}[2J"}]})),
        "items: text: unsafe\\n\\u001b[2J"
    );
}

#[test]
fn status_uses_a_sentence_by_default_and_preserves_explicit_json() {
    let value = json!({"microsoft":"reachable","oauth_client":"not_rejected"});
    let mut bytes = Vec::new();
    status_to(
        "Microsoft connection successful.",
        &value,
        false,
        &mut bytes,
    )
    .unwrap();
    assert_eq!(bytes, b"Microsoft connection successful.\n");
    bytes.clear();
    status_to("Microsoft connection successful.", &value, true, &mut bytes).unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
    bytes.clear();
    status_to(
        "Signed out forged\u{1b}[2J@example.com.",
        &value,
        false,
        &mut bytes,
    )
    .unwrap();
    assert_eq!(bytes, b"Signed out forged\\u001b[2J@example.com.\n");
}

#[test]
fn default_output_is_tsv_including_paginated_responses() {
    let value = json!({"items":[{"name":"one"}],"nextPageToken":"next"});
    for paginated in [false, true] {
        let mut bytes = Vec::new();
        emit_to(&value, false, paginated, &mut bytes).unwrap();
        assert_eq!(bytes, b"name\none\n");
    }
}

#[test]
fn json_option_preserves_response_and_uses_one_line_per_page() {
    let value = json!({"items":[{"name":"one"}],"nextPageToken":"next"});
    for paginated in [false, true] {
        let mut bytes = Vec::new();
        emit_to(&value, true, paginated, &mut bytes).unwrap();
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
        if paginated {
            assert_eq!(bytes.iter().filter(|&&b| b == b'\n').count(), 1);
        }
    }
}

#[test]
fn table_escapes_terminal_sequences_and_multiline_fields() {
    let mut bytes = Vec::new();
    emit_to(
        &json!({"items":[{"name":"x\t\n\u{1b}[2J"}]}),
        false,
        false,
        &mut bytes,
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(bytes).unwrap(),
        "name\nx\\t\\n\\u001b[2J\n"
    );
}
