/// JWT management.
///
/// Tokens are stored in the OS keyring — never written to disk in plain text.
/// Expiry is checked by decoding the JWT payload and inspecting the `exp`
/// claim, with no crypto verification (the server is the authoritative source
/// of truth for validity).
use base64::Engine as _;
use keyring::Entry;
use serde::Deserialize;

use super::AuthError;

const KEYRING_TOKEN_LABEL: &str = "jwt";

/// Manages the local JWT lifecycle (store, load, expiry, clear).
pub struct TokenManager;

/// Minimal JWT payload fields we care about.
#[derive(Debug, Deserialize)]
struct JwtPayload {
    exp: Option<u64>,
}

impl TokenManager {
    /// Create a new [`TokenManager`].
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Persist `token` in the OS keyring.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] if the keyring rejects the write.
    pub fn store_token(&self, token: &str) -> Result<(), AuthError> {
        let entry = Self::entry()?;
        entry
            .set_password(token)
            .map_err(|e| AuthError::KeyringError(format!("cannot store JWT: {e}")))?;
        Ok(())
    }

    /// Load the stored JWT from the OS keyring.
    ///
    /// Returns `Ok(None)` when no token has been stored yet.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] for unexpected keyring failures.
    pub fn load_token(&self) -> Result<Option<String>, AuthError> {
        let entry = Self::entry()?;
        match entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AuthError::KeyringError(format!("cannot load JWT: {e}"))),
        }
    }

    /// Return `true` if `token` has expired or its expiry cannot be determined.
    ///
    /// Expiry is checked against the system clock.  If the payload cannot be
    /// decoded, the token is treated as expired (fail-safe).
    #[must_use]
    pub fn is_expired(&self, token: &str) -> bool {
        match Self::decode_payload(token) {
            Some(payload) => {
                let Some(exp) = payload.exp else {
                    // No `exp` claim — treat as never-expiring (e.g. API keys).
                    return false;
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(u64::MAX);
                now >= exp
            }
            None => true, // Can't parse → treat as expired.
        }
    }

    /// Delete the stored JWT from the OS keyring.
    ///
    /// Succeeds silently when no token exists.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] for unexpected keyring failures.
    pub fn clear_token(&self) -> Result<(), AuthError> {
        let entry = Self::entry()?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AuthError::KeyringError(format!("cannot clear JWT: {e}"))),
        }
    }

    // --- private helpers ---

    fn entry() -> Result<Entry, AuthError> {
        Entry::new(&super::keyring_service(), KEYRING_TOKEN_LABEL)
            .map_err(|e| AuthError::KeyringError(format!("cannot access keyring: {e}")))
    }

    /// Decode the middle (payload) segment of a JWT without signature
    /// verification.
    fn decode_payload(token: &str) -> Option<JwtPayload> {
        let payload_b64 = token.split('.').nth(1)?;
        // JWT uses URL-safe base64 without padding.
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(payload_b64)
            .ok()?;
        serde_json::from_slice(&bytes).ok()
    }
}

impl Default for TokenManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    /// Build a minimal JWT with the given `exp` claim.
    fn fake_jwt(exp: Option<u64>) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"EdDSA","typ":"JWT"}"#);
        let payload_json = match exp {
            Some(e) => format!(r#"{{"sub":"test","exp":{e}}}"#),
            None => r#"{"sub":"test"}"#.to_owned(),
        };
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_json);
        format!("{header}.{payload}.fakesig")
    }

    /// RAII guard that restores the keyring env var on drop (even on panic).
    struct KeyringGuard;

    impl Drop for KeyringGuard {
        fn drop(&mut self) {
            let _ = TokenManager::new().clear_token();
            std::env::remove_var("FIXONCE_KEYRING_SERVICE");
        }
    }

    /// Set up an isolated keyring service for a test, returning the service
    /// name and a guard that cleans up on drop (panic-safe).
    /// Must be paired with `#[serial(keyring)]` to prevent env var races.
    fn setup_test_keyring() -> (String, KeyringGuard) {
        let service = format!("fixonce-test-{}", std::process::id());
        std::env::set_var("FIXONCE_KEYRING_SERVICE", &service);
        // Ensure a clean slate: clear any leftover token.
        let _ = TokenManager::new().clear_token();
        (service, KeyringGuard)
    }

    #[test]
    fn expired_token_detected() {
        let mgr = TokenManager::new();
        // exp in the past
        assert!(mgr.is_expired(&fake_jwt(Some(1))));
    }

    #[test]
    fn future_token_not_expired() {
        let mgr = TokenManager::new();
        // exp far in the future
        assert!(!mgr.is_expired(&fake_jwt(Some(u64::MAX / 2))));
    }

    #[test]
    fn token_without_exp_is_not_expired() {
        let mgr = TokenManager::new();
        assert!(!mgr.is_expired(&fake_jwt(None)));
    }

    #[test]
    fn garbage_token_is_expired() {
        let mgr = TokenManager::new();
        assert!(mgr.is_expired("not.a.jwt"));
    }

    #[test]
    #[serial(keyring)]
    fn isolated_load_returns_none_when_no_token() {
        let (_service, _guard) = setup_test_keyring();
        let mgr = TokenManager::new();
        // In an isolated keyring service, there should be no stored token
        // regardless of whether the developer has real credentials.
        let result = mgr.load_token();
        match result {
            Ok(None) | Err(_) => {} // No entry or backend unavailable — both fine.
            Ok(Some(_)) => panic!("isolated keyring should not contain a token"),
        }
    }

    #[test]
    #[serial(keyring)]
    fn override_does_not_leak_to_default_service() {
        {
            let (test_service, _guard) = setup_test_keyring();
            assert_ne!(
                test_service, "fixonce",
                "test service must differ from default"
            );
        }
        // After guard drops, the env var is removed and service returns to default.
        assert_eq!(super::super::keyring_service(), "fixonce");
    }

    #[test]
    #[serial(keyring)]
    fn keyring_service_reads_env_var() {
        let _guard = KeyringGuard;
        std::env::set_var("FIXONCE_KEYRING_SERVICE", "custom-service");
        assert_eq!(super::super::keyring_service(), "custom-service");
    }

    #[test]
    #[serial(keyring)]
    fn keyring_service_defaults_to_fixonce() {
        // Ensure env var is not set, then check default.
        std::env::remove_var("FIXONCE_KEYRING_SERVICE");
        assert_eq!(super::super::keyring_service(), "fixonce");
    }
}
