//! Master keys stored in the user's Secret Service over D-Bus.

pub(super) fn initialize() -> keyring_core::Result<()> {
    keyring_core::set_default_store(dbus_secret_service_keyring_store::Store::new()?);
    Ok(())
}
