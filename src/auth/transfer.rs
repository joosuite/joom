use super::{Credentials, Tokens, masked_prompt};
use crate::{http::limited, store::Store};
use anyhow::{Context, Result, ensure};
use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const MAGIC: &[u8; 8] = b"JOOMEXP1";
const LIMIT: u64 = 8 * 1024 * 1024;
const CONTEXT: &str = "joom-credential-export-v1";

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Bundle {
    version: u32,
    client: Credentials,
    accounts: Vec<Account>,
}

#[derive(Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
#[serde(deny_unknown_fields)]
struct Account {
    email: String,
    refresh_token: String,
    #[serde(default)]
    personal: bool,
}

fn email(value: &str) -> Result<String> {
    ensure!(
        value.len() <= 320
            && value.split('@').count() == 2
            && value.split('@').all(|part| !part.is_empty())
            && !value
                .chars()
                .any(|ch| ch.is_whitespace() || ch.is_control()),
        "invalid account email in credential transfer"
    );
    Ok(value.to_ascii_lowercase())
}

fn collect(store: &Store, selected: Option<&str>) -> Result<Bundle> {
    let client: Credentials = store.secret("oauth-client")?;
    let mut emails = if let Some(selected) = selected {
        vec![email(selected)?]
    } else {
        store.config()?.accounts
    };
    emails.sort();
    emails.dedup();
    ensure!(!emails.is_empty(), "no signed-in accounts to export");
    let mut accounts = Vec::new();
    for address in emails {
        let address = email(&address)?;
        let tokens: Tokens = store.secret(&format!("account/{address}"))?;
        accounts.push(Account {
            email: address,
            refresh_token: tokens.refresh_token.clone(),
            personal: tokens.personal,
        });
    }
    let bundle = Bundle {
        version: 1,
        client,
        accounts,
    };
    validate(&bundle)?;
    Ok(bundle)
}

fn validate(bundle: &Bundle) -> Result<()> {
    ensure!(bundle.version == 1, "unsupported credential export version");
    ensure!(
        !bundle.client.client_id.trim().is_empty(),
        "credential export is missing its OAuth client credentials"
    );
    ensure!(
        !bundle.accounts.is_empty() && bundle.accounts.len() <= 128,
        "credential export must contain between 1 and 128 accounts"
    );
    let mut seen = std::collections::BTreeSet::new();
    for account in &bundle.accounts {
        let address = email(&account.email)?;
        ensure!(
            seen.insert(address),
            "credential export contains a duplicate account"
        );
        ensure!(
            !account.refresh_token.trim().is_empty() && account.refresh_token.len() <= 64 * 1024,
            "credential export is missing a valid refresh token"
        );
    }
    Ok(())
}

