//! Midnight ecosystem component version detection.
//!
//! Scans common project files to identify version strings for:
//!
//! - **`compact_pragma`** — `pragma compiler` declaration in `.compact` files
//! - **`compact_compiler`** — Compact compiler version (package.json / .compactrc)
//! - **`midnight_js`** — `@midnight-ntwrk/midnight-js-*` SDK version in package.json
//! - **`indexer_version`** — Midnight indexer version from package.json
//! - **`node_version`** — Node.js engine requirement from package.json or `.node-version`

use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Detected versions for components of the Midnight ecosystem.
///
/// Each field is `None` when the corresponding component was not found in the
/// scanned project files.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MidnightVersions {
    /// `pragma compiler` version from a `.compact` source file.
    pub compact_pragma: Option<String>,
    /// Compact compiler npm package version.
    pub compact_compiler: Option<String>,
    /// `@midnight-ntwrk/midnight-js-*` SDK version from `package.json`.
    pub midnight_js: Option<String>,
    /// Midnight indexer version from `package.json`.
    pub indexer_version: Option<String>,
    /// Node.js engine or runtime version requirement.
    pub node_version: Option<String>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Read a file, returning `None` on any I/O error.
fn read_file_opt(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Search `directory` (non-recursively, depth 1) for the first `*.compact`
/// file and extract the `pragma compiler` version from it.
fn detect_compact_pragma(project_root: &Path) -> Option<String> {
    let re = Regex::new(r"pragma\s+compiler\s+[>=<~^]*\s*([0-9][^\s;]*)").ok()?;

    // Walk only the project root first; also check a `src` sub-directory.
    for dir in [project_root, &project_root.join("src")] {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("compact") {
                    if let Some(content) = read_file_opt(&path) {
                        if let Some(cap) = re.captures(&content) {
                            return Some(cap[1].trim().to_owned());
                        }
                    }
                }
            }
        }
    }

    // Fallback: shallow recursive search (up to 3 levels)
    detect_compact_pragma_recursive(project_root, 0)
}

fn detect_compact_pragma_recursive(dir: &Path, depth: u8) -> Option<String> {
    if depth > 3 {
        return None;
    }
    let re = Regex::new(r"pragma\s+compiler\s+[>=<~^]*\s*([0-9][^\s;]*)").ok()?;

    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip hidden directories and node_modules / target.
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            if let Some(v) = detect_compact_pragma_recursive(&path, depth + 1) {
                return Some(v);
            }
        } else if path.extension().and_then(|e| e.to_str()) == Some("compact") {
            if let Some(content) = read_file_opt(&path) {
                if let Some(cap) = re.captures(&content) {
                    return Some(cap[1].trim().to_owned());
                }
            }
        }
    }
    None
}

/// Extract version information from `package.json` in `project_root`.
///
/// Returns a tuple of `(compact_compiler, midnight_js, indexer_version,
/// node_version)`.
fn detect_from_package_json(
    project_root: &Path,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let path = project_root.join("package.json");
    let Some(content) = read_file_opt(&path) else {
        return (None, None, None, None);
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (None, None, None, None),
    };

    let compact_compiler = extract_dep_version(&json, "@midnight-ntwrk/compact-compiler")
        .or_else(|| extract_dep_version(&json, "compact-compiler"));

    // midnight-js: look for any @midnight-ntwrk/midnight-js-* package.
    let midnight_js = extract_midnight_js_version(&json);

    // indexer: @midnight-ntwrk/midnight-js-indexer or similar.
    let indexer_version = extract_dep_version(&json, "@midnight-ntwrk/midnight-js-indexer")
        .or_else(|| extract_dep_version(&json, "@midnight-ntwrk/indexer"));

    // Node version from `engines.node`.
    let node_version = json
        .get("engines")
        .and_then(|e| e.get("node"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('^').trim_start_matches('~').to_owned());

    (compact_compiler, midnight_js, indexer_version, node_version)
}

/// Extract the version of `package_name` from `dependencies`,
/// `devDependencies`, or `peerDependencies`.
fn extract_dep_version(json: &serde_json::Value, package_name: &str) -> Option<String> {
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(v) = json.get(section).and_then(|d| d.get(package_name)) {
            if let Some(s) = v.as_str() {
                // Strip leading semver range characters.
                let cleaned = s
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches(">=")
                    .trim_start_matches('>')
                    .to_owned();
                return Some(cleaned);
            }
        }
    }
    None
}

/// Find any `@midnight-ntwrk/midnight-js-*` entry and return its version.
fn extract_midnight_js_version(json: &serde_json::Value) -> Option<String> {
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(deps) = json.get(section).and_then(|d| d.as_object()) {
            for (key, val) in deps {
                if key.starts_with("@midnight-ntwrk/midnight-js") {
                    if let Some(s) = val.as_str() {
                        let cleaned = s
                            .trim_start_matches('^')
                            .trim_start_matches('~')
                            .trim_start_matches(">=")
                            .trim_start_matches('>')
                            .to_owned();
                        return Some(cleaned);
                    }
                }
            }
        }
    }
    None
}

