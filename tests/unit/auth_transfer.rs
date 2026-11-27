use super::*;
use crate::store::Config;

fn bundle() -> Bundle {
    Bundle {
        version: 1,
        client: Credentials {
            client_id: "private-client-id".into(),
            client_secret: "private-client-secret".into(),
        },
        accounts: vec![
            Account {
                email: "alice@example.com".into(),
                refresh_token: "private-alice-refresh".into(),
                personal: true,
            },
            Account {
                email: "other@example.com".into(),
                refresh_token: "private-other-refresh".into(),
                personal: false,
            },
        ],
    }
}

fn store(directory: &Path) -> Store {
    Store::new(Some(directory.to_str().unwrap())).unwrap()
}

fn password_file(directory: &Path, value: &[u8]) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new_in(directory).unwrap();
    file.write_all(value).unwrap();
    file
}

#[test]
fn encrypted_transfer_round_trip_is_portable_and_rejects_wrong_passwords_and_tampering() {
    let laptop = tempfile::tempdir().unwrap();
    let source = store(laptop.path());
    restore(&source, &bundle(), None, true, false).unwrap();
    let password = password_file(laptop.path(), b"a long private transfer passphrase\n");
    let file = laptop.path().join("credentials.joom");
    assert_eq!(
        export(
            &source,
            Some("ALICE@example.com"),
            &file,
            Some(password.path())
        )
        .unwrap(),
        ["alice@example.com"]
    );
    let bytes = fs::read(&file).unwrap();
    for value in [
        "private-client-id",
        "private-client-secret",
        "private-alice-refresh",
        "alice@example.com",
        "a long private transfer passphrase",
    ] {
        assert!(
            !bytes
                .windows(value.len())
                .any(|window| window == value.as_bytes())
        );
    }
    assert!(export(&source, None, &file, Some(password.path())).is_err());
    assert_eq!(fs::read(&file).unwrap(), bytes);

    let vps = tempfile::tempdir().unwrap();
    let destination = store(vps.path());
    let wrong = password_file(vps.path(), b"an incorrect transfer passphrase");
    let error = import(&destination, None, &file, Some(wrong.path()), true, false).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("cannot decrypt credential export")
    );
    assert!(!destination.root.exists());
    let mut damaged = bytes.clone();
    *damaged.last_mut().unwrap() ^= 1;
    fs::write(&file, damaged).unwrap();
    assert!(
        import(
            &destination,
            None,
            &file,
            Some(password.path()),
            true,
            false
        )
        .is_err()
    );
    assert!(!destination.root.exists());
    fs::write(&file, &bytes).unwrap();
    assert_eq!(
        import(
            &destination,
            None,
            &file,
            Some(password.path()),
            true,
            false
        )
        .unwrap(),
        ["alice@example.com"]
    );

    let reopened = store(vps.path());
    let restored: Tokens = reopened.secret("account/alice@example.com").unwrap();
    assert_eq!(restored.refresh_token, "private-alice-refresh");
    assert!(restored.access_token.is_empty());
    assert_eq!(restored.expires_at, 0);
    assert!(restored.personal);
    let client: Credentials = reopened.secret("oauth-client").unwrap();
    assert_eq!(client.client_secret, "private-client-secret");
    assert_ne!(
        fs::read(source.root.join("master.key")).unwrap(),
        fs::read(reopened.root.join("master.key")).unwrap()
    );
    assert_eq!(reopened.config().unwrap().accounts, ["alice@example.com"]);
    assert!(reopened.uses_key_file().unwrap());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&file).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn imports_preserve_other_accounts_and_refuse_client_changes_that_would_break_them() {
    let directory = tempfile::tempdir().unwrap();
    let destination = store(directory.path());
    restore(&destination, &bundle(), None, true, false).unwrap();
    let mut config = destination.config().unwrap();
    config.account = Some("other@example.com".into());
    destination.save_config(&config).unwrap();
    let mut incoming = bundle();
    incoming.accounts[0].refresh_token = "new-alice-refresh".into();
    assert!(
        restore(
            &destination,
            &incoming,
            Some("alice@example.com"),
            false,
            false
        )
        .unwrap_err()
        .to_string()
        .contains("--replace")
    );
    let old: Tokens = destination.secret("account/alice@example.com").unwrap();
    assert_eq!(old.refresh_token, "private-alice-refresh");
    restore(
        &destination,
        &incoming,
        Some("alice@example.com"),
        false,
        true,
    )
    .unwrap();
    let replaced: Tokens = destination.secret("account/alice@example.com").unwrap();
    assert_eq!(replaced.refresh_token, "new-alice-refresh");
    let other: Tokens = destination.secret("account/other@example.com").unwrap();
    assert_eq!(other.refresh_token, "private-other-refresh");
    assert_eq!(
        destination.config().unwrap().account.as_deref(),
        Some("other@example.com")
    );

    incoming.client.client_id = "different-client".into();
    assert!(
        restore(
            &destination,
            &incoming,
            Some("alice@example.com"),
            false,
            true
        )
        .unwrap_err()
        .to_string()
        .contains("other accounts")
    );
    let client: Credentials = destination.secret("oauth-client").unwrap();
    assert_eq!(client.client_id, "private-client-id");
    restore(&destination, &incoming, None, false, true).unwrap();
    let client: Credentials = destination.secret("oauth-client").unwrap();
    assert_eq!(client.client_id, "different-client");
    assert_eq!(destination.config().unwrap().accounts.len(), 2);
}

