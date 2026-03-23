pub mod challenge;
pub mod keypair;
pub mod oauth;
pub mod token;

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
