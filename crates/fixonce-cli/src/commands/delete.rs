/// `fixonce delete <id>` — soft-delete a memory.
use anyhow::{Context, Result};
use fixonce_core::{
    api::{memories::delete_memory, ApiClient},
    auth::token::TokenManager,
};

/// Execute `fixonce delete`.
///
/// # Errors
///
/// Propagates errors from token loading or the delete-memory API call.
pub async fn run_delete(api_url: &str, id: &str) -> Result<()> {
    // 1. Load token
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(token);

    // 2. Soft-delete via the API
    let resp = delete_memory(&client, id)
        .await
        .context("Failed to delete memory")?;

    println!("Memory deleted.");
    println!("  id         : {}", resp.id);
    println!("  deleted_at : {}", resp.deleted_at);

    Ok(())
}
