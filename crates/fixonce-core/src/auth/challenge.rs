/// Challenge-response authentication protocol.
///
/// Exchanges a signed nonce with the backend to obtain a JWT without requiring
/// a full OAuth browser flow.  This is used to re-authenticate a machine that
/// already has a registered Ed25519 keypair.
use base64::Engine as _;
use ed25519_dalek::{Signer, SigningKey};
use serde::Deserialize;

use super::AuthError;

/// Response from the `auth-nonce` edge function.
#[derive(Debug, Deserialize)]
struct NonceResponse {
    nonce: String,
}

/// Response from the `auth-verify` edge function.
#[derive(Debug, Deserialize)]
struct VerifyResponse {
    access_token: String,
}

/// Perform a challenge-response authentication and return a JWT.
///
/// # Flow
///
/// 1. `GET {base_url}/functions/v1/auth-nonce` → receive a random nonce.
/// 2. Sign the nonce bytes with `signing_key`.
/// 3. `POST {base_url}/functions/v1/auth-verify` with the base64-encoded
///    signature and the public key.
/// 4. Return the JWT from the response.
///
/// # Errors
///
/// Returns [`AuthError::ChallengeFailed`] when the server rejects the
/// challenge.  Returns [`AuthError::HttpError`] when the network request
/// fails.
pub async fn challenge_auth(base_url: &str, signing_key: &SigningKey) -> Result<String, AuthError> {
    let client = reqwest::Client::new();

    // Step 1 — Request a nonce.
    let nonce_url = format!("{base_url}/functions/v1/auth-nonce");
    let nonce_resp: NonceResponse = client
        .get(&nonce_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AuthError::ChallengeFailed(format!("nonce request failed: {e}")))?
        .json()
        .await?;

    // Step 2 — Sign the nonce.
    let signature = signing_key.sign(nonce_resp.nonce.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());

    let verifying_key = signing_key.verifying_key();
    let pub_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    // Step 3 — Verify the signature with the backend.
    let verify_url = format!("{base_url}/functions/v1/auth-verify");
    let body = serde_json::json!({
        "nonce": nonce_resp.nonce,
        "signature": sig_b64,
        "public_key": pub_b64,
    });

    let verify_resp: VerifyResponse = client
        .post(&verify_url)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| AuthError::ChallengeFailed(format!("verify request failed: {e}")))?
        .json()
        .await?;

    Ok(verify_resp.access_token)
}
