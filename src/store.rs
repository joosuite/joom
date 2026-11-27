use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Payload},
};
use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use zeroize::Zeroizing;

#[cfg(not(target_os = "macos"))]
static KEYRING: std::sync::LazyLock<keyring_core::Result<()>> = std::sync::LazyLock::new(|| {
    #[cfg(target_os = "windows")]
    {
        windows::initialize()
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        linux::initialize()
    }
    #[cfg(not(any(unix, windows)))]
    Err(keyring_core::Error::NoDefaultStore)
});

const MAGIC: &[u8; 8] = b"GOGVLT01";
const SECRET_LIMIT: u64 = 1024 * 1024;

trait Keychain: Send + Sync {
    fn load(&self, namespace: &str) -> Result<Option<Zeroizing<Vec<u8>>>>;
    fn save(&self, namespace: &str, key: &[u8]) -> Result<()>;
}

struct OsKeychain;

impl OsKeychain {
    #[cfg(not(target_os = "macos"))]
    fn entry(namespace: &str) -> Result<keyring_core::Entry> {
        KEYRING
            .as_ref()
            .map_err(|e| anyhow::anyhow!("OS keychain unavailable: {e}"))?;
        keyring_core::Entry::new(namespace, "master-key").context("cannot access master key")
    }
}

impl Keychain for OsKeychain {
    fn load(&self, namespace: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
        #[cfg(target_os = "macos")]
        return macos::load(namespace);
        #[cfg(not(target_os = "macos"))]
        match Self::entry(namespace)?.get_secret() {
            Ok(key) => Ok(Some(Zeroizing::new(key))),
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(e).context("cannot unlock master key in OS keychain"),
        }
    }

    fn save(&self, namespace: &str, key: &[u8]) -> Result<()> {
        #[cfg(target_os = "macos")]
        return macos::save(namespace, key);
        #[cfg(not(target_os = "macos"))]
        Self::entry(namespace)?
            .set_secret(key)
            .context("cannot save master key in OS keychain")
    }
}

#[cfg(target_os = "macos")]
#[path = "store/macos.rs"]
mod macos;

#[cfg(target_os = "windows")]
#[path = "store/windows.rs"]
mod windows;

#[cfg(all(unix, not(target_os = "macos")))]
#[path = "store/linux.rs"]
mod linux;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct VaultIdentity {
    version: u32,
    service: String,
    #[serde(default, skip_serializing_if = "KeyStorage::is_keychain")]
    key_storage: KeyStorage,
}

#[derive(Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum KeyStorage {
    #[default]
    OsKeychain,
    File,
}

