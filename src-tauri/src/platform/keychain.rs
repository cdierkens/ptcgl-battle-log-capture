use crate::ports::{SecretError, SecretStore};

/// The OS keychain. macOS uses the login Keychain; other platforms are stubbed.
pub struct KeychainSecretStore;

#[cfg(target_os = "macos")]
const SERVICE: &str = "dev.dierkens.ptcgl-tracker";

#[cfg(target_os = "macos")]
impl KeychainSecretStore {
    fn entry(account: &str) -> Result<keyring::Entry, SecretError> {
        keyring::Entry::new(SERVICE, account).map_err(|e| SecretError::Store(e.to_string()))
    }
}

#[cfg(target_os = "macos")]
impl SecretStore for KeychainSecretStore {
    fn get(&self, account: &str) -> Result<Option<String>, SecretError> {
        match Self::entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(SecretError::Store(e.to_string())),
        }
    }

    fn set(&self, account: &str, secret: &str) -> Result<(), SecretError> {
        Self::entry(account)?.set_password(secret).map_err(|e| SecretError::Store(e.to_string()))
    }

    fn delete(&self, account: &str) -> Result<(), SecretError> {
        match Self::entry(account)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(SecretError::Store(e.to_string())),
        }
    }
}

#[cfg(not(target_os = "macos"))]
impl SecretStore for KeychainSecretStore {
    fn get(&self, _account: &str) -> Result<Option<String>, SecretError> {
        Err(SecretError::Unsupported)
    }

    fn set(&self, _account: &str, _secret: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported)
    }

    fn delete(&self, _account: &str) -> Result<(), SecretError> {
        Err(SecretError::Unsupported)
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    // Touches the real login Keychain: `cargo test -- --ignored keychain`.
    #[test]
    #[ignore]
    fn keychain_round_trip() {
        let store = KeychainSecretStore;
        let account = format!("test-{}", std::process::id());
        store.set(&account, "s3cret").unwrap();
        assert_eq!(store.get(&account).unwrap().as_deref(), Some("s3cret"));
        store.delete(&account).unwrap();
        assert_eq!(store.get(&account).unwrap(), None);
        store.delete(&account).unwrap();
    }
}
