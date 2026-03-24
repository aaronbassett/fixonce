/// `fixonce get <id>` — retrieve a memory by ID.
use anyhow::{Context, Result};
use fixonce_core::{
    api::{memories::get_memory, ApiClient},
    auth::token::TokenManager,
    output::{json::format_memory_json, text::format_memory_text, toon::format_memory_toon},
};

use crate::output::OutputFormat;

/// Execute `fixonce get`.
///
/// # Errors
///
/// Propagates errors from token loading or the get-memory API call.
pub async fn run_get(api_url: &str, id: &str, format: OutputFormat) -> Result<()> {
    // 1. Load token
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(token);

    // 2. Fetch the memory
    let memory = get_memory(&client, id)
        .await
        .context("Failed to fetch memory")?;

    // 3. Format and print
    match format {
        OutputFormat::Json => print!("{}", format_memory_json(&memory)),
        OutputFormat::Toon => print!("{}", format_memory_toon(&memory)),
        OutputFormat::Text => print!("{}", format_memory_text(&memory)),
    }

    Ok(())
}
