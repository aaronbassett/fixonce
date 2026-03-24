//! T302 — Authentication flow end-to-end integration tests.
//!
//! Tests token management (expiry, parse), keypair generation, public key
//! extraction, base64 encoding, and nonce signing + local signature
//! verification.  No network calls are made.

use base64::Engine as _;
use ed25519_dalek::{Signer, Verifier};
use fixonce_core::auth::token::TokenManager;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal fake JWT with the given `exp` claim.
///
/// The signature segment is a placeholder; only the payload is used by
/// `TokenManager::is_expired`.
fn fake_jwt(exp: Option<u64>) -> String {
    let header =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"EdDSA","typ":"JWT"}"#);
    let payload_json = match exp {
        Some(e) => format!(r#"{{"sub":"test-user","iss":"fixonce","exp":{e}}}"#),
        None => r#"{"sub":"test-user","iss":"fixonce"}"#.to_owned(),
    };
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json.as_bytes());
    format!("{header}.{payload}.fake-signature-segment")
}

// ---------------------------------------------------------------------------
// T302-a: Token management — expiry checking
// ---------------------------------------------------------------------------

#[test]
fn expired_jwt_is_detected() {
    let mgr = TokenManager::new();
    // exp = 1 is 1970-01-01, always in the past
    assert!(
        mgr.is_expired(&fake_jwt(Some(1))),
        "token with past exp must be treated as expired"
    );
}

#[test]
fn future_jwt_is_not_expired() {
    let mgr = TokenManager::new();
    // exp far in the future (year ~2527)
    assert!(
        !mgr.is_expired(&fake_jwt(Some(u64::MAX / 2))),
        "token with future exp must not be expired"
    );
}

#[test]
fn jwt_without_exp_claim_is_not_expired() {
    let mgr = TokenManager::new();
    // No `exp` field — treated as a non-expiring API key
    assert!(
        !mgr.is_expired(&fake_jwt(None)),
        "token without exp claim must not be expired"
    );
}

#[test]
fn malformed_jwt_is_treated_as_expired() {
    let mgr = TokenManager::new();
    // Not a valid JWT structure at all
    assert!(
        mgr.is_expired("not-a-jwt"),
        "malformed token must be treated as expired (fail-safe)"
    );
}

#[test]
fn jwt_with_only_one_segment_is_expired() {
    let mgr = TokenManager::new();
    assert!(
        mgr.is_expired("onlyone"),
        "single-segment string is not a JWT, must be treated as expired"
    );
}

#[test]
fn jwt_with_corrupt_payload_base64_is_expired() {
    let mgr = TokenManager::new();
    let bad = "eyJhbGciOiJFZERTQSJ9.NOT_VALID_BASE64!!!.sig";
    assert!(
        mgr.is_expired(bad),
        "JWT with corrupt base64 payload must be treated as expired"
    );
}

#[test]
fn jwt_with_valid_base64_but_non_json_payload_is_expired() {
    let mgr = TokenManager::new();
    let garbage_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("not-json");
    let token = format!("header.{garbage_payload}.sig");
    assert!(
        mgr.is_expired(&token),
        "JWT with non-JSON payload must be treated as expired"
    );
}

// ---------------------------------------------------------------------------
// T302-b: Keypair generation → public key extraction → base64 encoding
// ---------------------------------------------------------------------------

#[test]
fn generate_keypair_produces_valid_signing_key() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (signing_key, verifying_key) = generate_keypair().expect("keypair generation must succeed");

    // Derived verifying key must match.
    let derived = signing_key.verifying_key();
    assert_eq!(
        derived.to_bytes(),
        verifying_key.to_bytes(),
        "verifying key from signing key must match returned verifying key"
    );
}

#[test]
fn verifying_key_encodes_to_32_byte_base64() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (_, verifying_key) = generate_keypair().expect("keypair generation must succeed");

    let encoded = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());
    // Base64 of 32 bytes = ceil(32/3)*4 = 44 characters
    assert_eq!(
        encoded.len(),
        44,
        "base64-encoded Ed25519 public key must be 44 chars"
    );
}

#[test]
fn public_key_base64_decodes_back_to_original() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (_, verifying_key) = generate_keypair().expect("keypair generation must succeed");
    let original_bytes = verifying_key.to_bytes();

    let encoded = base64::engine::general_purpose::STANDARD.encode(original_bytes);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(&encoded)
        .expect("must decode");

    assert_eq!(
        decoded.as_slice(),
        original_bytes.as_ref(),
        "public key must survive base64 round-trip"
    );
}

