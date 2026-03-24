/// `fixonce auth` — re-authenticate this machine using its registered keypair.
use anyhow::{Context, Result};
use fixonce_core::auth::{challenge::challenge_auth, keypair::load_keypair, token::TokenManager};

/// The label used for the machine's primary signing key.
const DEFAULT_KEY_LABEL: &str = "machine";

/// Load the machine's Ed25519 signing key, perform a challenge-response
/// authentication against the backend, and store the resulting JWT.
///
/// # Errors
///
/// Returns an error when the keypair is not found, the challenge fails, or the
/// token cannot be stored.
pub async fn run_auth(base_url: &str) -> Result<()> {
    let signing_key = load_keypair(DEFAULT_KEY_LABEL)
        .context("No signing key found — run `fixonce keys add` to register this machine")?;

    let jwt = challenge_auth(base_url, &signing_key)
        .await
        .context("Challenge-response authentication failed")?;

    let mgr = TokenManager::new();
    mgr.store_token(&jwt)
        .context("Failed to store authentication token")?;

    println!("Machine authenticated successfully.");
    Ok(())
}
