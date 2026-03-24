pub mod challenge;
pub mod keypair;
pub mod oauth;
pub mod token;

/// Returns the keyring service name used for all fixonce credential storage.
///
/// Defaults to `"fixonce"`.  Set the `FIXONCE_KEYRING_SERVICE` environment
/// variable to override — primarily useful for test isolation so that
/// `cargo test` never reads or writes the developer's real credentials.
pub(crate) fn keyring_service() -> String {
    std::env::var("FIXONCE_KEYRING_SERVICE").unwrap_or_else(|_| "fixonce".to_owned())
}

/// Errors that can occur in any auth subsystem.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("OAuth flow failed: {0}")]
    OAuthFailed(String),

    #[error("Key generation failed: {0}")]
    KeyGenFailed(String),

    #[error("Challenge-response failed: {0}")]
    ChallengeFailed(String),

    #[error("Token expired")]
    TokenExpired,

    /// The user has not authenticated yet.
    #[error("No token found — run `fixonce login` or `fixonce auth`")]
    NoToken,

    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
}