#[test]
fn two_generated_keypairs_are_distinct() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (_, vk1) = generate_keypair().expect("first keypair");
    let (_, vk2) = generate_keypair().expect("second keypair");

    assert_ne!(
        vk1.to_bytes(),
        vk2.to_bytes(),
        "distinct keypairs must produce distinct public keys"
    );
}

// ---------------------------------------------------------------------------
// T302-c: Nonce signing → local signature verification
// ---------------------------------------------------------------------------

#[test]
fn sign_nonce_and_verify_locally() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (signing_key, verifying_key) = generate_keypair().expect("keypair generation must succeed");

    let nonce = "fixonce-challenge-nonce-abc123";
    let signature = signing_key.sign(nonce.as_bytes());

    // Verify locally — should succeed.
    assert!(
        verifying_key.verify(nonce.as_bytes(), &signature).is_ok(),
        "signature produced by signing_key must verify with verifying_key"
    );
}

#[test]
fn signature_verification_fails_for_tampered_nonce() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (signing_key, verifying_key) = generate_keypair().expect("keypair generation must succeed");

    let nonce = "original-nonce";
    let signature = signing_key.sign(nonce.as_bytes());

    let tampered = "tampered-nonce";
    assert!(
        verifying_key
            .verify(tampered.as_bytes(), &signature)
            .is_err(),
        "signature must not verify against a different message"
    );
}

#[test]
fn signature_verification_fails_with_wrong_key() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (signing_key, _) = generate_keypair().expect("first keypair");
    let (_, other_verifying_key) = generate_keypair().expect("second keypair");

    let nonce = "shared-nonce-value";
    let signature = signing_key.sign(nonce.as_bytes());

    assert!(
        other_verifying_key
            .verify(nonce.as_bytes(), &signature)
            .is_err(),
        "signature from key A must not verify with key B"
    );
}

#[test]
fn signature_base64_encodes_to_88_chars() {
    use fixonce_core::auth::keypair::generate_keypair;

    let (signing_key, _) = generate_keypair().expect("keypair generation must succeed");
    let nonce = "test-nonce-for-encoding";
    let signature = signing_key.sign(nonce.as_bytes());

    let encoded = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    // Ed25519 signature = 64 bytes → base64 = ceil(64/3)*4 = 88 chars
    assert_eq!(
        encoded.len(),
        88,
        "base64-encoded Ed25519 signature must be 88 chars"
    );
}

#[test]
fn challenge_flow_end_to_end_local_simulation() {
    // Simulate the full challenge-response flow locally:
    // 1. Server sends nonce.
    // 2. Client signs nonce.
    // 3. Server verifies with registered public key.
    use fixonce_core::auth::keypair::generate_keypair;

    // Setup: client has a keypair, server has the public key registered.
    let (signing_key, verifying_key) = generate_keypair().expect("keypair generation must succeed");

    // Step 1: server sends nonce.
    let server_nonce = "server-generated-nonce-0f1a2b3c4d";

    // Step 2: client signs nonce and encodes both sig and pubkey as base64.
    let signature = signing_key.sign(server_nonce.as_bytes());
    let sig_b64 = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    let pub_b64 = base64::engine::general_purpose::STANDARD.encode(verifying_key.to_bytes());

    // Step 3: server decodes the public key and verifies the signature.
    let decoded_pub = base64::engine::general_purpose::STANDARD
        .decode(&pub_b64)
        .expect("pub decode");
    let decoded_sig = base64::engine::general_purpose::STANDARD
        .decode(&sig_b64)
        .expect("sig decode");

    let pub_array: [u8; 32] = decoded_pub.try_into().expect("32 bytes");
    let sig_array: [u8; 64] = decoded_sig.try_into().expect("64 bytes");

    let server_vk = ed25519_dalek::VerifyingKey::from_bytes(&pub_array).expect("parse pub key");
    let server_sig = ed25519_dalek::Signature::from_bytes(&sig_array);

    assert!(
        server_vk
            .verify(server_nonce.as_bytes(), &server_sig)
            .is_ok(),
        "server must successfully verify client signature"
    );
}
