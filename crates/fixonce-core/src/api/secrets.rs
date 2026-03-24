/// Ephemeral secret retrieval.
///
/// Secrets are fetched on demand and returned as a plain `String`.  The caller
/// is responsible for using and discarding the value immediately.  This module
/// intentionally provides no caching layer so that secrets are not retained in
/// memory longer than necessary.
use serde::Deserialize;
use tracing::instrument;

use super::{ApiClient, ApiError};

/// Response payload from the `secret-get` edge function.
#[derive(Debug, Deserialize)]
struct SecretResponse {
    value: String,
}

/// Fetch the plaintext value of a named secret from the backend.
///
/// # Security note
///
/// The returned `String` is live secret material.  The caller **must** use it
/// immediately and let it drop — do not clone, log, or persist it.
///
/// # Errors
///
/// Returns [`ApiError::Unauthenticated`] when the client has no token.
/// Returns [`ApiError::Http`] on network failure.
/// Returns [`ApiError::ServerError`] when the backend rejects the request.
/// Returns [`ApiError::UnexpectedResponse`] when the payload is malformed.
#[instrument(skip(client))]
pub async fn get_secret(client: &ApiClient, name: &str) -> Result<String, ApiError> {
    let path = format!("/functions/v1/secret-get?name={name}");
    let response = client.get_authenticated(&path)?.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_owned());
        return Err(ApiError::ServerError { status, body });
    }

    let secret: SecretResponse = response.json().await.map_err(|e| {
        ApiError::UnexpectedResponse(format!("secret-get response is not valid JSON: {e}"))
    })?;

    Ok(secret.value)
}
