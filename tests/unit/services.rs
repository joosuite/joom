use super::*;
#[test]
fn scopes_use_microsoft_delegated_permissions_and_deduplicate() {
    assert_eq!(scopes("mail,drive").unwrap(), scopes("mail,drive").unwrap());
    assert_eq!(
        scopes("drive,sheets,docs,slides").unwrap(),
        ["Files.ReadWrite"]
    );
    let all = scopes("all").unwrap();
    assert_eq!(all, scopes("all,mail,drive,all").unwrap());
    assert!(all.iter().all(|scope| !scope.contains("://")));
    assert!(scopes("gmail").is_err());
    assert!(all.iter().any(|s| s == "Mail.Send"));
    assert!(scopes("").is_err());
    assert!(scopes("unknown").is_err());
    assert!(scopes("forms").is_err());
    assert!(scopes("youtube").is_err());
}
