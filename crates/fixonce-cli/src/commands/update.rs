/// `fixonce update <id>` — partially update an existing memory.
use anyhow::{Context, Result};
use clap::Args;
use fixonce_core::{
    api::{memories::update_memory, ApiClient},
    auth::token::TokenManager,
};

/// Arguments for `fixonce update`.
#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// New title (optional)
    #[arg(long)]
    pub title: Option<String>,

    /// New content (optional)
    #[arg(long)]
    pub content: Option<String>,

    /// New summary (optional)
    #[arg(long)]
    pub summary: Option<String>,
}

/// Execute `fixonce update`.
///
/// Sends only the fields that were supplied on the command line.
///
/// # Errors
///
/// Propagates errors from token loading or the update-memory API call.
pub async fn run_update(api_url: &str, id: &str, args: UpdateArgs) -> Result<()> {
    // 1. Load token
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(token);

    // 2. Build the partial update payload
    let mut patch = serde_json::Map::new();

    if let Some(title) = args.title {
        patch.insert("title".to_owned(), serde_json::Value::String(title));
    }
    if let Some(content) = args.content {
        patch.insert("content".to_owned(), serde_json::Value::String(content));
    }
    if let Some(summary) = args.summary {
        patch.insert("summary".to_owned(), serde_json::Value::String(summary));
    }

    if patch.is_empty() {
        anyhow::bail!(
            "No fields to update. Provide at least one of --title, --content, or --summary."
        );
    }

    // 3. Call the API
    let resp = update_memory(&client, id, &serde_json::Value::Object(patch))
        .await
        .context("Failed to update memory")?;

    println!("Memory updated.");
    println!("  id         : {}", resp.id);
    println!("  updated_at : {}", resp.updated_at);

    Ok(())
}
