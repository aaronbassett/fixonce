//! Claude Code lifecycle hooks for the `FixOnce` memory system.
//!
//! Each hook is designed to run in under 3 seconds and must never block
//! the Claude Code agent — shell wrappers always exit 0.
//!
//! # Edge cases
//!
//! - **EC-41** — 3-second timeout enforced in shell scripts via `timeout(1)`.
//! - **EC-42** — Shell scripts check for the `fixonce` binary before calling.
//! - **EC-43** — [`HookError::Unauthenticated`] is handled silently; hooks skip
//!   gracefully when no token is stored.

pub mod post_tool_use;
pub mod pre_tool_use;
pub mod session_start;
pub mod stop;
pub mod user_prompt;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur in any hook handler.
#[derive(Debug, thiserror::Error)]
pub enum HookError {
    /// An API call failed.
    #[error("API error: {0}")]
    Api(#[from] fixonce_core::api::ApiError),

    /// The authentication subsystem failed.
    #[error("Auth error: {0}")]
    Auth(#[from] fixonce_core::auth::AuthError),

    /// The hook took longer than the permitted budget.
    #[error("Hook timed out after {0}s")]
    Timeout(u64),

    /// No token was found in the keyring — skip silently (EC-43).
    #[error("Not authenticated — skipping hook")]
    Unauthenticated,
}