#[test]
fn invalid_or_unselected_accounts_do_not_initialize_a_destination() {
    let directory = tempfile::tempdir().unwrap();
    let destination = store(directory.path());
    assert!(
        restore(
            &destination,
            &bundle(),
            Some("missing@example.com"),
            true,
            false
        )
        .is_err()
    );
    let mut invalid = bundle();
    invalid.accounts[1].email = "ALICE@example.com".into();
    assert!(restore(&destination, &invalid, None, true, false).is_err());
    invalid.accounts[1].email = "other@example.com".into();
    invalid.accounts[0].refresh_token.clear();
    assert!(restore(&destination, &invalid, None, true, false).is_err());
    assert!(!destination.root.exists());
    let invalid_file = directory.path().join("invalid.joom");
    fs::write(&invalid_file, b"not a credential export").unwrap();
    assert!(
        import(&destination, None, &invalid_file, None, true, false)
            .unwrap_err()
            .to_string()
            .contains("invalid or unsupported")
    );
    assert!(!destination.root.exists());
}

#[test]
fn passphrase_files_accept_one_final_newline_and_refuse_public_files() {
    let directory = tempfile::tempdir().unwrap();
    let file = password_file(directory.path(), b"private passphrase\r\n");
    assert_eq!(
        passphrase(Some(file.path()), false).unwrap().as_str(),
        "private passphrase"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(file.path(), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            passphrase(Some(file.path()), false)
                .unwrap_err()
                .to_string()
                .contains("chmod 600")
        );
    }
}

#[test]
fn exporting_all_accounts_uses_the_enrolled_client_and_refresh_tokens_only() {
    let directory = tempfile::tempdir().unwrap();
    let source = store(directory.path());
    restore(&source, &bundle(), None, true, false).unwrap();
    source
        .save_config(&Config {
            account: Some("other@example.com".into()),
            accounts: vec!["other@example.com".into(), "alice@example.com".into()],
        })
        .unwrap();
    let collected = collect(&source, None).unwrap();
    assert_eq!(
        collected
            .accounts
            .iter()
            .map(|account| account.email.as_str())
            .collect::<Vec<_>>(),
        ["alice@example.com", "other@example.com"]
    );
    let raw = Zeroizing::new(serde_json::to_string(&collected).unwrap());
    assert!(!raw.contains("access_token"));
    assert!(!raw.contains("master"));
    assert_eq!(collected.client.client_id, "private-client-id");
}
