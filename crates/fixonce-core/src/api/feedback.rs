//! API call for submitting user feedback on a memory.

use crate::memory::types::{Feedback, FeedbackRating};

use super::{ApiClient, ApiError};

/// Submit user feedback for a memory.
///
/// `context` is an optional free-text explanation accompanying the rating.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server rejects the request.
pub async fn submit_feedback(
    client: &ApiClient,
    memory_id: &str,
    rating: FeedbackRating,
    context: Option<&str>,
) -> Result<Feedback, ApiError> {
    let payload = serde_json::json!({
        "memory_id": memory_id,
        "rating": rating,
        "context": context,
    });

    let response = client
        .post_authenticated("/rest/v1/rpc/submit_feedback")?
        .json(&payload)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_owned());
        return Err(ApiError::ServerError { status, body });
    }

    response
        .json::<Feedback>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}
