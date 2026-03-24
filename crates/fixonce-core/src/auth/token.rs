/// JWT management.
///
/// Tokens are stored in `~/.config/fixonce/credentials.json` with mode 0600,
/// following the same pattern as the GitHub CLI. No system keychain is used,
/// so there are no OS permission prompts.
///
/// Expiry is checked by decoding the JWT payload and inspecting the `exp`
/// claim, with no crypto verification (the server is the authoritative source
/// of truth for validity).
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use super::AuthError;

/// On-disk credentials file shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Credentials {
    #[serde(skip_serializing_if = "Option::is_none")]
    access_token: Option<String>,
}

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

    /// Persist `token` to the credentials file.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] if the file cannot be written.
    pub fn store_token(&self, token: &str) -> Result<(), AuthError> {
        let path = Self::credentials_path()?;

        // Ensure parent directory exists.
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AuthError::KeyringError(format!("cannot create config directory: {e}"))
            })?;
        }

        let creds = Credentials {
            access_token: Some(token.to_owned()),
        };
        let json = serde_json::to_string_pretty(&creds).map_err(|e| {
            AuthError::KeyringError(format!("cannot serialise credentials: {e}"))
        })?;

        fs::write(&path, &json)
            .map_err(|e| AuthError::KeyringError(format!("cannot write credentials: {e}")))?;

        // Set file permissions to 0600 (owner read/write only).
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|e| {
                AuthError::KeyringError(format!("cannot set file permissions: {e}"))
            })?;
        }

        Ok(())
    }

    /// Load the stored JWT from the credentials file.
    ///
    /// Returns `Ok(None)` when no credentials file exists or the token field
    /// is absent.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] for unexpected I/O or parse failures.
    pub fn load_token(&self) -> Result<Option<String>, AuthError> {
        let path = Self::credentials_path()?;

        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read_to_string(&path)
            .map_err(|e| AuthError::KeyringError(format!("cannot read credentials: {e}")))?;
        let creds: Credentials = serde_json::from_str(&data)
            .map_err(|e| AuthError::KeyringError(format!("cannot parse credentials: {e}")))?;

        Ok(creds.access_token)
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

    /// Delete the stored token from the credentials file.
    ///
    /// Succeeds silently when no credentials file exists.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::KeyringError`] for unexpected I/O failures.
    pub fn clear_token(&self) -> Result<(), AuthError> {
        let path = Self::credentials_path()?;
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| AuthError::KeyringError(format!("cannot remove credentials: {e}")))?;
        }
        Ok(())
    }

    // --- private helpers ---

    fn credentials_path() -> Result<PathBuf, AuthError> {
        let config_dir = dirs::config_dir().ok_or_else(|| {
            AuthError::KeyringError("cannot determine config directory".to_owned())
        })?;
        Ok(config_dir.join("fixonce").join("credentials.json"))
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
}
