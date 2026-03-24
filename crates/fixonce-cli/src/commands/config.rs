//! `fixonce config` — show and set CLI configuration.
//!
//! Configuration is read from the `FIXONCE_API_URL` environment variable and
//! (for future use) from a config file in the user's config directory.

use anyhow::Result;
use fixonce_core::auth::token::TokenManager;

// ---------------------------------------------------------------------------
// Config entry point
// ---------------------------------------------------------------------------

/// Execute `fixonce config`.
///
/// Prints the active configuration values to stdout.
///
/// # Errors
///
/// Returns an error if the token store cannot be read.
// Kept as `Result<()>` for API consistency; future callers may add error paths.
#[allow(clippy::unnecessary_wraps)]
pub fn run_config() -> Result<()> {
    let api_url = std::env::var("FIXONCE_API_URL")
        .unwrap_or_else(|_| "https://fixonce.supabase.co".to_owned());

    let mgr = TokenManager::new();
    let token_state = match mgr.load_token() {
        Ok(Some(_)) => "authenticated",
        Ok(None) => "not authenticated",
        Err(_) => "error reading token",
    };

    println!("FixOnce CLI Configuration");
    println!("─────────────────────────");
    println!("  api_url      : {api_url}");
    println!("  auth_status  : {token_state}");
    println!();
    println!("Override the API URL with the FIXONCE_API_URL environment variable.");
    println!("Run `fixonce login` to authenticate.");

    Ok(())
}
