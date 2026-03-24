//! T305 — Environment detection end-to-end integration tests.
//!
//! Tests environment detection on a mock project directory and context
//! gathering with git info.  Uses `tempfile` for filesystem isolation.

use std::fs;
use tempfile::TempDir;

use fixonce_core::detect::{
    context::gather_context,
    midnight::{detect_midnight_versions, MidnightVersions},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tmp() -> TempDir {
    tempfile::tempdir().expect("tempdir creation must succeed")
}

macro_rules! git {
    ($dir:expr, $($arg:expr),+) => {
        std::process::Command::new("git")
            .args([$($arg),+])
            .current_dir($dir)
            .env("GIT_AUTHOR_NAME", "Integration Test")
            .env("GIT_AUTHOR_EMAIL", "test@fixonce.dev")
            .env("GIT_COMMITTER_NAME", "Integration Test")
            .env("GIT_COMMITTER_EMAIL", "test@fixonce.dev")
            .output()
    };
}

/// Returns `true` if `git` is available in PATH.
fn git_available() -> bool {
    std::process::Command::new("git")
        .arg("--version")
        .output()
        .is_ok()
}

// ---------------------------------------------------------------------------
// T305-a: Environment detection on mock project directory
// ---------------------------------------------------------------------------

#[test]
fn empty_project_produces_all_none_versions() {
    let dir = tmp();
    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions,
        MidnightVersions::default(),
        "empty project must return all-None versions"
    );
}

#[test]
fn detects_midnight_js_version_from_package_json() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@midnight-ntwrk/midnight-js-contracts":"^2.1.0"}}"#,
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.midnight_js.as_deref(),
        Some("2.1.0"),
        "must detect midnight-js version from package.json"
    );
}

#[test]
fn detects_compact_compiler_from_dev_dependencies() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"devDependencies":{"@midnight-ntwrk/compact-compiler":"0.16.2"}}"#,
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.compact_compiler.as_deref(),
        Some("0.16.2"),
        "must detect compact-compiler from devDependencies"
    );
}

#[test]
fn detects_node_version_from_engines_field() {
    let dir = tmp();
    // The implementation strips ^ and ~ prefixes from engines.node.
    fs::write(
        dir.path().join("package.json"),
        r#"{"engines":{"node":"^20.0.0"}}"#,
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.node_version.as_deref(),
        Some("20.0.0"),
        "must detect node version stripping ^ prefix"
    );
}

#[test]
fn node_version_file_overrides_package_json_engines() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"engines":{"node":"^18.0.0"}}"#,
    )
    .unwrap();
    fs::write(dir.path().join(".node-version"), "20.11.0\n").unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.node_version.as_deref(),
        Some("20.11.0"),
        ".node-version file must override package.json engines"
    );
}

#[test]
fn nvmrc_strips_leading_v_prefix() {
    let dir = tmp();
    fs::write(dir.path().join(".nvmrc"), "v22.1.0\n").unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.node_version.as_deref(),
        Some("22.1.0"),
        ".nvmrc must strip leading 'v'"
    );
}

#[test]
fn detects_compact_pragma_from_compact_file_in_root() {
    let dir = tmp();
    fs::write(
        dir.path().join("main.compact"),
        "pragma compiler >= 0.14;\ncontract MyContract { }",
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert!(
        versions.compact_pragma.is_some(),
        "must detect compact pragma from .compact file"
    );
    // The semicolon should not be captured
    let pragma = versions.compact_pragma.unwrap();
    assert!(
        !pragma.contains(';'),
        "captured pragma must not contain semicolon, got: {pragma}"
    );
}

#[test]
fn detects_compact_pragma_from_src_subdirectory() {
    let dir = tmp();
    let src = dir.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("token.compact"),
        "pragma compiler 0.15.1;\ncontract Token {}",
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.compact_pragma.as_deref(),
        Some("0.15.1"),
        "must detect compact pragma from src/ subdirectory"
    );
}

#[test]
fn detects_indexer_version_from_dependencies() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@midnight-ntwrk/midnight-js-indexer":"~3.0.0"}}"#,
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions.indexer_version.as_deref(),
        Some("3.0.0"),
        "must detect indexer version"
    );
}

