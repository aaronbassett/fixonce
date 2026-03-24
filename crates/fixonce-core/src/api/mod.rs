pub mod client;
pub mod dashboard;
pub mod feedback;
pub mod memories;
pub mod search;
pub mod secrets;

pub use client::ApiClient;

/// Errors that can occur in API calls.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Not authenticated — run `fixonce login` or `fixonce auth`")]
    Unauthenticated,

    #[error("Server returned an error: {status} — {body}")]
    ServerError {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Unexpected response format: {0}")]
    UnexpectedResponse(String),
}
