use super::*;

#[test]
fn read_only_post_cache_keys_include_the_body_without_exposing_its_contents() {
    let first = body_identity(
        "https://graph.microsoft.com/v1.0/me/drive/root/search(q='fixture')".into(),
        Some(&serde_json::json!({"albumId":"private-album-one"})),
    )
    .unwrap();
    let second = body_identity(
        "https://graph.microsoft.com/v1.0/me/drive/root/search(q='fixture')".into(),
        Some(&serde_json::json!({"albumId":"private-album-two"})),
    )
    .unwrap();
    assert_ne!(first, second);
    assert!(!first.contains("private-album"));
    assert_eq!(
        body_identity("https://graph.microsoft.com/v1.0/me".into(), None).unwrap(),
        "https://graph.microsoft.com/v1.0/me"
    );
}
use serde_json::json;
use std::cell::Cell;

const URL: &str = "https://graph.microsoft.com/v1.0/me/mailFolders";
fn cache(root: &Path, account: &str, ttl: u32) -> Responses {
    Responses::new(root, account, Zeroizing::new(vec![7; 32]), ttl).unwrap()
}

#[test]
fn zero_ttl_always_fetches_and_creates_no_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(root.path(), "me@example.com", 0);
    let calls = Cell::new(0);
    for _ in 0..2 {
        cache
            .fetch(URL, || {
                calls.set(calls.get() + 1);
                Ok(json!({"labels":[]}))
            })
            .unwrap();
    }
    assert_eq!(calls.get(), 2);
    assert!(!root.path().join("responses").exists());
}

#[test]
fn cache_reuses_only_fresh_entries_and_keeps_response_data_encrypted() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(root.path(), "private-account@example.com", 60);
    let value = json!({"labels":[{"name":"private-response"}]});
    assert_eq!(cache.fetch(URL, || Ok(value.clone())).unwrap(), value);
    assert_eq!(
        cache
            .fetch(URL, || panic!("fresh data should be cached"))
            .unwrap(),
        value
    );
    let file = cache.path(&cache.record(URL).unwrap());
    let bytes = fs::read(&file).unwrap();
    for secret in ["private-response", "private-account@example.com", URL] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
    let modified = std::time::SystemTime::now() - Duration::from_secs(3601);
    fs::OpenOptions::new()
        .write(true)
        .open(file)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    assert_eq!(
        cache.fetch(URL, || Ok(json!({"fresh":true}))).unwrap(),
        json!({"fresh":true})
    );
}

#[test]
fn account_url_and_generation_isolate_cached_responses() {
    let root = tempfile::tempdir().unwrap();
    let first = cache(root.path(), "first@example.com", 60);
    let second = cache(root.path(), "second@example.com", 60);
    first.fetch(URL, || Ok(json!({"account":"first"}))).unwrap();
    assert_eq!(
        second
            .fetch(URL, || Ok(json!({"account":"second"})))
            .unwrap(),
        json!({"account":"second"})
    );
    let other = format!("{URL}?fields=labels(id)");
    assert_eq!(
        first
            .fetch(&other, || Ok(json!({"different":true})))
            .unwrap(),
        json!({"different":true})
    );
    invalidate(root.path(), "first@example.com").unwrap();
    assert_eq!(
        first.fetch(URL, || Ok(json!({"updated":true}))).unwrap(),
        json!({"updated":true})
    );
    assert_eq!(
        second
            .fetch(URL, || panic!("other account must keep its own cache"))
            .unwrap(),
        json!({"account":"second"})
    );
}

#[test]
fn invalidation_during_a_request_prevents_reusing_its_old_result() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(root.path(), "me@example.com", 60);
    cache
        .fetch(URL, || {
            invalidate(root.path(), "me@example.com")?;
            Ok(json!({"stale":true}))
        })
        .unwrap();
    assert_eq!(
        cache.fetch(URL, || Ok(json!({"fresh":true}))).unwrap(),
        json!({"fresh":true})
    );
}

#[test]
fn corrupt_wrong_key_or_swapped_entries_refetch_instead_of_returning_data() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(root.path(), "me@example.com", 60);
    cache
        .fetch(URL, || Ok(json!({"private":"original"})))
        .unwrap();
    let file = cache.path(&cache.record(URL).unwrap());
    let mut ciphertext = fs::read(&file).unwrap();
    ciphertext[30] ^= 1;
    fs::write(&file, ciphertext).unwrap();
    assert_eq!(
        cache.fetch(URL, || Ok(json!({"refetched":true}))).unwrap(),
        json!({"refetched":true})
    );
    let wrong_key = Responses::new(
        root.path(),
        "me@example.com",
        Zeroizing::new(vec![8; 32]),
        60,
    )
    .unwrap();
    assert_eq!(
        wrong_key
            .fetch(URL, || Ok(json!({"wrong_key_refetched":true})))
            .unwrap(),
        json!({"wrong_key_refetched":true})
    );
    let other = format!("{URL}?fields=labels(id)");
    fs::copy(&file, cache.path(&cache.record(&other).unwrap())).unwrap();
    assert_eq!(
        cache
            .fetch(&other, || Ok(json!({"swapped_refetched":true})))
            .unwrap(),
        json!({"swapped_refetched":true})
    );
}

#[test]
fn failures_are_not_cached_and_cache_storage_is_bounded() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(root.path(), "me@example.com", 60);
    assert!(
        cache
            .fetch(URL, || anyhow::bail!("Microsoft request failed"))
            .is_err()
    );
    for index in 0..40 {
        cache
            .fetch(&format!("{URL}?page={index}"), || Ok(json!({"id":index})))
            .unwrap();
    }
    let entries = fs::read_dir(&cache.root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "enc")
        })
        .count();
    assert_eq!(entries, ENTRIES);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in fs::read_dir(&cache.root).unwrap() {
            assert_eq!(
                entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            fs::metadata(&cache.root).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