#[test]
fn full_midnight_project_detects_all_fields() {
    let dir = tmp();

    fs::write(
        dir.path().join("package.json"),
        r#"{
  "dependencies": {
    "@midnight-ntwrk/midnight-js-contracts": "^1.5.0",
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
        "pragma compiler 0.15;\ncontract Foo {}",
    )
    .unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(versions.midnight_js.as_deref(), Some("1.5.0"));
    assert_eq!(versions.compact_compiler.as_deref(), Some("0.15.0"));
    assert_eq!(versions.indexer_version.as_deref(), Some("1.2.3"));
    assert_eq!(versions.node_version.as_deref(), Some("20.0.0"));
    assert!(versions.compact_pragma.is_some());
}

#[test]
fn invalid_json_in_package_json_returns_all_none() {
    let dir = tmp();
    fs::write(dir.path().join("package.json"), "{ invalid json }").unwrap();

    let versions = detect_midnight_versions(dir.path());
    assert_eq!(
        versions,
        MidnightVersions::default(),
        "invalid JSON in package.json must produce all-None versions"
    );
}

// ---------------------------------------------------------------------------
// T305-b: Context gathering
// ---------------------------------------------------------------------------

#[test]
fn gather_context_non_git_dir_has_no_git_info() {
    let dir = tmp();
    let ctx = gather_context(dir.path());

    assert!(ctx.git_remote.is_none(), "non-git dir must have no remote");
    assert!(ctx.git_branch.is_none(), "non-git dir must have no branch");
    assert!(
        ctx.recent_commits.is_empty(),
        "non-git dir must have no recent commits"
    );
}

#[test]
fn gather_context_includes_file_structure() {
    let dir = tmp();
    fs::write(dir.path().join("README.md"), "# Test project").unwrap();
    fs::write(dir.path().join("package.json"), "{}").unwrap();
    fs::create_dir(dir.path().join("src")).unwrap();

    let ctx = gather_context(dir.path());

    assert!(
        ctx.file_structure.contains(&"README.md".to_owned()),
        "file structure must include README.md"
    );
    assert!(
        ctx.file_structure.contains(&"package.json".to_owned()),
        "file structure must include package.json"
    );
    assert!(
        ctx.file_structure.contains(&"src/".to_owned()),
        "file structure must include src/ directory"
    );
}

#[test]
fn gather_context_file_structure_is_sorted() {
    let dir = tmp();
    for name in &["zebra.txt", "alpha.txt", "mango.txt"] {
        fs::write(dir.path().join(name), "").unwrap();
    }

    let ctx = gather_context(dir.path());
    let mut sorted = ctx.file_structure.clone();
    sorted.sort();
    assert_eq!(ctx.file_structure, sorted, "file structure must be sorted");
}

#[test]
fn gather_context_excludes_target_directory() {
    let dir = tmp();
    fs::create_dir(dir.path().join("target")).unwrap();
    fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

    let ctx = gather_context(dir.path());
    assert!(
        !ctx.file_structure
            .iter()
            .any(|s| s == "target" || s == "target/"),
        "file structure must exclude target directory"
    );
}

#[test]
fn gather_context_picks_up_midnight_js_version() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@midnight-ntwrk/midnight-js-contracts":"^1.0.0"}}"#,
    )
    .unwrap();

    let ctx = gather_context(dir.path());
    assert_eq!(
        ctx.versions.midnight_js.as_deref(),
        Some("1.0.0"),
        "context must contain detected midnight-js version"
    );
}

#[test]
fn gather_context_with_git_branch_after_init_and_commit() {
    if !git_available() {
        return; // skip if git is not in PATH
    }

    let dir = tmp();

    if git!(dir.path(), "init", "-b", "integration-test-branch").is_err() {
        return;
    }

    fs::write(dir.path().join("init.txt"), "init").unwrap();
    let _ = git!(dir.path(), "add", ".");
    let _ = git!(dir.path(), "commit", "-m", "initial commit");

    let ctx = gather_context(dir.path());
    assert_eq!(
        ctx.git_branch.as_deref(),
        Some("integration-test-branch"),
        "context must report the current git branch"
    );
    assert!(
        ctx.git_remote.is_none(),
        "local-only git repo must have no remote"
    );
}

#[test]
fn gather_context_recent_commits_limited_to_five() {
    if !git_available() {
        return;
    }

    let dir = tmp();

    if git!(dir.path(), "init", "-b", "main").is_err() {
        return;
    }

    for i in 0..8u8 {
        fs::write(dir.path().join(format!("file{i}.txt")), i.to_string()).unwrap();
        let _ = git!(dir.path(), "add", ".");
        let _ = git!(dir.path(), "commit", "-m", &format!("commit {i}"));
    }

    let ctx = gather_context(dir.path());
    assert!(
        ctx.recent_commits.len() <= 5,
        "must return at most 5 recent commits, got {}",
        ctx.recent_commits.len()
    );
}

#[test]
fn context_serializes_to_json_successfully() {
    let dir = tmp();
    fs::write(
        dir.path().join("package.json"),
        r#"{"dependencies":{"@midnight-ntwrk/midnight-js-contracts":"^1.2.3"}}"#,
    )
    .unwrap();

    let ctx = gather_context(dir.path());
    let json = serde_json::to_string(&ctx).expect("context must serialize to JSON");
    assert!(
        json.contains("midnight_js"),
        "serialized context must contain midnight_js field"
    );
}
