//! Search API — thin wrapper around the `hybrid_search` Supabase RPC.
//!
//! This module exposes [`search_memories`] which calls the `hybrid_search`
//! edge-function RPC that combines full-text search and vector similarity.
//!
//! The function in [`crate::api::memories`] calls the `search_memories` RPC;
//! this module calls `hybrid_search` for the richer combined mode and
//! re-exports the shared types.

use crate::memory::types::{SearchMemoryRequest, SearchMemoryResponse};

use super::{ApiClient, ApiError};

/// Perform a hybrid (FTS + vector) search via the `hybrid_search` edge function.
///
/// Falls back to the standard `search_memories` RPC if the edge function is
/// not available (status 404).
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server returns a non-success status.
pub async fn search_memories(
    client: &ApiClient,
    req: &SearchMemoryRequest,
) -> Result<SearchMemoryResponse, ApiError> {
    let path = "/rest/v1/rpc/hybrid_search";

    let response = client.post_authenticated(path)?.json(req).send().await?;

    // If the hybrid_search function is unavailable, fall back to standard search.
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return crate::api::memories::search_memories(client, req).await;
    }

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_owned());
        return Err(ApiError::ServerError { status, body });
    }

    response
        .json::<SearchMemoryResponse>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::memory::types::SearchMemoryRequest;

    /// The request type serialises correctly.
    #[test]
    fn search_request_serialises() {
        let req = SearchMemoryRequest {
            query: "how do I fix ENOMEM?".to_owned(),
            limit: Some(10),
            threshold: Some(0.75),
            language: Some("rust".to_owned()),
        };
        let json = serde_json::to_value(&req).expect("must serialise");
        assert_eq!(json["query"], "how do I fix ENOMEM?");
        assert_eq!(json["limit"], 10);
        assert!((json["threshold"].as_f64().unwrap() - 0.75).abs() < f64::EPSILON);
    }

    /// Omitting optional fields produces the correct JSON shape.
    #[test]
    fn search_request_optional_fields_omitted() {
        let req = SearchMemoryRequest {
            query: "query text".to_owned(),
            limit: None,
            threshold: None,
            language: None,
        };
        let json = serde_json::to_value(&req).expect("must serialise");
        // These are Option<T> and should serialise as null when absent.
        assert!(json["limit"].is_null());
        assert!(json["threshold"].is_null());
        assert!(json["language"].is_null());
    }
}