/// Check `.node-version` or `.nvmrc` for a Node.js version override.
fn detect_node_version_file(project_root: &Path) -> Option<String> {
    for name in [".node-version", ".nvmrc"] {
        if let Some(content) = read_file_opt(&project_root.join(name)) {
            let trimmed = content.trim().trim_start_matches('v').to_owned();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

/// Check `tsconfig.json` for `compilerOptions.target` or any Compact-specific
/// fields.
fn detect_from_tsconfig(project_root: &Path) -> Option<String> {
    let path = project_root.join("tsconfig.json");
    let content = read_file_opt(&path)?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    // Some Compact toolchains embed the compiler version in tsconfig references.
    json.get("compactCompilerVersion")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Scan `project_root` for Midnight ecosystem component versions.
///
/// The scan is best-effort and non-recursive beyond three directory levels.
/// Missing files are silently skipped; parse failures fall back to `None`.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use fixonce_core::detect::midnight::detect_midnight_versions;
///
/// let versions = detect_midnight_versions(Path::new("."));
/// println!("{:?}", versions.midnight_js);
/// ```
#[must_use]
pub fn detect_midnight_versions(project_root: &Path) -> MidnightVersions {
    // Scan package.json.
    let (pkg_compact_compiler, pkg_midnight_js, pkg_indexer, pkg_node) =
        detect_from_package_json(project_root);

    // Compact pragma from .compact files.
    let compact_pragma = detect_compact_pragma(project_root);

    // Compact compiler from tsconfig (may override package.json).
    let tsconfig_compiler = detect_from_tsconfig(project_root);
    let compact_compiler = tsconfig_compiler.or(pkg_compact_compiler);

    // Node version: prefer .node-version / .nvmrc over package.json engines.
    let node_version = detect_node_version_file(project_root).or(pkg_node);

    MidnightVersions {
        compact_pragma,
        compact_compiler,
        midnight_js: pkg_midnight_js,
        indexer_version: pkg_indexer,
        node_version,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // --- detect_midnight_versions: empty project ---

    #[test]
    fn empty_project_returns_all_none() {
        let dir = tmp();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v, MidnightVersions::default());
    }

    // --- package.json parsing ---

    #[test]
    fn detects_midnight_js_from_package_json() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "dependencies": {
    "@midnight-ntwrk/midnight-js-contracts": "^1.2.3"
  }
}"#,
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.midnight_js.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn detects_compact_compiler_from_package_json() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "devDependencies": {
    "@midnight-ntwrk/compact-compiler": "0.15.0"
  }
}"#,
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.compact_compiler.as_deref(), Some("0.15.0"));
    }

    #[test]
    fn detects_indexer_from_package_json() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "dependencies": {
    "@midnight-ntwrk/midnight-js-indexer": "~2.0.1"
  }
}"#,
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.indexer_version.as_deref(), Some("2.0.1"));
    }

    #[test]
    fn detects_node_from_engines_field() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "engines": { "node": "^20.0.0" }
}"#,
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.node_version.as_deref(), Some("20.0.0"));
    }

    // --- .node-version / .nvmrc ---

    #[test]
    fn node_version_file_overrides_engines() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{"engines":{"node":"^18.0.0"}}"#,
        )
        .unwrap();
        fs::write(dir.path().join(".node-version"), "20.11.0\n").unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.node_version.as_deref(), Some("20.11.0"));
    }

    #[test]
    fn nvmrc_strips_leading_v() {
        let dir = tmp();
        fs::write(dir.path().join(".nvmrc"), "v20.5.1\n").unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.node_version.as_deref(), Some("20.5.1"));
    }

    // --- .compact pragma ---

    #[test]
    fn detects_pragma_in_root_compact_file() {
        let dir = tmp();
        fs::write(
            dir.path().join("contract.compact"),
            "pragma compiler >= 0.14;\n\ncontract Foo {}",
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        // The regex stops at `;` so the captured version must not include it.
        assert_eq!(v.compact_pragma.as_deref(), Some("0.14"));
    }

    #[test]
    fn detects_pragma_in_src_subdirectory() {
        let dir = tmp();
        let src = dir.path().join("src");
        fs::create_dir_all(&src).unwrap();
        fs::write(
            src.join("main.compact"),
            "pragma compiler 0.15.2;\ncontract Bar {}",
        )
        .unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.compact_pragma.as_deref(), Some("0.15.2"));
    }

    // --- Invalid JSON falls back gracefully ---

    #[test]
    fn invalid_package_json_returns_none() {
        let dir = tmp();
        fs::write(dir.path().join("package.json"), "not json at all").unwrap();
        let v = detect_midnight_versions(dir.path());
        assert_eq!(v, MidnightVersions::default());
    }

    // --- Full project layout ---

    #[test]
    fn full_project_detects_all_fields() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{
  "dependencies": {
    "@midnight-ntwrk/midnight-js-contracts": "^1.0.0",
    "@midnight-ntwrk/midnight-js-indexer": "1.2.3"
  },
  "devDependencies": {
    "@midnight-ntwrk/compact-compiler": "0.15.0"
  },
  "engines": { "node": "^20.0.0" }
}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("contract.compact"),
            "pragma compiler 0.15;\ncontract X {}",
        )
        .unwrap();
        fs::write(dir.path().join(".node-version"), "20.11.1").unwrap();

        let v = detect_midnight_versions(dir.path());
        assert_eq!(v.midnight_js.as_deref(), Some("1.0.0"));
        assert_eq!(v.compact_compiler.as_deref(), Some("0.15.0"));
        assert_eq!(v.indexer_version.as_deref(), Some("1.2.3"));
        assert_eq!(v.node_version.as_deref(), Some("20.11.1"));
        assert!(v.compact_pragma.is_some());
    }
}
