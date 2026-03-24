/// Top-level error type for `fixonce-core`.
///
/// Individual subsystems (auth, api, …) define their own fine-grained error
/// enums.  This type is a thin wrapper that lets callers work with a single
/// error type when they don't need to distinguish subsystem errors.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error(transparent)]
    Auth(#[from] crate::auth::AuthError),

    #[error(transparent)]
    Api(#[from] crate::api::ApiError),

    #[error(transparent)]
    Embedding(#[from] EmbeddingError),
}

/// Errors that can occur when generating embeddings.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    /// The HTTP layer failed (network error, TLS setup, etc.).
    #[error("HTTP error while calling embedding API: {0}")]
    Http(#[from] reqwest::Error),

    /// The embedding API returned a non-2xx status.
    #[error("Embedding API error {status}: {body}")]
    ApiError { status: u16, body: String },

    /// The response did not contain any embedding data.
    #[error("Embedding API returned an empty response")]
    EmptyResponse,

    /// The embedding vector has an unexpected number of dimensions.
    #[error("Expected {expected} dimensions but got {got}")]
    UnexpectedDimensions { expected: usize, got: usize },

    /// The response body could not be parsed.
    #[error("Unexpected embedding response format: {0}")]
    UnexpectedResponse(String),
}
