use serde_json::Value;
use std::path::Path;

#[test]
fn bundled_contracts_use_graph_origins_permissions_and_service_identifiers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for service in ["mail", "drive", "calendar", "sheets", "tasks", "contacts"] {
        let source = std::fs::read_to_string(root.join(format!("api/{service}.json"))).unwrap();
        let api: Value = serde_json::from_str(&source).unwrap();
        assert_eq!(api["x-api-service"], service);
        assert_eq!(api["x-api-version"], "v1.0");
        assert_eq!(api["x-api-root"], "https://graph.microsoft.com/");
        assert_eq!(api["x-api-service-path"], "v1.0/");
        for (command, definition) in api["$defs"].as_object().unwrap() {
            let identifier = format!("{service}.{command}");
            assert_eq!(definition["x-api-method-id"], identifier);
            let method = &api["x-api-methods"][&identifier];
            assert_eq!(method["id"], identifier);
            for permission in method["scopes"].as_array().into_iter().flatten() {
                assert!(!permission.as_str().unwrap().contains("://"));
            }
        }
    }
}
