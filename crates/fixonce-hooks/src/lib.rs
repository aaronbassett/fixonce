//! Claude Code lifecycle hooks for the `FixOnce` memory system.
//!
//! Each hook is designed to run in under 3 seconds and must never block
//! the Claude Code agent — shell wrappers always exit 0.
//!
//! # Edge cases
//!
//! - **EC-41** — 3-second timeout enforced in shell scripts via `timeout(1)`.
//! - **EC-42** — Shell scripts check for the `fixonce` binary before calling.
//! - **EC-43** — [`HookError::Unauthenticated`] is handled silently; hooks skip
//!   gracefully when no token is stored or the stored token has expired.

pub mod post_tool_use;
pub mod pre_tool_use;
pub mod session_start;
pub mod stop;
pub mod user_prompt;

use fixonce_core::auth::token::TokenManager;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in any hook handler.
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// An API call failed.
    #[error("API error: {0}")]
    Api(#[from] fixonce_core::api::ApiError),

    /// The authentication subsystem failed.
    #[error("Auth error: {0}")]
    Auth(#[from] fixonce_core::auth::AuthError),

    /// The hook took longer than the permitted budget.
    #[error("Hook timed out after {0}s")]
    Timeout(u64),

    /// No usable token available — skip silently (EC-43).
    ///
    /// Returned when the token is missing, expired, or malformed.
    #[error("Not authenticated — skipping hook")]
    Unauthenticated,
}

// ---------------------------------------------------------------------------
// Shared auth helper
// ---------------------------------------------------------------------------

/// Load the stored JWT and verify it is still usable.
///
/// Returns [`HookError::Unauthenticated`] when the token is missing, expired,
/// or malformed.  All hooks should use this instead of checking token presence
/// alone.
pub(crate) fn load_valid_token() -> Result<String, HookError> {
    let mgr = TokenManager::new();
    let Some(token) = mgr.load_token().map_err(HookError::Auth)? else {
        return Err(HookError::Unauthenticated);
    };
    if mgr.is_expired(&token) {
        return Err(HookError::Unauthenticated);
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_valid_token_fails_when_no_token_stored() {
        let result = load_valid_token();
        assert!(
            result.is_err(),
            "should return an error when no token is stored"
        );
    }

    #[test]
    fn expired_token_is_rejected_by_token_manager() {
        let mgr = TokenManager::new();
        let expired = fake_jwt(Some(1));
        assert!(
            mgr.is_expired(&expired),
            "token with past exp must be expired"
        );
    }

    #[test]
    fn valid_token_passes_expiry_check() {
        let mgr = TokenManager::new();
        let valid = fake_jwt(Some(u64::MAX / 2));
        assert!(
            !mgr.is_expired(&valid),
            "token with future exp must not be expired"
        );
    }

    #[test]
    fn malformed_token_is_treated_as_expired() {
        let mgr = TokenManager::new();
        assert!(
            mgr.is_expired("not.a.jwt"),
            "malformed token must be treated as expired"
        );
    }

    #[test]
    fn token_without_exp_is_not_expired() {
        let mgr = TokenManager::new();
        let no_exp = fake_jwt(None);
        assert!(
            !mgr.is_expired(&no_exp),
            "token without exp claim must not be expired"
        );
    }

    fn fake_jwt(exp: Option<u64>) -> String {
        use base64::Engine as _;
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(r#"{"alg":"EdDSA","typ":"JWT"}"#);
        let payload_json = match exp {
            Some(e) => format!(r#"{{"sub":"test","exp":{e}}}"#),
            None => r#"{"sub":"test"}"#.to_owned(),
        };
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_json);
        format!("{header}.{payload}.fakesig")
    }
}
