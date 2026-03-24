//! Project context gathering: combines environment detection with git metadata
//! and a top-level file structure snapshot.

use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::midnight::{detect_midnight_versions, MidnightVersions};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Full project context: Midnight ecosystem versions, git metadata, and a
/// snapshot of the top-level file structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContext {
    /// Detected Midnight ecosystem versions.
    pub versions: MidnightVersions,
    /// Remote origin URL, or `None` when the repo has no remotes (EC-38).
    pub git_remote: Option<String>,
    /// Current branch name.
    pub git_branch: Option<String>,
    /// Short hashes + subjects for the five most recent commits.
    pub recent_commits: Vec<String>,
    /// Top-level entries in `project_root` (files and directories).
    pub file_structure: Vec<String>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Run a `git` command in `dir` and return trimmed stdout, or `None` on
/// failure.
fn git_output(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .ok()?;

    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    } else {
        None
    }
}

/// Return the `origin` remote URL (EC-38: returns `None` when none exists).
fn detect_git_remote(project_root: &Path) -> Option<String> {
    git_output(project_root, &["remote", "get-url", "origin"])
}

/// Return the current branch name.
fn detect_git_branch(project_root: &Path) -> Option<String> {
    // `git symbolic-ref --short HEAD` works on regular branches.
    git_output(project_root, &["symbolic-ref", "--short", "HEAD"])
        // Fallback for detached HEAD: show abbreviated commit hash.
        .or_else(|| git_output(project_root, &["rev-parse", "--short", "HEAD"]))
}

/// Return the last `n` commit one-liners (`<hash> <subject>`).
fn detect_recent_commits(project_root: &Path, n: usize) -> Vec<String> {
    let count = n.to_string();
    let Some(raw) = git_output(
        project_root,
        &["log", &format!("-{count}"), "--pretty=format:%h %s"],
    ) else {
        return Vec::new();
    };

    raw.lines().map(ToOwned::to_owned).collect()
}

