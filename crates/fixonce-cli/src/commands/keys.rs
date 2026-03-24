/// `fixonce keys` — manage Ed25519 signing keys.
use anyhow::{Context, Result};
use fixonce_core::{
    api::ApiClient,
    auth::{
        keypair::{generate_keypair, store_keypair},
        token::TokenManager,
    },
};

/// The label used for the machine's primary signing key.
const DEFAULT_KEY_LABEL: &str = "machine";

/// Generate a new Ed25519 keypair, store the private key locally,
/// and register the public key with the backend.
///
/// # Errors
///
/// Returns an error when key generation fails, local storage rejects the write,
/// or the backend registration call fails.
pub async fn run_keys_add(base_url: &str) -> Result<()> {
    let (signing_key, verifying_key) =
        generate_keypair().context("Failed to generate Ed25519 keypair")?;

    store_keypair(&signing_key, DEFAULT_KEY_LABEL).context("Failed to store private key")?;

    // Load the current JWT so we can authenticate the registration call.
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(base_url)
        .context("Failed to create API client")?
        .with_token(token);

    let pub_key_hex = hex::encode(verifying_key.to_bytes());
    let body = serde_json::json!({ "public_key": pub_key_hex });

    client
        .post_authenticated("/functions/v1/keys-add")?
        .json(&body)
        .send()
        .await
        .context("Failed to register public key with backend")?
        .error_for_status()
        .context("Backend rejected public key registration")?;

    println!("Signing key added. Public key: {pub_key_hex}");
    Ok(())
}

/// List all signing keys registered for the current user.
///
/// # Errors
///
/// Returns an error when the backend call fails.
pub async fn run_keys_list(base_url: &str) -> Result<()> {
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(base_url)
        .context("Failed to create API client")?
        .with_token(token);

    let response = client
        .get_authenticated("/functions/v1/keys-list")?
        .send()
        .await
        .context("Failed to list signing keys")?
        .error_for_status()
        .context("Backend returned an error for keys-list")?;

    let keys: serde_json::Value = response
        .json()
        .await
        .context("Invalid response from keys-list")?;

    if let Some(arr) = keys.as_array() {
        if arr.is_empty() {
            println!("No signing keys registered.");
        } else {
            for (i, key) in arr.iter().enumerate() {
                let id = key.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                let pub_key = key
                    .get("public_key")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let created = key
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                println!("[{}] id={id}  key={pub_key}  created={created}", i + 1);
            }
        }
    } else {
        println!("{keys}");
    }

    Ok(())
}

/// Revoke a registered signing key by its ID.
///
/// Sends the revocation to the backend.  Local keypair cleanup is a future
/// enhancement that requires a server-side lookup to match `key_id` to the
/// stored public key.
///
/// # Errors
///
/// Returns an error when the backend call fails.
pub async fn run_keys_revoke(base_url: &str, key_id: &str) -> Result<()> {
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(base_url)
        .context("Failed to create API client")?
        .with_token(token);

    let body = serde_json::json!({ "key_id": key_id });

    client
        .post_authenticated("/functions/v1/keys-revoke")?
        .json(&body)
        .send()
        .await
        .context("Failed to revoke signing key")?
        .error_for_status()
        .context("Backend rejected key revocation")?;

    println!("Key {key_id} revoked.");
    Ok(())
}
