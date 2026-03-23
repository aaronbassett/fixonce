/// Ed25519 key generation and secure local storage.
///
/// Private keys are stored exclusively in the OS keyring.  They are **never**
/// written to disk in plain-text form.
use ed25519_dalek::{SigningKey, VerifyingKey};
use keyring::Entry;
use rand_core::OsRng;

use super::AuthError;

/// The keyring service name used for all fixonce key material.
const KEYRING_SERVICE: &str = "fixonce";

/// Generate a fresh Ed25519 keypair.
///
/// # Errors
///
/// Returns [`AuthError::KeyGenFailed`] if the system CSPRNG is unavailable.
pub fn generate_keypair() -> Result<(SigningKey, VerifyingKey), AuthError> {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();
    Ok((signing_key, verifying_key))
}

/// Store `signing_key` in the OS keyring under `label`.
///
/// The 32-byte raw seed is encoded as hex before being handed to the keyring so
/// that it remains printable and survives keyring implementations that only
/// accept UTF-8.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if the OS keyring rejects the entry.
pub fn store_keypair(signing_key: &SigningKey, label: &str) -> Result<(), AuthError> {
    let seed_hex = hex::encode(signing_key.to_bytes());

    let entry = Entry::new(KEYRING_SERVICE, label)
        .map_err(|e| AuthError::KeyringError(format!("cannot create keyring entry: {e}")))?;

    entry
        .set_password(&seed_hex)
        .map_err(|e| AuthError::KeyringError(format!("cannot store key in keyring: {e}")))?;

    Ok(())
}

/// Load a previously stored [`SigningKey`] from the OS keyring.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if the key is not found or the stored
/// data is corrupt.
pub fn load_keypair(label: &str) -> Result<SigningKey, AuthError> {
    let entry = Entry::new(KEYRING_SERVICE, label)
        .map_err(|e| AuthError::KeyringError(format!("cannot create keyring entry: {e}")))?;

    let seed_hex = entry
        .get_password()
        .map_err(|e| AuthError::KeyringError(format!("key '{label}' not found in keyring: {e}")))?;

    let bytes = hex::decode(&seed_hex).map_err(|e| {
        AuthError::KeyringError(format!("stored key for '{label}' is corrupt: {e}"))
    })?;

    let array: [u8; 32] = bytes.try_into().map_err(|_| {
        AuthError::KeyringError(format!(
            "stored key for '{label}' has wrong length (expected 32 bytes)"
        ))
    })?;

    Ok(SigningKey::from_bytes(&array))
}

/// Remove a stored keypair from the OS keyring.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if deletion fails.
pub fn delete_keypair(label: &str) -> Result<(), AuthError> {
    let entry = Entry::new(KEYRING_SERVICE, label)
        .map_err(|e| AuthError::KeyringError(format!("cannot create keyring entry: {e}")))?;

    entry
        .delete_credential()
        .map_err(|e| AuthError::KeyringError(format!("cannot delete key '{label}': {e}")))?;

    Ok(())
}