impl KeyStorage {
    fn is_keychain(&self) -> bool {
        *self == Self::OsKeychain
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct Config {
    pub account: Option<String>,
    #[serde(default)]
    pub accounts: Vec<String>,
}

pub struct Store {
    pub root: PathBuf,
    keychain: Arc<dyn Keychain>,
    legacy_root: Option<PathBuf>,
    default_vault: bool,
    key: Mutex<Option<Zeroizing<Vec<u8>>>>,
}

fn validate_key(key: &[u8]) -> Result<()> {
    anyhow::ensure!(
        key.len() == 32,
        "invalid master key length; refusing to replace it"
    );
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("write to string");
    }
    result
}

fn valid_service(service: &str) -> bool {
    service == "joom/vault"
        || service
            .strip_prefix("joom/vault/")
            .is_some_and(|id| id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn legacy_namespace(root: &Path) -> Result<String> {
    Ok(format!(
        "joom-rust/vault/{:x}",
        Sha256::digest(std::path::absolute(root)?.to_string_lossy().as_bytes())
    ))
}

fn has_state(root: &Path) -> Result<bool> {
    match fs::symlink_metadata(root) {
        Ok(metadata) => {
            anyhow::ensure!(metadata.is_dir(), "credential path is not a directory");
            Ok(fs::read_dir(root)?.next().is_some())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

impl Store {
    pub fn new(home: Option<&str>) -> Result<Self> {
        let (root, legacy_root) = if let Some(home) = home {
            (
                PathBuf::from(home).join("joom"),
                PathBuf::from(home).join("joom-rust"),
            )
        } else {
            let base = directories_next::BaseDirs::new()
                .context("cannot resolve config directory; use --home")?
                .config_dir()
                .to_owned();
            (base.join("joom"), base.join("joom-rust"))
        };
        let root = if has_state(&legacy_root)? {
            anyhow::ensure!(
                !has_state(&root)?,
                "both {} and {} contain state; refusing to choose or change either vault",
                legacy_root.display(),
                root.display()
            );
            legacy_root.clone()
        } else {
            root
        };
        Ok(Self {
            root,
            keychain: Arc::new(OsKeychain),
            legacy_root: Some(legacy_root),
            default_vault: home.is_none(),
            key: Mutex::new(None),
        })
    }

    pub fn config(&self) -> Result<Config> {
        match fs::File::open(self.root.join("config.json")) {
            Ok(file) => serde_json::from_slice(&crate::http::limited(file, SECRET_LIMIT)?)
                .context("invalid config.json"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_config(&self, config: &Config) -> Result<()> {
        atomic(
            &self.root.join("config.json"),
            &serde_json::to_vec_pretty(config)?,
        )
    }

    fn identity(&self) -> Result<Option<VaultIdentity>> {
        let path = self.root.join("vault.json");
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                anyhow::ensure!(metadata.is_file(), "invalid vault identity file");
                let identity: VaultIdentity =
                    serde_json::from_slice(&crate::http::limited(fs::File::open(&path)?, 1024)?)
                        .context("invalid vault identity")?;
                anyhow::ensure!(
                    identity.version == 1 && valid_service(&identity.service),
                    "invalid vault identity"
                );
                Ok(Some(identity))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn namespace(&self) -> Result<String> {
        if let Some(identity) = self.identity()? {
            Ok(identity.service)
        } else if self.legacy_root.as_ref() == Some(&self.root) {
            legacy_namespace(&self.root)
        } else if self.default_vault {
            Ok("joom/vault".to_owned())
        } else {
            anyhow::bail!("vault is not initialized; run joom auth init")
        }
    }

    pub fn uses_key_file(&self) -> Result<bool> {
        Ok(self
            .identity()?
            .is_some_and(|identity| identity.key_storage == KeyStorage::File))
    }

    fn load_file_key(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let path = self.root.join("master.key");
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        anyhow::ensure!(
            metadata.is_file(),
            "invalid master.key file; symbolic links are not accepted"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            anyhow::ensure!(
                metadata.permissions().mode() & 0o077 == 0,
                "master.key must be readable only by its owner; run chmod 600 on that file"
            );
        }
        let key = Zeroizing::new(crate::http::limited(fs::File::open(&path)?, 32)?);
        validate_key(&key)?;
        Ok(Some(key))
    }

    fn lock(&self) -> Result<fs::File> {
        private_dir(&self.root)?;
        let path = self.root.join(".vault.lock");
        if let Ok(meta) = fs::symlink_metadata(&path) {
            anyhow::ensure!(meta.is_file(), "invalid vault lock file");
        }
        let mut options = fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match FileExt::try_lock_exclusive(&file) {
                Ok(()) => return Ok(file),
                Err(e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        && std::time::Instant::now() < deadline =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(25));
                }
                Err(e) => return Err(e).context("cannot lock credential vault"),
            }
        }
    }

    pub(crate) fn master_key(&self) -> Result<Zeroizing<Vec<u8>>> {
        let _lock = self.lock()?;
        self.load_key()?
            .context("master key missing; existing encrypted secrets require their original key")
    }

    fn load_key(&self) -> Result<Option<Zeroizing<Vec<u8>>>> {
        let mut cached = self
            .key
            .lock()
            .map_err(|_| anyhow::anyhow!("credential session unavailable"))?;
        if let Some(key) = cached.as_ref() {
            return Ok(Some(key.clone()));
        }
        if self.uses_key_file()? {
            let key = self.load_file_key()?;
            *cached = key.clone();
            return Ok(key);
        }
        let namespace = self.namespace()?;
        let key = if let Some(key) = self.keychain.load(&namespace)? {
            Some(key)
        } else if let Some(root) = &self.legacy_root {
            let old_service = legacy_namespace(root)?;
            if old_service == namespace {
                None
            } else {
                self.keychain.load(&old_service)?
            }
        } else {
            None
        };
        if let Some(key) = key {
            validate_key(&key)?;
            *cached = Some(key.clone());
            Ok(Some(key))
        } else {
            Ok(None)
        }
    }

    fn retain_key(&self, namespace: &str, key: &[u8]) -> Result<()> {
        self.keychain.save(namespace, key)?;
        let saved = self.keychain.load(namespace)?
            .context("master key missing; run joom auth init; existing encrypted secrets require their original key")?;
        anyhow::ensure!(
            bool::from(subtle::ConstantTimeEq::ct_eq(key, saved.as_slice())),
            "OS keychain did not retain the master key"
        );
        Ok(())
    }

    pub fn initialize(&self) -> Result<()> {
        let _lock = self.lock()?;
        self.initialize_locked(false)
    }

    pub fn initialize_headless(&self) -> Result<()> {
        let _lock = self.lock()?;
        self.initialize_locked(true)
    }

    fn initialize_locked(&self, headless: bool) -> Result<()> {
        if headless {
            anyhow::ensure!(
                self.legacy_root.as_ref() != Some(&self.root),
                "this is a legacy keychain vault; use --home with a new directory for a headless vault"
            );
            anyhow::ensure!(
                self.identity()?.is_none() || self.uses_key_file()?,
                "this vault uses the OS keychain; use --home with a new directory for a headless vault"
            );
        }
        let records = self.root.join("secrets");
        if records.exists() && fs::read_dir(&records)?.next().is_some() {
            anyhow::ensure!(
                !headless || self.uses_key_file()?,
                "existing encrypted credentials require their original keychain; use --home with a new directory"
            );
            // Initializing storage is not a credential read or an upgrade step.
            // Existing credentials and their Keychain permissions stay intact.
            return Ok(());
        }
        if !self.root.join("vault.json").exists() && self.legacy_root.as_ref() != Some(&self.root) {
            let service = if self.default_vault {
                "joom/vault".to_owned()
            } else {
                let mut id = [0_u8; 16];
                getrandom::fill(&mut id).context("operating system randomness unavailable")?;
                format!("joom/vault/{}", hex(&id))
            };
            atomic(
                &self.root.join("vault.json"),
                &serde_json::to_vec(&VaultIdentity {
                    version: 1,
                    service,
                    key_storage: if headless {
                        KeyStorage::File
                    } else {
                        KeyStorage::OsKeychain
                    },
                })?,
            )?;
        }
        if self.load_key()?.is_some() {
            return Ok(());
        }
        let mut key = Zeroizing::new(vec![0_u8; 32]);
        getrandom::fill(&mut key).context("operating system randomness unavailable")?;
        if self.uses_key_file()? {
            let path = self.root.join("master.key");
            anyhow::ensure!(
                !path.try_exists()?,
                "refusing to replace an existing master.key"
            );
            atomic(&path, &key)?;
        } else {
            self.retain_key(&self.namespace()?, &key)?;
        }
        *self
            .key
            .lock()
            .map_err(|_| anyhow::anyhow!("credential session unavailable"))? = Some(key);
        Ok(())
    }

    fn record(&self, name: &str) -> PathBuf {
        self.root
            .join("secrets")
            .join(format!("{:x}.enc", Sha256::digest(name.as_bytes())))
    }

    pub fn secret<T: DeserializeOwned>(&self, name: &str) -> Result<T> {
        let _lock = self.lock()?;
        let key = self
            .load_key()?
            .context("master key missing; existing encrypted secrets require their original key")?;
        let path = self.record(name);
        let meta = fs::symlink_metadata(&path).context(
            "encrypted credential missing; run joom auth enroll or joom auth <account> login <service-list>",
        )?;
        anyhow::ensure!(meta.is_file(), "invalid encrypted credential file");
        let bytes = crate::http::limited(fs::File::open(path)?, SECRET_LIMIT + 64)?;
        let raw = decrypt(&key, name, &bytes)?;
        serde_json::from_slice(&raw).context("invalid encrypted credential data")
    }

    pub fn set_secret(&self, name: &str, secret: &impl Serialize) -> Result<()> {
        let _lock = self.lock()?;
        let key = self
            .load_key()?
            .context("master key missing; existing encrypted secrets require their original key")?;
        let raw = Zeroizing::new(serde_json::to_vec(secret)?);
        anyhow::ensure!(
            raw.len() as u64 <= SECRET_LIMIT,
            "credential exceeds size limit"
        );
        atomic(&self.record(name), &encrypt(&key, name, &raw)?)
    }

    pub fn delete_secret(&self, name: &str) -> Result<()> {
        let _lock = self.lock()?;
        match fs::remove_file(self.record(name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e).context("cannot remove encrypted credential"),
        }
    }

    pub(crate) fn import_credentials(
        &self,
        records: &[(String, Zeroizing<Vec<u8>>)],
        accounts: &[String],
        replace: bool,
        headless: bool,
    ) -> Result<()> {
        let _lock = self.lock()?;
        self.initialize_locked(headless)?;
        let key = self
            .load_key()?
            .context("master key missing; existing encrypted secrets require their original key")?;
        let mut config = self.config()?;
        let mut writes = Vec::new();
        for (name, raw) in records {
            let path = self.record(name);
            let previous = match fs::symlink_metadata(&path) {
                Ok(metadata) => {
                    anyhow::ensure!(metadata.is_file(), "invalid encrypted credential file");
                    Some(crate::http::limited(
                        fs::File::open(&path)?,
                        SECRET_LIMIT + 64,
                    )?)
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            if let Some(previous) = &previous {
                let current = decrypt(&key, name, previous)?;
                if name == "oauth-client" {
                    if current.as_slice() == raw.as_slice() {
                        continue;
                    }
                    anyhow::ensure!(
                        replace,
                        "this vault has different OAuth client credentials; use --replace to replace them, or --home for a separate vault"
                    );
                    anyhow::ensure!(
                        config
                            .accounts
                            .iter()
                            .all(|account| accounts.contains(account)),
                        "replacing this OAuth client would affect other accounts; use --home with a separate vault"
                    );
                } else {
                    anyhow::ensure!(
                        replace,
                        "credentials for {} already exist; use --replace to replace them",
                        name.strip_prefix("account/").unwrap_or(name)
                    );
                }
            }
            anyhow::ensure!(
                raw.len() as u64 <= SECRET_LIMIT,
                "credential exceeds size limit"
            );
            writes.push((path, encrypt(&key, name, raw)?, previous));
        }
        for account in accounts {
            if !config.accounts.contains(account) {
                config.accounts.push(account.clone());
            }
        }
        if config.account.is_none() {
            config.account = accounts.first().cloned();
        }
        let config_path = self.root.join("config.json");
        let previous_config = match fs::File::open(&config_path) {
            Ok(file) => Some(crate::http::limited(file, SECRET_LIMIT)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        writes.push((
            config_path,
            serde_json::to_vec_pretty(&config)?,
            previous_config,
        ));
        for (index, (path, bytes, _)) in writes.iter().enumerate() {
            if let Err(error) = atomic(path, bytes) {
                for (path, _, previous) in writes[..index].iter().rev() {
                    match previous {
                        Some(previous) => atomic(path, previous),
                        None => fs::remove_file(path).map_err(Into::into),
                    }
                    .context("credential import failed and could not restore the previous vault")?;
                }
                return Err(error)
                    .context("credential import failed; previous credentials restored");
            }
        }
        Ok(())
    }
}

fn aad(name: &str) -> Vec<u8> {
    [MAGIC.as_slice(), name.as_bytes()].concat()
}

pub(crate) fn encrypt(key: &[u8], name: &str, raw: &[u8]) -> Result<Vec<u8>> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("invalid master key"))?;
    let mut nonce = [0_u8; 12];
    getrandom::fill(&mut nonce).context("operating system randomness unavailable")?;
    let ciphertext = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: raw,
                aad: &aad(name),
            },
        )
        .map_err(|_| anyhow::anyhow!("credential encryption failed"))?;
    Ok([MAGIC.as_slice(), &nonce, &ciphertext].concat())
}

pub(crate) fn decrypt(key: &[u8], name: &str, bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    anyhow::ensure!(
        bytes.len() >= 36 && &bytes[..8] == MAGIC,
        "invalid encrypted credential format"
    );
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|_| anyhow::anyhow!("invalid master key"))?;
    let nonce: [u8; 12] = bytes[8..20].try_into()?;
    let raw = cipher
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: &bytes[20..],
                aad: &aad(name),
            },
        )
        .map_err(|_| {
            anyhow::anyhow!(
                "credential authentication failed; data was modified or the master key is incorrect"
            )
        })?;
    Ok(Zeroizing::new(raw))
}

pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("file has no parent directory")?;
    private_dir(parent)?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/store.rs"]
mod tests;
