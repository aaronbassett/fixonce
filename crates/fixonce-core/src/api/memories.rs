//! API calls for creating, reading, updating, deleting, and searching memories.

use crate::memory::types::{
    CreateMemoryRequest, CreateMemoryResponse, DeleteMemoryResponse, Memory, SearchMemoryRequest,
    SearchMemoryResponse, UpdateMemoryResponse,
};

use super::{ApiClient, ApiError};

/// Create a new memory via the backend API.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server rejects the request.
pub async fn create_memory(
    client: &ApiClient,
    req: &CreateMemoryRequest,
) -> Result<CreateMemoryResponse, ApiError> {
    let response = client
        .post_authenticated("/rest/v1/rpc/create_memory")?
        .json(req)
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
        .json::<CreateMemoryResponse>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

/// Fetch a single memory by its UUID.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server returns a non-success status.
pub async fn get_memory(client: &ApiClient, id: &str) -> Result<Memory, ApiError> {
    let path = format!("/rest/v1/memories?id=eq.{id}&select=*");
    let response = client.get_authenticated(&path)?.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_owned());
        return Err(ApiError::ServerError { status, body });
    }

    // PostgREST returns a JSON array for row-level queries.
    let mut rows: Vec<Memory> = response
        .json()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))?;

    rows.pop()
        .ok_or_else(|| ApiError::UnexpectedResponse(format!("no memory found with id={id}")))
}

/// List recent memories (newest first), up to `limit`.
///
/// Queries the `PostgREST` `memory` table directly, excluding soft-deleted rows.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server rejects the request.
pub async fn list_memories(client: &ApiClient, limit: usize) -> Result<Vec<Memory>, ApiError> {
    let path =
        format!("/rest/v1/memory?deleted_at=is.null&order=created_at.desc&limit={limit}&select=*");
    let response = client.get_authenticated(&path)?.send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "(unreadable body)".to_owned());
        return Err(ApiError::ServerError { status, body });
    }

    response
        .json::<Vec<Memory>>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

/// Partially update a memory.
///
/// `updates` is a JSON object whose keys are the fields to change.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server rejects the patch.
pub async fn update_memory(
    client: &ApiClient,
    id: &str,
    updates: &serde_json::Value,
) -> Result<UpdateMemoryResponse, ApiError> {
    let path = format!("/rest/v1/memories?id=eq.{id}");
    let token = client.token.as_deref().ok_or(ApiError::Unauthenticated)?;
    let url = format!("{}{}", client.base_url, path);

    let response = client
        .http
        .patch(url)
        .bearer_auth(token)
        .json(updates)
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
        .json::<UpdateMemoryResponse>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

/// Soft-delete a memory by its UUID.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure or server rejection.
pub async fn delete_memory(client: &ApiClient, id: &str) -> Result<DeleteMemoryResponse, ApiError> {
    let path = "/rest/v1/rpc/delete_memory";
    let payload = serde_json::json!({ "memory_id": id });

    let response = client
        .post_authenticated(path)?
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
        .json::<DeleteMemoryResponse>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

/// Perform a vector similarity search over the stored memories.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure or server rejection.
pub async fn search_memories(
    client: &ApiClient,
    req: &SearchMemoryRequest,
) -> Result<SearchMemoryResponse, ApiError> {
    let path = "/rest/v1/rpc/search_memories";

    let response = client.post_authenticated(path)?.json(req).send().await?;

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
