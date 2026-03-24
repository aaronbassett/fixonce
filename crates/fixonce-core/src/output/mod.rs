//! Output formatters for memory types.
//!
//! Three formats are available:
//!
//! - [`text`] — human-readable, multi-line plain text
//! - [`json`] — pretty-printed JSON (pass-through for `serde_json`)
//! - [`toon`] — TOON (Token-Optimised Output Notation), compact key-value pairs
//!   for LLM context injection

pub mod json;
pub mod text;
pub mod toon;
