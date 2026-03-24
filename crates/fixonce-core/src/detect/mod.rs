//! Environment detection and project context gathering.
//!
//! This module provides two main capabilities:
//!
//! 1. **Midnight ecosystem version detection** — scans `package.json`,
//!    `.compact` source files, and related config files to identify which
//!    versions of the Compact compiler, midnight-js SDK, indexer, and Node.js
//!    are in use.
//!
//! 2. **Project context gathering** — combines version detection with git
//!    metadata (branch, remote, recent commits) and a top-level file-structure
//!    snapshot into a single [`context::ProjectContext`] value.
//!
//! # Quick start
//!
//! ```no_run
//! use std::path::Path;
//! use fixonce_core::detect::{context::gather_context, midnight::detect_midnight_versions};
//!
//! let versions = detect_midnight_versions(Path::new("."));
//! let ctx = gather_context(Path::new("."));
//! ```

pub mod context;
pub mod midnight;

pub use context::{gather_context, ProjectContext};
pub use midnight::{detect_midnight_versions, MidnightVersions};
