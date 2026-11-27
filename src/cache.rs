use crate::{http, store};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use zeroize::Zeroizing;

const CAPACITY: u64 = 64 * 1024 * 1024;
const ENTRIES: usize = 32;

pub fn request_identity(
    doc: &crate::discovery::Document,
    method: &crate::discovery::MethodSpec,
    params: &serde_json::Map<String, Value>,
    relative_times: &[&str],
) -> Result<String> {
    let mut params = params.clone();
    for key in relative_times {
        params.remove(*key);
    }
    let mut identity = doc.url(method, &params)?;
    if !relative_times.is_empty() {
        // This fragment distinguishes a relative window from an unbounded request.
        // The identity is only hashed for caching; it is never sent to Microsoft.
        identity.set_fragment(Some(&format!(
            "joom-relative-time={}",
            relative_times.join(",")
        )));
    }
    Ok(identity.into())
}

pub fn body_identity(mut identity: String, body: Option<&Value>) -> Result<String> {
    if let Some(body) = body {
        let digest = Sha256::digest(serde_json::to_vec(body)?);
        identity.push_str(if identity.contains('#') { "&" } else { "#" });
        use std::fmt::Write;
        write!(&mut identity, "joom-body={digest:x}")?;
    }
    Ok(identity)
}

pub struct Responses {
    root: PathBuf,
    account: String,
    key: Zeroizing<Vec<u8>>,
    ttl: Duration,
}

fn directory(root: &Path, account: &str) -> PathBuf {
    root.join("responses").join(format!(
        "{:x}",
        Sha256::digest(account.to_ascii_lowercase().as_bytes())
    ))
}

pub fn invalidate(root: &Path, account: &str) -> Result<()> {
    let directory = directory(root, account);
    if directory.exists() {
        let mut generation = [0_u8; 32];
        getrandom::fill(&mut generation).context("operating system randomness unavailable")?;
        store::atomic(&directory.join("generation"), &generation)?;
    }
    Ok(())
}

impl Responses {
    pub fn new(root: &Path, account: &str, key: Zeroizing<Vec<u8>>, minutes: u32) -> Result<Self> {
        let cache = Self {
            root: directory(root, account),
            account: account.to_ascii_lowercase(),
            key,
            ttl: Duration::from_secs(u64::from(minutes) * 60),
        };
        if minutes > 0 {
            store::private_dir(cache.root.parent().context("cache has no parent")?)?;
            store::private_dir(&cache.root)?;
        }
        Ok(cache)
    }

    fn record(&self, url: &str) -> Result<String> {
        let generation = match fs::File::open(self.root.join("generation")) {
            Ok(file) => http::limited(file, 32)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error.into()),
        };
        let mut digest = Sha256::new();
        digest.update(self.account.as_bytes());
        digest.update([0]);
        digest.update(url.as_bytes());
        digest.update([0]);
        digest.update(generation);
        Ok(format!("response/{:x}", digest.finalize()))
    }

    fn path(&self, record: &str) -> PathBuf {
        self.root
            .join(format!("{:x}.enc", Sha256::digest(record.as_bytes())))
    }

    fn read(&self, record: &str) -> Result<Value> {
        let path = self.path(record);
        let meta = fs::symlink_metadata(&path)?;
        ensure!(
            meta.is_file() && meta.len() <= CAPACITY,
            "invalid response cache entry"
        );
        ensure!(
            meta.modified()?.elapsed()? < self.ttl,
            "expired response cache entry"
        );
        let bytes = http::limited(fs::File::open(path)?, CAPACITY)?;
        let plaintext = store::decrypt(&self.key, record, &bytes)?;
        serde_json::from_slice(&plaintext).context("invalid cached response")
    }

    fn write(&self, record: &str, value: &Value) -> Result<()> {
        let raw = Zeroizing::new(serde_json::to_vec(value)?);
        if raw.len() as u64 + 36 > CAPACITY {
            return Ok(());
        }
        let bytes = store::encrypt(&self.key, record, &raw)?;
        let lock_path = self.root.join(".lock");
        if let Ok(meta) = fs::symlink_metadata(&lock_path) {
            ensure!(meta.is_file(), "invalid response cache lock");
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options.open(lock_path)?;
        fs2::FileExt::try_lock_exclusive(&lock)?;
        let target = self.path(record);
        let mut entries = fs::read_dir(&self.root)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                if entry.path() == target {
                    return None;
                }
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.len() != 68
                    || !name.ends_with(".enc")
                    || !name[..64].bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return None;
                }
                let meta = fs::symlink_metadata(entry.path()).ok()?;
                meta.is_file().then(|| {
                    (
                        meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                        meta.len(),
                        entry.path(),
                    )
                })
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.0);
        let mut total = entries.iter().fold(bytes.len() as u64, |total, entry| {
            total.saturating_add(entry.1)
        });
        let mut count = entries.len() + 1;
        for (_, size, path) in entries {
            if count <= ENTRIES && total <= CAPACITY {
                break;
            }
            fs::remove_file(path)?;
            count -= 1;
            total = total.saturating_sub(size);
        }
        store::atomic(&target, &bytes)
    }

    pub fn fetch(&self, url: &str, fetch: impl FnOnce() -> Result<Value>) -> Result<Value> {
        if self.ttl.is_zero() {
            return fetch();
        }
        let Ok(record) = self.record(url) else {
            return fetch();
        };
        if let Ok(value) = self.read(&record) {
            return Ok(value);
        }
        let value = fetch()?;
        // Caching must not turn a successful Microsoft request into an error.
        let _ = self.write(&record, &value);
        Ok(value)
    }
}

#[cfg(test)]
#[path = "../tests/unit/cache.rs"]
mod tests;
