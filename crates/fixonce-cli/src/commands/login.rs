/// `fixonce login` — authenticate with GitHub OAuth.
use anyhow::{Context, Result};
use fixonce_core::auth::{oauth, token::TokenManager};

/// Execute the GitHub OAuth login flow, store the resulting JWT, and print a
/// success message.
///
/// # Errors
///
/// Propagates any error from the OAuth flow or keyring storage.
pub async fn run_login(supabase_url: &str, anon_key: &str) -> Result<()> {
    let jwt = oauth::login_with_github(supabase_url, anon_key)
        .await
        .context("GitHub OAuth login failed")?;

    let mgr = TokenManager::new();
    mgr.store_token(&jwt)
        .context("Failed to store authentication token")?;

    println!("Logged in successfully.");
    Ok(())
}
