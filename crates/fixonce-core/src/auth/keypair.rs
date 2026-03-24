/// Ed25519 key generation and secure local file storage.
///
/// Private keys are stored in `~/.config/fixonce/keys/` with mode 0600,
/// hex-encoded. They are never written to stdout or logs.
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use std::fs;
use std::path::PathBuf;

use super::AuthError;

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

/// Store `signing_key` to a file under `label`.
///
/// The 32-byte raw seed is encoded as hex.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if the file cannot be written.
pub fn store_keypair(signing_key: &SigningKey, label: &str) -> Result<(), AuthError> {
    let path = key_path(label)?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| AuthError::KeyringError(format!("cannot create keys directory: {e}")))?;
    }

    let seed_hex = hex::encode(signing_key.to_bytes());
    fs::write(&path, &seed_hex)
        .map_err(|e| AuthError::KeyringError(format!("cannot store key: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(&path, perms).map_err(|e| {
            AuthError::KeyringError(format!("cannot set key file permissions: {e}"))
        })?;
    }

    Ok(())
}

/// Load a previously stored [`SigningKey`] from the keys directory.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if the key is not found or the stored
/// data is corrupt.
pub fn load_keypair(label: &str) -> Result<SigningKey, AuthError> {
    let path = key_path(label)?;

    let seed_hex = fs::read_to_string(&path)
        .map_err(|e| AuthError::KeyringError(format!("key '{label}' not found: {e}")))?;

    let bytes = hex::decode(seed_hex.trim()).map_err(|e| {
        AuthError::KeyringError(format!("stored key for '{label}' is corrupt: {e}"))
    })?;

    let array: [u8; 32] = bytes.try_into().map_err(|_| {
        AuthError::KeyringError(format!(
            "stored key for '{label}' has wrong length (expected 32 bytes)"
        ))
    })?;

    Ok(SigningKey::from_bytes(&array))
}

/// Remove a stored keypair file.
///
/// # Errors
///
/// Returns [`AuthError::KeyringError`] if deletion fails.
pub fn delete_keypair(label: &str) -> Result<(), AuthError> {
    let path = key_path(label)?;
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|e| AuthError::KeyringError(format!("cannot delete key '{label}': {e}")))?;
    }
    Ok(())
}

/// Resolve the file path for a given key label.
fn key_path(label: &str) -> Result<PathBuf, AuthError> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| AuthError::KeyringError("cannot determine config directory".to_owned()))?;
    // Sanitise label to prevent path traversal.
    let safe_label = label.replace(['/', '\\'], "_").replace("..", "_");
    Ok(config_dir.join("fixonce").join("keys").join(safe_label))
}
