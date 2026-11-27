use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output},
};

fn private_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
}

// A synthetic laptop vault. The test never reads the user's vault or contacts Microsoft.
fn laptop(home: &Path) {
    let root = home.join("joom");
    fs::create_dir_all(root.join("secrets")).unwrap();
    let key = [0x5a; 32];
    private_file(&root.join("master.key"), &key);
    private_file(
        &root.join("vault.json"),
        br#"{"version":1,"service":"joom/vault","key_storage":"file"}"#,
    );
    private_file(
        &root.join("config.json"),
        br#"{"account":"test@example.com","accounts":["test@example.com"]}"#,
    );
    for (name, value) in [
        (
            "oauth-client",
            json!({"client_id":"synthetic-client-id","client_secret":"synthetic-client-secret"}),
        ),
        (
            "account/test@example.com",
            json!({"access_token":"synthetic-access-token","refresh_token":"synthetic-refresh-token","expires_at":1}),
        ),
    ] {
        let mut nonce = [0; 12];
        getrandom::fill(&mut nonce).unwrap();
        let encrypted = Aes256Gcm::new_from_slice(&key)
            .unwrap()
            .encrypt(
                &Nonce::from(nonce),
                Payload {
                    msg: &serde_json::to_vec(&value).unwrap(),
                    aad: &[b"GOGVLT01".as_slice(), name.as_bytes()].concat(),
                },
            )
            .unwrap();
        private_file(
            &root
                .join("secrets")
                .join(format!("{:x}.enc", Sha256::digest(name.as_bytes()))),
            &[b"GOGVLT01".as_slice(), &nonce, &encrypted].concat(),
        );
    }
}

fn run(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_joom"))
        .args(["--home", home.to_str().unwrap(), "--json", "auth"])
        .args(args)
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent-joom-test-session",
        )
        .output()
        .unwrap()
}

#[test]
fn invalid_microsoft_client_ids_are_rejected_before_creating_a_vault() {
    let home = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "--json",
            "auth",
            "enroll",
            "--stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"client_id":"not-an-application-id.example.com","client_secret":"must-not-be-saved"}"#).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("Application (client) ID")
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("must-not-be-saved"));
    assert!(!home.path().join("joom").exists());
}

#[test]
fn live_session_only_refuses_expired_tokens_without_refreshing() {
    let home = tempfile::tempdir().unwrap();
    laptop(home.path());
    let output = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "--json",
            "mail",
            "test@example.com",
            "profile",
        ])
        .env("JOOM_LIVE_SESSION_ONLY", "1")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert!(
        error["error"]
            .as_str()
            .unwrap()
            .contains("no authentication or refresh attempted")
    );
    assert!(!error["error"].as_str().unwrap().contains("HTTP"));
}

#[test]
fn native_enrollment_saves_the_application_id_without_a_client_secret() {
    let home = tempfile::tempdir().unwrap();
    laptop(home.path());
    let mut child = Command::new(env!("CARGO_BIN_EXE_joom"))
        .args([
            "--home",
            home.path().to_str().unwrap(),
            "--json",
            "auth",
            "enroll",
            "--stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"client_id":"00001111-aaaa-2222-bbbb-3333cccc4444","client_secret":"legacy-secret-must-not-be-saved"}"#).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let label = b"oauth-client";
    let bytes = fs::read(
        home.path()
            .join("joom/secrets")
            .join(format!("{:x}.enc", Sha256::digest(label))),
    )
    .unwrap();
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&bytes[8..20]);
    let plain = Aes256Gcm::new_from_slice(&[0x5a; 32])
        .unwrap()
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: &bytes[20..],
                aad: &[b"GOGVLT01".as_slice(), label].concat(),
            },
        )
        .unwrap();
    let credentials: Value = serde_json::from_slice(&plain).unwrap();
    assert_eq!(
        credentials["client_id"],
        "00001111-aaaa-2222-bbbb-3333cccc4444"
    );
    assert_eq!(credentials["client_secret"], "");
}

#[test]
fn laptop_export_and_headless_server_import_work_through_the_executable() {
    let laptop_dir = tempfile::tempdir().unwrap();
    let server_dir = tempfile::tempdir().unwrap();
    laptop(laptop_dir.path());
    let mut passphrase = tempfile::NamedTempFile::new().unwrap();
    passphrase
        .write_all(b"synthetic transfer passphrase\n")
        .unwrap();
    let password = passphrase.path().to_str().unwrap();
    let export = laptop_dir.path().join("credentials.joom");
    let result = run(
        laptop_dir.path(),
        &[
            "test@example.com",
            "export",
            export.to_str().unwrap(),
            "--passphrase-file",
            password,
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["accounts"], json!(["test@example.com"]));
    assert!(!String::from_utf8_lossy(&result.stdout).contains("synthetic-"));

    let copied = server_dir.path().join("credentials.joom");
    fs::copy(&export, &copied).unwrap();
    let result = run(
        server_dir.path(),
        &[
            "import",
            copied.to_str().unwrap(),
            "--headless",
            "--passphrase-file",
            password,
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&result.stdout).unwrap()["accounts"],
        json!(["test@example.com"])
    );
    assert!(server_dir.path().join("joom/master.key").is_file());
    assert_ne!(
        fs::read(laptop_dir.path().join("joom/master.key")).unwrap(),
        fs::read(server_dir.path().join("joom/master.key")).unwrap()
    );

    let reexport = server_dir.path().join("server-copy.joom");
    let result = run(
        server_dir.path(),
        &[
            "export",
            reexport.to_str().unwrap(),
            "--passphrase-file",
            password,
        ],
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(reexport.is_file());
    let collision = run(
        server_dir.path(),
        &[
            "import",
            copied.to_str().unwrap(),
            "--headless",
            "--passphrase-file",
            password,
        ],
    );
    assert!(!collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("--replace"));
    let replacement = run(
        server_dir.path(),
        &[
            "import",
            copied.to_str().unwrap(),
            "--replace",
            "--passphrase-file",
            password,
        ],
    );
    assert!(
        replacement.status.success(),
        "{}",
        String::from_utf8_lossy(&replacement.stderr)
    );
}

#[test]
fn headless_init_needs_no_keychain_and_preserves_its_key_on_reinitialization() {
    let directory = tempfile::tempdir().unwrap();
    let first = run(directory.path(), &["init", "--headless"]);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&first.stdout).unwrap()["master_key"],
        "file"
    );
    let key = fs::read(directory.path().join("joom/master.key")).unwrap();
    let second = run(directory.path(), &["init"]);
    assert!(second.status.success());
    assert_eq!(
        fs::read(directory.path().join("joom/master.key")).unwrap(),
        key
    );
}