fn derive(passphrase: &str, salt: &[u8]) -> Result<Zeroizing<Vec<u8>>> {
    let params = Params::new(64 * 1024, 3, 1, Some(32))
        .map_err(|_| anyhow::anyhow!("cannot configure export encryption"))?;
    let mut key = Zeroizing::new(vec![0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|_| anyhow::anyhow!("cannot derive export encryption key"))?;
    Ok(key)
}

fn seal(bundle: &Bundle, passphrase: &str) -> Result<Vec<u8>> {
    ensure!(
        passphrase.chars().count() >= 12,
        "use an export passphrase with at least 12 characters"
    );
    let mut salt = [0; 16];
    getrandom::fill(&mut salt).context("operating system randomness unavailable")?;
    let key = derive(passphrase, &salt)?;
    let raw = Zeroizing::new(serde_json::to_vec(bundle)?);
    ensure!(
        raw.len() as u64 <= LIMIT - 128,
        "credential export exceeds size limit"
    );
    let ciphertext = crate::store::encrypt(&key, CONTEXT, &raw)?;
    Ok([MAGIC.as_slice(), salt.as_slice(), &ciphertext].concat())
}

fn open(bytes: &[u8], passphrase: &str) -> Result<Bundle> {
    ensure!(
        bytes.len() >= 60 && bytes.len() as u64 <= LIMIT && bytes.starts_with(MAGIC),
        "invalid or unsupported credential export file"
    );
    let key = derive(passphrase, &bytes[8..24])?;
    let raw = crate::store::decrypt(&key, CONTEXT, &bytes[24..]).map_err(|_| {
        anyhow::anyhow!(
            "cannot decrypt credential export; the passphrase is incorrect or the file was damaged"
        )
    })?;
    let bundle: Bundle = serde_json::from_slice(&raw)
        .map_err(|_| anyhow::anyhow!("invalid credential export contents"))?;
    validate(&bundle)?;
    Ok(bundle)
}

fn passphrase(path: Option<&Path>, confirm: bool) -> Result<Zeroizing<String>> {
    let value = if let Some(path) = path {
        let metadata = fs::symlink_metadata(path).context("cannot read passphrase file")?;
        ensure!(
            metadata.is_file(),
            "passphrase file must be a regular file, not a symbolic link"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            ensure!(
                metadata.permissions().mode() & 0o077 == 0,
                "passphrase file must be readable only by its owner; use chmod 600"
            );
        }
        let raw = Zeroizing::new(limited(fs::File::open(path)?, 16 * 1024)?);
        let mut value = Zeroizing::new(
            std::str::from_utf8(&raw)
                .context("passphrase file must contain UTF-8 text")?
                .to_owned(),
        );
        if value.ends_with('\n') {
            value.pop();
            if value.ends_with('\r') {
                value.pop();
            }
        }
        value
    } else {
        let value = Zeroizing::new(masked_prompt("Transfer passphrase: ")?);
        if confirm {
            let repeated = Zeroizing::new(masked_prompt("Confirm transfer passphrase: ")?);
            ensure!(
                bool::from(value.as_bytes().ct_eq(repeated.as_bytes())),
                "transfer passphrases do not match"
            );
        }
        value
    };
    ensure!(!value.is_empty(), "transfer passphrase cannot be empty");
    Ok(value)
}

pub fn export(
    store: &Store,
    selected: Option<&str>,
    path: &Path,
    passphrase_file: Option<&Path>,
) -> Result<Vec<String>> {
    ensure!(
        fs::symlink_metadata(path).is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound),
        "export destination already exists or cannot be accessed; choose a new file"
    );
    let bundle = collect(store, selected)?;
    let password = passphrase(passphrase_file, true)?;
    let bytes = seal(&bundle, &password)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).context("cannot create export file")?;
    file.write_all(&bytes)?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path)
        .map_err(|error| error.error)
        .context("cannot save export file; existing files are never overwritten")?;
    Ok(bundle
        .accounts
        .iter()
        .map(|account| account.email.clone())
        .collect())
}

pub fn import(
    store: &Store,
    selected: Option<&str>,
    path: &Path,
    passphrase_file: Option<&Path>,
    headless: bool,
    replace: bool,
) -> Result<Vec<String>> {
    let bytes = limited(
        fs::File::open(path).context("cannot open credential export")?,
        LIMIT,
    )?;
    ensure!(
        bytes.starts_with(MAGIC) && bytes.len() >= 60,
        "invalid or unsupported credential export file"
    );
    let password = passphrase(passphrase_file, false)?;
    restore(
        store,
        &open(&bytes, &password)?,
        selected,
        headless,
        replace,
    )
}

fn restore(
    store: &Store,
    bundle: &Bundle,
    selected: Option<&str>,
    headless: bool,
    replace: bool,
) -> Result<Vec<String>> {
    validate(bundle)?;
    let selected = selected.map(email).transpose()?;
    let mut accounts = Vec::new();
    let mut records = vec![(
        "oauth-client".to_owned(),
        Zeroizing::new(serde_json::to_vec(&bundle.client)?),
    )];
    for account in &bundle.accounts {
        let address = email(&account.email)?;
        if selected
            .as_ref()
            .is_some_and(|selected| *selected != address)
        {
            continue;
        }
        let tokens = Tokens {
            access_token: String::new(),
            refresh_token: account.refresh_token.clone(),
            expires_at: 0,
            personal: account.personal,
        };
        records.push((
            format!("account/{address}"),
            Zeroizing::new(serde_json::to_vec(&tokens)?),
        ));
        accounts.push(address);
    }
    ensure!(
        !accounts.is_empty(),
        "the selected account is not in this credential export"
    );
    store.import_credentials(&records, &accounts, replace, headless)?;
    for account in &accounts {
        let _ = crate::cache::invalidate(&store.root, account);
    }
    Ok(accounts)
}

use subtle::ConstantTimeEq;

#[cfg(test)]
#[path = "../../tests/unit/auth_transfer.rs"]
mod tests;
