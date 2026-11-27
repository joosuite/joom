use super::*;
use serde_json::json;

pub fn document() -> Document {
    serde_json::from_value(json!({"name":"test", "version":"v1.0", "rootUrl":"https://graph.microsoft.com/", "servicePath":"v1.0/", "parameters":{"fields":{"type":"string","location":"query"}}, "resources":{"things":{"methods":{"get":{"id":"test.things.get","httpMethod":"GET","path":"{+name}","parameters":{"name":{"location":"path","type":"string","required":true},"pageSize":{"location":"query","type":"integer","minimum":"1","maximum":"100"},"tag":{"location":"query","type":"string","repeated":true}}}}}}})).unwrap()
}

#[test]
fn resource_paths_take_precedence_when_they_share_the_api_name() {
    let mut doc = document();
    doc.name = "things".into();
    doc.root
        .resources
        .get_mut("things")
        .unwrap()
        .methods
        .get_mut("get")
        .unwrap()
        .id = "things.things.get".into();
    assert_eq!(doc.method("things.get").unwrap().id, "things.things.get");
    assert_eq!(
        doc.method("things.things.get").unwrap().id,
        "things.things.get"
    );
}

#[test]
fn builds_nested_methods_and_encodes_paths_queries() {
    let doc = document();
    let m = doc.method("test.things.get").unwrap();
    let params = json!({"name":"things/A B?#", "tag":["a&b","c"], "pageSize":5});
    let url = doc.url(m, params.as_object().unwrap()).unwrap();
    assert_eq!(url.path(), "/v1.0/things/A%20B%3F%23");
    assert_eq!(url.query_pairs().filter(|(k, _)| k == "tag").count(), 2);
    assert!(doc.method("things.nope").is_err());
}

#[test]
fn rejects_bad_parameters_and_path_traversal() {
    let doc = document();
    let m = doc.method("things.get").unwrap();
    for p in [
        json!({}),
        json!({"name":"../other"}),
        json!({"name":"things/../other"}),
        json!({"name":"x","pageSize":101}),
        json!({"name":"x","pageSize":1.5}),
        json!({"name":"x","unknown":1}),
        json!({"name":null}),
        json!({"name":"x","tag":[{}]}),
    ] {
        assert!(doc.url(m, p.as_object().unwrap()).is_err(), "{p}");
    }
}

#[test]
fn per_method_hosts_preserve_path_validation_and_query_encoding() {
    let mut doc = document();
    let method = doc
        .root
        .resources
        .get_mut("things")
        .unwrap()
        .methods
        .get_mut("get")
        .unwrap();
    method.base_url = Some("https://graph.microsoft.com/v1.0/".into());
    method.anonymous = true;
    let method = doc.method("things.get").unwrap();
    let parameters = json!({"name":"things/A B","tag":["a&b"]});
    let url = doc.url(method, parameters.as_object().unwrap()).unwrap();
    assert_eq!(url.host_str(), Some("graph.microsoft.com"));
    assert_eq!(url.path(), "/v1.0/things/A%20B");
    assert_eq!(
        url.query_pairs().find(|(key, _)| key == "tag").unwrap().1,
        "a&b"
    );
    let mut invalid = method.clone();
    invalid.base_url = Some("https://example.invalid/".into());
    assert!(doc.url(&invalid, parameters.as_object().unwrap()).is_err());
}
