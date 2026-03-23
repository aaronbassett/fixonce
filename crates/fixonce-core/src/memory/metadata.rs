//! Midnight-specific version metadata extracted from source files.

use serde::{Deserialize, Serialize};

/// Version metadata captured from Midnight/Compact source files.
///
/// All fields are optional because they may not be present in every source.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VersionMetadata {
    /// Compact pragma string (e.g. `">=0.4.0"`).
    pub compact_pragma: Option<String>,
    /// Compact compiler version used to compile the contract.
    pub compact_compiler: Option<String>,
    /// Version of the `@midnight-ntwrk/midnight-js-*` packages.
    pub midnight_js: Option<String>,
    /// Compact indexer version.
    pub indexer_version: Option<String>,
    /// Node.js runtime version in use.
    pub node_version: Option<String>,
}
