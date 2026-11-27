//! Master keys stored in Windows Credential Manager.

pub(super) fn initialize() -> keyring_core::Result<()> {
    keyring_core::set_default_store(windows_native_keyring_store::Store::new()?);
    Ok(())
}
