//! Secure secret storage abstraction (DEC-026 / SEC-009). The Langfuse credential keys live in
//! the macOS Keychain via the `keyring` crate, behind a small trait so the config resolver and IPC
//! layer are unit-testable with an in-memory fake — no real Keychain, no network in CI. Error
//! messages are coarse and secret-free: a value is never echoed back, not even on failure.

/// Service name for every Keychain entry — the app bundle identifier (`tauri.conf.json`).
pub const SERVICE: &str = "dev.vire.app";

/// A read/write/clear store for individual secret values, keyed by an opaque account name.
/// Implementors MUST NOT log, echo, or otherwise surface the stored value, including in errors.
pub trait SecretStore: Send + Sync {
    /// The stored value for `account`, or `None` when no entry exists.
    fn get(&self, account: &str) -> Result<Option<String>, SecretStoreError>;
    /// Create or replace the entry for `account`.
    fn set(&self, account: &str, value: &str) -> Result<(), SecretStoreError>;
    /// Remove the entry for `account`. Clearing an absent entry is a success (idempotent).
    fn delete(&self, account: &str) -> Result<(), SecretStoreError>;
}

/// A secret-free secure-storage failure. The message is a stable, coarse description and never
/// contains key material or the underlying driver string.
#[derive(Debug, Clone)]
pub struct SecretStoreError(pub String);

impl std::fmt::Display for SecretStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for SecretStoreError {}

/// macOS Keychain-backed store (`keyring` crate / Security framework). Every account is namespaced
/// under the `SERVICE` bundle id. Keychain access is a blocking OS call and stays in the Rust core.
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    pub fn new() -> Self {
        KeyringSecretStore {
            service: SERVICE.to_string(),
        }
    }

    fn entry(&self, account: &str) -> Result<keyring::Entry, SecretStoreError> {
        keyring::Entry::new(&self.service, account)
            .map_err(|_| SecretStoreError("could not access the system keychain".into()))
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for KeyringSecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretStoreError> {
        match self.entry(account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(SecretStoreError(
                "could not read the credential from the system keychain".into(),
            )),
        }
    }

    fn set(&self, account: &str, value: &str) -> Result<(), SecretStoreError> {
        self.entry(account)?
            .set_password(value)
            .map_err(|_| SecretStoreError("could not store the credential in the system keychain".into()))
    }

    fn delete(&self, account: &str) -> Result<(), SecretStoreError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretStoreError(
                "could not remove the credential from the system keychain".into(),
            )),
        }
    }
}

/// In-memory fake used by tests so resolver/IPC logic runs with no real Keychain. Compiled only in
/// test builds; never shipped.
#[cfg(test)]
#[derive(Default)]
pub struct MemorySecretStore {
    inner: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

#[cfg(test)]
impl SecretStore for MemorySecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretStoreError> {
        Ok(self.inner.lock().unwrap().get(account).cloned())
    }
    fn set(&self, account: &str, value: &str) -> Result<(), SecretStoreError> {
        self.inner
            .lock()
            .unwrap()
            .insert(account.to_string(), value.to_string());
        Ok(())
    }
    fn delete(&self, account: &str) -> Result<(), SecretStoreError> {
        self.inner.lock().unwrap().remove(account);
        Ok(())
    }
}
