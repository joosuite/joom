use serde_json::{Value, json};
use std::process::Command;

#[test]
fn malformed_parameter_files_fail_locally_for_every_service() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("parameters.json");
    let fixtures: Value = serde_json::from_str(include_str!("data/cli-inputs.json")).unwrap();
    let mut checked = 0;
    for fixture in fixtures.as_array().unwrap() {
        let service = fixture["service"].as_str().unwrap();
        let action = fixture["command"].as_str().unwrap();
        let mut unknown = fixture["file"].clone();
        unknown["__proto__"] = json!({"injected":true});
        for invalid in [Value::Null, json!([]), json!("not an object"), unknown] {
            std::fs::write(&path, serde_json::to_vec(&invalid).unwrap()).unwrap();
            let output = Command::new(env!("CARGO_BIN_EXE_joom"))
                .arg("--home")
                .arg(directory.path())
                .args([
                    "--json",
                    "--timeout",
                    "1",
                    service,
                    "offline@example.com",
                    action,
                    "--file",
                ])
                .arg(&path)
                .output()
                .unwrap();
            assert!(
                !output.status.success(),
                "{service} accepted malformed input"
            );
            assert_ne!(output.status.code(), Some(101), "{service} panicked");
            assert!(
                output.stdout.is_empty(),
                "{service} wrote output before rejecting input"
            );
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            let message = error["error"].as_str().unwrap();
            assert!(!message.contains("HTTP "), "{service} reached the network");
            assert!(
                !message.contains("master key"),
                "{service} reached the keychain"
            );
            assert!(
                !message.contains("sign in"),
                "{service} requested credentials before validation"
            );
            assert!(!message.contains("panicked"));
            checked += 1;
        }
    }
    assert_eq!(checked, 24);
}
