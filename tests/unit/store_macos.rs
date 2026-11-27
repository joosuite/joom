use super::*;

#[test]
fn lookup_accepts_current_and_legacy_joom_namespaces() {
    let legacy =
        crate::store::legacy_namespace(std::path::Path::new("/tmp/joom-keychain-namespace-test"))
            .unwrap();
    for service in [
        "joom/vault".to_owned(),
        format!("joom/vault/{}", "ab".repeat(16)),
        legacy,
    ] {
        assert!(valid_lookup(&service), "{service}");
    }
    for service in [
        "other-app/vault".to_owned(),
        format!("gogcli-rust/vault/{}", "ab".repeat(32)),
        "joom-rust/vault/short".to_owned(),
        format!("joom-rust/vault/{}\nquit", "ab".repeat(32)),
    ] {
        assert!(!valid_lookup(&service), "{service}");
    }
}

#[test]
fn binary_and_printable_legacy_keys_are_decoded_without_trimming() {
    let printable = b" abcdef0123456789abcdef01234567 ";
    assert_eq!(printable.len(), 32);
    let mut raw = printable.to_vec();
    raw.push(b'\n');
    assert_eq!(decode(&raw).unwrap().as_slice(), printable);
    let bytes = [0_u8, 10, 13, 255].repeat(8);
    let hex = format!("{}\n", crate::store::hex(&bytes));
    assert_eq!(decode(hex.as_bytes()).unwrap().as_slice(), bytes);
    for invalid in [b"".as_slice(), b"short\n", &[b'g'; 65]] {
        assert!(decode(invalid).is_err());
    }
}

#[test]
fn writes_use_private_input_and_trust_only_the_system_tool() {
    let input = save_input("joom/vault", &[7; 32]).unwrap();
    assert!(input.contains("-T /usr/bin/security"));
    assert!(!input.contains("-A"));
    assert!(!input.contains("-U"));
    assert!(input.ends_with(&format!("{}\n", "07".repeat(32))));
    for service in ["joom/vault\nquit", "joom/vault/../other", "-a", "other"] {
        assert!(save_input(service, &[0; 32]).is_err());
    }
    assert!(save_input("joom/vault", &[0; 31]).is_err());
}
