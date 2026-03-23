//! `fixonce lineage <id>` — display the lineage chain for a memory.
//!
//! Fetches all lineage events for the given memory ID from the API and prints
//! them in chronological order (root → tip).

use anyhow::{Context, Result};
use fixonce_core::{
    api::{memories::get_memory, ApiClient, ApiError},
    auth::token::TokenManager,
    memory::lineage::{build_chain, LineageEvent},
};

use crate::output::OutputFormat;

// ---------------------------------------------------------------------------
// API helper
// ---------------------------------------------------------------------------

/// Fetch lineage events for `memory_id` from the backend.
async fn fetch_lineage(client: &ApiClient, memory_id: &str) -> Result<Vec<LineageEvent>, ApiError> {
    let path = format!("/rest/v1/memory_lineage?memory_id=eq.{memory_id}&order=created_at.asc");
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
        .json::<Vec<LineageEvent>>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce lineage <memory_id>`.
///
/// # Errors
///
/// Propagates errors from token loading or API calls.
pub async fn run_lineage(api_url: &str, memory_id: &str, format: OutputFormat) -> Result<()> {
    // 1. Load auth token.
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(&token);

    // 2. Verify the memory exists (provides a useful error message if not).
    let _memory = get_memory(&client, memory_id)
        .await
        .with_context(|| format!("Memory '{memory_id}' not found"))?;

    // 3. Fetch lineage events from the API.
    let events = fetch_lineage(&client, memory_id)
        .await
        .with_context(|| format!("Failed to fetch lineage for '{memory_id}'"))?;

    // 4. Build ordered chain.
    let chain = build_chain(memory_id, &events);

    // 5. Format and print.
    match format {
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&chain).unwrap_or_default();
            println!("{output}");
        }
        OutputFormat::Toon => {
            if chain.is_empty() {
                println!("[LINEAGE:{memory_id}:empty]");
            } else {
                println!("[LINEAGE:{memory_id} events={}]", chain.len());
                for event in &chain {
                    let parent = event.parent_id.as_deref().unwrap_or("—");
                    println!(
                        "[EVT id={} action={} parent={}]{}",
                        event.id,
                        event.action,
                        parent,
                        event
                            .rationale
                            .as_deref()
                            .map(|r| format!(" {r}"))
                            .unwrap_or_default(),
                    );
                }
            }
        }
        OutputFormat::Text => {
            if chain.is_empty() {
                println!("No lineage events found for memory '{memory_id}'.");
            } else {
                println!(
                    "Lineage for memory '{memory_id}' ({} events):\n",
                    chain.len()
                );
                for (i, event) in chain.iter().enumerate() {
                    let parent = event.parent_id.as_deref().unwrap_or("(root)");
                    println!(
                        "{}. [{}] action={} parent={}",
                        i + 1,
                        event.created_at,
                        event.action,
                        parent,
                    );
                    if let Some(ref r) = event.rationale {
                        println!("   Rationale: {r}");
                    }
                    println!("   Event ID : {}", event.id);
                    println!();
                }
            }
        }
    }

    Ok(())
}
