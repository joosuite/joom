use super::*;

#[derive(Default)]
struct MemoryKeys {
    keys: std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>,
    saves: std::sync::atomic::AtomicUsize,
    locked: std::sync::atomic::AtomicBool,
    loads: std::sync::atomic::AtomicUsize,
    legacy_needs_approval: std::sync::atomic::AtomicBool,
}

impl Keychain for MemoryKeys {
    fn load(&self, namespace: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        self.loads.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        anyhow::ensure!(
            !self.locked.load(std::sync::atomic::Ordering::SeqCst),
            "keychain locked"
        );
        anyhow::ensure!(
            !namespace.starts_with("joom-rust/")
                || !self
                    .legacy_needs_approval
                    .load(std::sync::atomic::Ordering::SeqCst),
            "legacy approval required"
        );
        Ok(self
            .keys
            .lock()
            .unwrap()
            .get(namespace)
            .cloned()
            .map(Zeroizing::new))
    }
    fn save(&self, namespace: &str, key: &[u8]) -> Result<()> {
        self.keys
            .lock()
            .unwrap()
            .insert(namespace.to_owned(), key.to_vec());
        self.saves.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

fn vault() -> (tempfile::TempDir, Store, Arc<MemoryKeys>) {
    let dir = tempfile::tempdir().unwrap();
    let keys = Arc::new(MemoryKeys::default());
    let store = Store {
        root: dir.path().join("vault"),
        keychain: keys.clone(),
        legacy_root: None,
        default_vault: false,
        key: Mutex::new(None),
    };
    (dir, store, keys)
}

#[test]
fn headless_vaults_work_across_processes_without_contacting_a_keychain() {
    let (_directory, store, keys) = vault();
    keys.locked.store(true, std::sync::atomic::Ordering::SeqCst);
    store.initialize_headless().unwrap();
    store
        .set_secret("test", &serde_json::json!({"secret":"private-value"}))
        .unwrap();
    let reopened = Store {
        root: store.root.clone(),
        keychain: keys.clone(),
        legacy_root: None,
        default_vault: false,
        key: Mutex::new(None),
    };
    reopened.initialize().unwrap();
    assert_eq!(
        reopened.secret::<serde_json::Value>("test").unwrap()["secret"],
        "private-value"
    );
    assert_eq!(keys.loads.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 0);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&store.root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(store.root.join("master.key"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn headless_initialization_never_converts_an_existing_keychain_vault() {
    let (_directory, store, _keys) = vault();
    store.initialize().unwrap();
    let identity = fs::read(store.root.join("vault.json")).unwrap();
    assert!(store.initialize_headless().is_err());
    assert_eq!(fs::read(store.root.join("vault.json")).unwrap(), identity);
    assert!(!store.root.join("master.key").exists());
}

#[cfg(unix)]
#[test]
fn headless_keys_refuse_public_permissions_symlinks_and_regeneration_for_existing_records() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (directory, store, keys) = vault();
    store.initialize_headless().unwrap();
    store.set_secret("test", &"private-value").unwrap();
    let key_path = store.root.join("master.key");
    let key = Zeroizing::new(fs::read(&key_path).unwrap());
    let encrypted = fs::read(store.record("test")).unwrap();
    let reopened = || Store {
        root: store.root.clone(),
        keychain: keys.clone(),
        legacy_root: None,
        default_vault: false,
        key: Mutex::new(None),
    };
    fs::set_permissions(&key_path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(
        reopened()
            .secret::<String>("test")
            .unwrap_err()
            .to_string()
            .contains("chmod 600")
    );
    fs::remove_file(&key_path).unwrap();
    let outside = directory.path().join("outside-key");
    fs::write(&outside, &key).unwrap();
    symlink(&outside, &key_path).unwrap();
    assert!(
        reopened()
            .secret::<String>("test")
            .unwrap_err()
            .to_string()
            .contains("symbolic links")
    );
    fs::remove_file(&key_path).unwrap();
    reopened().initialize_headless().unwrap();
    assert!(!key_path.exists());
    assert!(reopened().secret::<String>("test").is_err());
    assert_eq!(fs::read(store.record("test")).unwrap(), encrypted);
    assert_eq!(keys.loads.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn import_into_a_keychain_vault_reencrypts_records_with_its_own_key() {
    let (_directory, store, keys) = vault();
    let client =
        Zeroizing::new(br#"{"client_id":"private-id","client_secret":"private-secret"}"#.to_vec());
    let token = Zeroizing::new(br#"{"refresh_token":"private-refresh"}"#.to_vec());
    store
        .import_credentials(
            &[
                ("oauth-client".into(), client),
                ("account/me@example.com".into(), token),
            ],
            &["me@example.com".into()],
            false,
            false,
        )
        .unwrap();
    assert!(!store.uses_key_file().unwrap());
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(
        store.secret::<serde_json::Value>("oauth-client").unwrap()["client_secret"],
        "private-secret"
    );
    assert_eq!(store.config().unwrap().accounts, ["me@example.com"]);
}

#[test]
fn credentials_and_tokens_are_encrypted_with_one_master_key() {
    let (_dir, store, keys) = vault();
    store.initialize().unwrap();
    store.initialize().unwrap();
    for (name, value) in [
        (
            "oauth-client",
            serde_json::json!({"client_id":"private-client-id", "client_secret":"private-client-secret"}),
        ),
        (
            "account/me@example.com",
            serde_json::json!({"access_token":"private-access-token", "refresh_token":"private-refresh-token"}),
        ),
    ] {
        store.set_secret(name, &value).unwrap();
        let first = fs::read(store.record(name)).unwrap();
        for secret in [
            "private-client-id",
            "private-client-secret",
            "private-access-token",
            "private-refresh-token",
        ] {
            assert!(!first.windows(secret.len()).any(|w| w == secret.as_bytes()));
        }
        assert_eq!(store.secret::<serde_json::Value>(name).unwrap(), value);
        store.set_secret(name, &value).unwrap();
        assert_ne!(first, fs::read(store.record(name)).unwrap());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(store.record(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(keys.keys.lock().unwrap().len(), 1);
    assert_eq!(keys.keys.lock().unwrap().values().next().unwrap().len(), 32);
    store.delete_secret("oauth-client").unwrap();
    assert!(!store.record("oauth-client").exists());
    assert!(
        store
            .secret::<serde_json::Value>("account/me@example.com")
            .is_ok()
    );
}

#[test]
fn rejects_tampering_truncation_wrong_key_and_record_swapping() {
    let (_dir, store, keys) = vault();
    store.initialize().unwrap();
    store.set_secret("first", &"secret-value").unwrap();
    let bytes = fs::read(store.record("first")).unwrap();
    for offset in [0, 8, 20, bytes.len() - 1] {
        let mut altered = bytes.clone();
        altered[offset] ^= 1;
        fs::write(store.record("first"), altered).unwrap();
        assert!(store.secret::<String>("first").is_err());
    }
    fs::write(store.record("first"), &bytes[..30]).unwrap();
    assert!(store.secret::<String>("first").is_err());
    fs::write(store.record("first"), &bytes).unwrap();
    fs::write(store.record("second"), &bytes).unwrap();
    assert!(store.secret::<String>("second").is_err());
    keys.keys
        .lock()
        .unwrap()
        .insert(store.namespace().unwrap(), vec![5; 32]);
    *store.key.lock().unwrap() = None;
    assert!(store.secret::<String>("first").is_err());
}

#[test]
fn missing_or_locked_keys_never_regenerate_or_overwrite_secrets() {
    let (_dir, store, keys) = vault();
    assert!(store.set_secret("client", &"secret-value").is_err());
    store.initialize().unwrap();
    store.set_secret("client", &"secret-value").unwrap();
    let bytes = fs::read(store.record("client")).unwrap();
    *store.key.lock().unwrap() = None;
    keys.locked.store(true, std::sync::atomic::Ordering::SeqCst);
    let loads = keys.loads.load(std::sync::atomic::Ordering::SeqCst);
    store.initialize().unwrap();
    assert_eq!(keys.loads.load(std::sync::atomic::Ordering::SeqCst), loads);
    assert!(store.secret::<String>("client").is_err());
    assert!(store.set_secret("client", &"replacement").is_err());
    keys.locked
        .store(false, std::sync::atomic::Ordering::SeqCst);
    keys.keys.lock().unwrap().clear();
    store.initialize().unwrap();
    assert!(store.secret::<String>("client").is_err());
    assert_eq!(fs::read(store.record("client")).unwrap(), bytes);
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn concurrent_initialization_creates_exactly_one_key() {
    let (_dir, store, keys) = vault();
    let store = Arc::new(store);
    let start = Arc::new(std::sync::Barrier::new(8));
    let threads = (0..8)
        .map(|_| {
            let store = store.clone();
            let start = start.clone();
            std::thread::spawn(move || {
                start.wait();
                store.initialize().unwrap();
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn one_keychain_read_serves_the_whole_credential_session() {
    let (_dir, store, keys) = vault();
    store.initialize().unwrap();
    store.set_secret("client", &"retained client").unwrap();
    *store.key.lock().unwrap() = None;
    keys.loads.store(0, std::sync::atomic::Ordering::SeqCst);
    for _ in 0..4 {
        assert_eq!(store.secret::<String>("client").unwrap(), "retained client");
        store.set_secret("token", &"updated token").unwrap();
        store.master_key().unwrap();
        store.initialize().unwrap();
    }
    assert_eq!(keys.loads.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn legacy_access_is_read_only_and_init_never_requests_keychain_access() {
    let dir = tempfile::tempdir().unwrap();
    let legacy = dir.path().join("joom-rust");
    private_dir(&legacy.join("secrets")).unwrap();
    let key = [29; 32];
    let encrypted = encrypt(
        &key,
        "oauth-client",
        br#"{"client_id":"existing","client_secret":"retained"}"#,
    )
    .unwrap();
    let filename = format!("{:x}.enc", Sha256::digest(b"oauth-client"));
    atomic(&legacy.join("secrets").join(&filename), &encrypted).unwrap();
    atomic(
        &legacy.join("config.json"),
        br#"{"accounts":["existing@example.com"]}"#,
    )
    .unwrap();
    let keys = Arc::new(MemoryKeys::default());
    let old_service = legacy_namespace(&legacy).unwrap();
    keys.keys
        .lock()
        .unwrap()
        .insert(old_service.clone(), key.to_vec());
    keys.legacy_needs_approval
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let mut store = Store::new(dir.path().to_str()).unwrap();
    store.keychain = keys.clone();
    assert!(legacy.exists());
    assert_eq!(store.root, legacy);
    assert_eq!(store.config().unwrap().accounts, ["existing@example.com"]);
    assert!(store.secret::<serde_json::Value>("oauth-client").is_err());
    let loads = keys.loads.load(std::sync::atomic::Ordering::SeqCst);
    store.initialize().unwrap();
    assert_eq!(keys.loads.load(std::sync::atomic::Ordering::SeqCst), loads);
    assert!(store.secret::<serde_json::Value>("oauth-client").is_err());
    keys.legacy_needs_approval
        .store(false, std::sync::atomic::Ordering::SeqCst);
    let credential: serde_json::Value = store.secret("oauth-client").unwrap();
    assert_eq!(credential["client_secret"], "retained");
    assert_eq!(fs::read(store.record("oauth-client")).unwrap(), encrypted);
    let service = store.namespace().unwrap();
    assert_eq!(service, old_service);
    assert_eq!(keys.keys.lock().unwrap()[&service], key);
    assert_eq!(keys.keys.lock().unwrap()[&old_service], key);
    assert!(!store.root.join("vault.json").exists());
    assert!(!dir.path().join("joom").exists());
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(keys.keys.lock().unwrap().len(), 1);
}

#[test]
fn conflicting_directories_and_malformed_identity_never_replace_credentials() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["joom-rust", "joom"] {
        private_dir(&dir.path().join(name)).unwrap();
        atomic(&dir.path().join(name).join("config.json"), name.as_bytes()).unwrap();
    }
    assert!(Store::new(dir.path().to_str()).is_err());
    assert_eq!(
        fs::read(dir.path().join("joom-rust/config.json")).unwrap(),
        b"joom-rust"
    );
    assert_eq!(
        fs::read(dir.path().join("joom/config.json")).unwrap(),
        b"joom"
    );
    let (_dir, store, keys) = vault();
    private_dir(&store.root).unwrap();
    for service in ["joom/vault\nquit", "other", "joom/vault/../other"] {
        atomic(
            &store.root.join("vault.json"),
            &serde_json::to_vec(&serde_json::json!({"version":1,"service":service})).unwrap(),
        )
        .unwrap();
        assert!(store.initialize().is_err());
    }
    assert_eq!(keys.saves.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
#[ignore = "requires an unlocked OS keychain"]
fn real_os_keychain_encrypts_and_decrypts() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::new(dir.path().to_str()).unwrap();
    store.initialize().unwrap();
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("/usr/bin/security")
                .args(["delete-generic-password", "-s", &self.0, "-a", "master-key"])
                .output();
            #[cfg(not(target_os = "macos"))]
            let _ = OsKeychain::entry(&self.0)
                .and_then(|entry| entry.delete_credential().map_err(Into::into));
        }
    }
    let _cleanup = Cleanup(store.namespace().unwrap());
    store
        .set_secret("test-client", &"test-secret-value")
        .unwrap();
    assert_eq!(
        store.secret::<String>("test-client").unwrap(),
        "test-secret-value"
    );
    let namespace = store.namespace().unwrap();
    let encrypted = fs::read(store.record("test-client")).unwrap();
    drop(store);
    for _ in 0..2 {
        let reopened = Store::new(dir.path().to_str()).unwrap();
        assert_eq!(reopened.namespace().unwrap(), namespace);
        reopened.initialize().unwrap();
        assert_eq!(
            reopened.secret::<String>("test-client").unwrap(),
            "test-secret-value"
        );
        assert_eq!(fs::read(reopened.record("test-client")).unwrap(), encrypted);
    }
    #[cfg(not(target_os = "macos"))]
    assert_native_legacy_access_is_read_only();
}

#[cfg(not(target_os = "macos"))]
fn assert_native_legacy_access_is_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let legacy = directory.path().join("joom-rust");
    private_dir(&legacy.join("secrets")).unwrap();
    let old_service = legacy_namespace(&legacy).unwrap();
    let key = [47; 32];
    OsKeychain.save(&old_service, &key).unwrap();
    struct Cleanup(Vec<String>);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            for service in &self.0 {
                if let Ok(entry) = OsKeychain::entry(service) {
                    let _ = entry.delete_credential();
                }
            }
        }
    }
    let mut cleanup = Cleanup(vec![old_service.clone()]);
    let filename = format!("{:x}.enc", Sha256::digest(b"test-client"));
    let encrypted = encrypt(&key, "test-client", br#""existing test credential""#).unwrap();
    atomic(&legacy.join("secrets").join(&filename), &encrypted).unwrap();
    let store = Store::new(directory.path().to_str()).unwrap();
    let service = store.namespace().unwrap();
    assert_eq!(service, old_service);
    assert_eq!(store.root, legacy);
    store.initialize().unwrap();
    assert_eq!(
        store.secret::<String>("test-client").unwrap(),
        "existing test credential"
    );
    assert_eq!(
        OsKeychain.load(&old_service).unwrap().unwrap().as_slice(),
        key
    );
    assert_eq!(OsKeychain.load(&service).unwrap().unwrap().as_slice(), key);
    assert_eq!(fs::read(store.record("test-client")).unwrap(), encrypted);
    assert!(!directory.path().join("joom").exists());
    assert!(!legacy.join("vault.json").exists());
    for service in &cleanup.0 {
        OsKeychain::entry(service)
            .unwrap()
            .delete_credential()
            .unwrap();
        assert!(OsKeychain.load(service).unwrap().is_none());
    }
    cleanup.0.clear();
}

#[test]
fn config_root_is_isolated_and_private() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::new(tmp.path().to_str()).unwrap();
    assert_eq!(store.root, tmp.path().join("joom"));
    assert!(store.config().unwrap().accounts.is_empty());
    store
        .save_config(&Config {
            account: Some("me@example.com".into()),
            accounts: vec!["me@example.com".into()],
        })
        .unwrap();
    assert_eq!(
        store.config().unwrap().account.as_deref(),
        Some("me@example.com")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(store.root.join("config.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