/// Return the top-level entries of `project_root`, sorted.
///
/// Hidden entries (starting with `.`) are included.  The `target` directory
/// is excluded to avoid noise.
fn detect_file_structure(project_root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(project_root) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name == "target" {
                return None;
            }
            // Append `/` for directories.
            if e.path().is_dir() {
                Some(format!("{name}/"))
            } else {
                Some(name)
            }
        })
        .collect();

    names.sort();
    names
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Gather full project context for `project_root`.
///
/// All components are best-effort; individual failures produce `None` / empty
/// values rather than propagating errors.
///
/// # Edge cases
///
/// * **EC-38** — Local-only repo: `git_remote` will be `None` and
///   `git_branch` / `recent_commits` may still be populated if a local git
///   repo exists.
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use fixonce_core::detect::context::gather_context;
///
/// let ctx = gather_context(Path::new("."));
/// println!("branch: {:?}", ctx.git_branch);
/// ```
#[must_use]
pub fn gather_context(project_root: &Path) -> ProjectContext {
    let versions = detect_midnight_versions(project_root);
    let git_remote = detect_git_remote(project_root);
    let git_branch = detect_git_branch(project_root);
    let recent_commits = detect_recent_commits(project_root, 5);
    let file_structure = detect_file_structure(project_root);

    ProjectContext {
        versions,
        git_remote,
        git_branch,
        recent_commits,
        file_structure,
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

    // -----------------------------------------------------------------------
    // File structure
    // -----------------------------------------------------------------------

    #[test]
    fn file_structure_lists_top_level_entries() {
        let dir = tmp();
        fs::write(dir.path().join("README.md"), "").unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();

        let structure = detect_file_structure(dir.path());
        assert!(structure.contains(&"README.md".to_owned()));
        assert!(structure.contains(&"package.json".to_owned()));
        assert!(structure.contains(&"src/".to_owned()));
    }

    #[test]
    fn file_structure_excludes_target_dir() {
        let dir = tmp();
        fs::create_dir(dir.path().join("target")).unwrap();
        fs::write(dir.path().join("Cargo.toml"), "").unwrap();

        let structure = detect_file_structure(dir.path());
        assert!(!structure.iter().any(|s| s == "target/" || s == "target"));
        assert!(structure.contains(&"Cargo.toml".to_owned()));
    }

    #[test]
    fn file_structure_is_sorted() {
        let dir = tmp();
        fs::write(dir.path().join("zzz.txt"), "").unwrap();
        fs::write(dir.path().join("aaa.txt"), "").unwrap();
        fs::write(dir.path().join("mmm.txt"), "").unwrap();

        let structure = detect_file_structure(dir.path());
        let mut sorted = structure.clone();
        sorted.sort();
        assert_eq!(structure, sorted);
    }

    #[test]
    fn file_structure_empty_dir_returns_empty_vec() {
        let dir = tmp();
        let structure = detect_file_structure(dir.path());
        assert!(structure.is_empty());
    }

    // -----------------------------------------------------------------------
    // Git remote (EC-38)
    // -----------------------------------------------------------------------

    #[test]
    fn no_git_repo_returns_no_remote_and_no_branch() {
        let dir = tmp();
        // No git init — all git commands will fail.
        let remote = detect_git_remote(dir.path());
        let branch = detect_git_branch(dir.path());
        assert!(remote.is_none(), "expected no remote for non-git dir");
        assert!(branch.is_none(), "expected no branch for non-git dir");
    }

    #[test]
    fn local_only_repo_has_no_remote() {
        let dir = tmp();
        // Init a real git repo with no remote.
        let status = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(dir.path())
            .output();

        // Skip if git is unavailable in the test environment.
        if status.is_err() {
            return;
        }

        let remote = detect_git_remote(dir.path());
        assert!(remote.is_none(), "local-only repo should have no remote");
    }

    #[test]
    fn local_only_repo_branch_detected_after_first_commit() {
        let dir = tmp();

        macro_rules! git {
            ($($arg:expr),+) => {
                std::process::Command::new("git")
                    .args([$($arg),+])
                    .current_dir(dir.path())
                    .env("GIT_AUTHOR_NAME", "Test")
                    .env("GIT_AUTHOR_EMAIL", "test@test.com")
                    .env("GIT_COMMITTER_NAME", "Test")
                    .env("GIT_COMMITTER_EMAIL", "test@test.com")
                    .output()
            };
        }

        let init = git!("init", "-b", "main");
        if init.is_err() {
            return; // git not available
        }

        fs::write(dir.path().join("f.txt"), "x").unwrap();
        let _ = git!("add", ".");
        let _ = git!("commit", "-m", "initial");

        let branch = detect_git_branch(dir.path());
        assert_eq!(
            branch.as_deref(),
            Some("main"),
            "branch should be 'main' after init + commit"
        );
    }

    // -----------------------------------------------------------------------
    // Recent commits
    // -----------------------------------------------------------------------

    #[test]
    fn no_git_repo_returns_empty_commits() {
        let dir = tmp();
        let commits = detect_recent_commits(dir.path(), 5);
        assert!(commits.is_empty());
    }

    #[test]
    fn recent_commits_returns_at_most_n() {
        let dir = tmp();

        macro_rules! git {
            ($($arg:expr),+) => {
                std::process::Command::new("git")
                    .args([$($arg),+])
                    .current_dir(dir.path())
                    .env("GIT_AUTHOR_NAME", "Test")
                    .env("GIT_AUTHOR_EMAIL", "test@test.com")
                    .env("GIT_COMMITTER_NAME", "Test")
                    .env("GIT_COMMITTER_EMAIL", "test@test.com")
                    .output()
            };
        }

        if git!("init", "-b", "main").is_err() {
            return;
        }

        for i in 0..7u8 {
            fs::write(dir.path().join(format!("f{i}.txt")), "x").unwrap();
            let _ = git!("add", ".");
            let _ = git!("commit", "-m", &format!("commit {i}"));
        }

        let commits = detect_recent_commits(dir.path(), 5);
        assert!(
            commits.len() <= 5,
            "should return at most 5 commits, got {}",
            commits.len()
        );
    }

    // -----------------------------------------------------------------------
    // gather_context smoke test
    // -----------------------------------------------------------------------

    #[test]
    fn gather_context_non_git_project_runs_without_panic() {
        let dir = tmp();
        fs::write(
            dir.path().join("package.json"),
            r#"{"dependencies":{"@midnight-ntwrk/midnight-js-contracts":"^1.0.0"}}"#,
        )
        .unwrap();

        let ctx = gather_context(dir.path());
        // No git repo — remote and branch must be absent.
        assert!(ctx.git_remote.is_none());
        assert!(ctx.git_branch.is_none());
        assert!(ctx.recent_commits.is_empty());
        // Versions from package.json should be picked up.
        assert_eq!(ctx.versions.midnight_js.as_deref(), Some("1.0.0"));
        // File structure must include package.json.
        assert!(ctx.file_structure.contains(&"package.json".to_owned()));
    }
}
