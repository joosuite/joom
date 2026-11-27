use super::*;

#[test]
fn installs_all_schema_defined_wrappers_and_can_run_again() {
    let directory = tempfile::tempdir().unwrap();
    let binary = directory
        .path()
        .join(format!("joom{}", std::env::consts::EXE_SUFFIX));
    std::fs::write(binary, b"native executable").unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("joom", directory.path().join("jmail")).unwrap();
        std::os::unix::fs::symlink("other-app", directory.path().join("jdrive")).unwrap();
        std::os::unix::fs::symlink("joom", directory.path().join("jmforms")).unwrap();
        std::os::unix::fs::symlink("other-program", directory.path().join("jmyt")).unwrap();
    }
    let schema: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/wrappers.schema.json"
    )))
    .unwrap();
    let wrappers = schema["x-wrappers"].as_object().unwrap();
    assert_eq!(install(directory.path()).unwrap(), wrappers.len());
    assert_eq!(install(directory.path()).unwrap(), wrappers.len());
    #[cfg(unix)]
    {
        assert!(std::fs::symlink_metadata(directory.path().join("jmail")).is_err());
        assert!(std::fs::symlink_metadata(directory.path().join("jmforms")).is_err());
        assert_eq!(
            std::fs::read_link(directory.path().join("jmyt")).unwrap(),
            Path::new("other-program")
        );
        assert_eq!(
            std::fs::read_link(directory.path().join("jdrive")).unwrap(),
            Path::new("other-app")
        );
    }
    for name in wrappers.keys() {
        let path = directory
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        assert_eq!(std::fs::read(&path).unwrap(), b"native executable");
        #[cfg(unix)]
        assert_eq!(std::fs::read_link(path).unwrap(), Path::new("joom"));
    }
}

#[test]
fn requires_joom_before_creating_wrappers() {
    let directory = tempfile::tempdir().unwrap();
    assert!(install(directory.path()).is_err());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn preserves_unrelated_files_and_symbolic_links() {
    for symlink in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("joom"), b"native executable").unwrap();
        let target = directory.path().join("jmmail");
        if symlink {
            std::os::unix::fs::symlink("other-program", &target).unwrap();
        } else {
            std::fs::write(&target, b"existing program").unwrap();
        }
        assert!(install(directory.path()).is_err());
        if symlink {
            assert_eq!(
                std::fs::read_link(target).unwrap(),
                Path::new("other-program")
            );
        } else {
            assert_eq!(std::fs::read(target).unwrap(), b"existing program");
        }
    }
}
