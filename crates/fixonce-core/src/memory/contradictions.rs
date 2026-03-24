//! Contradiction detection and resolution between memory records.
//!
//! When two memories make conflicting claims a [`ContradictionPair`] is
//! recorded.  Users can cast [`TiebreakerVote`]s; once three or more
//! **distinct** users vote for the same memory the pair is automatically
//! resolved.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Tracks a detected contradiction between two memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContradictionPair {
    /// Unique identifier for this contradiction record.
    pub id: String,
    /// ID of the first memory in the conflicting pair.
    pub memory_a_id: String,
    /// ID of the second memory in the conflicting pair.
    pub memory_b_id: String,
    /// Current resolution state.
    pub resolution_status: ResolutionStatus,
    /// Votes cast by users to break the tie.
    pub tiebreaker_votes: Vec<TiebreakerVote>,
    /// ISO 8601 timestamp when the contradiction was detected.
    pub detected_at: String,
    /// ISO 8601 timestamp when the contradiction was resolved (if ever).
    pub resolved_at: Option<String>,
}

/// Whether the contradiction has been acted on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionStatus {
    /// No resolution reached yet.
    Open,
    /// A winner was determined by tiebreaker votes.
    Resolved,
    /// The contradiction was manually dismissed (e.g. not actually conflicting).
    Dismissed,
}

/// A single user's vote for one of the two memories in a contradiction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TiebreakerVote {
    /// ID of the voting user.
    pub user_id: String,
    /// The memory ID the user considers correct.
    pub voted_for: String,
    /// Optional free-text rationale.
    pub context: Option<String>,
    /// ISO 8601 timestamp when the vote was cast.
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Resolution logic
// ---------------------------------------------------------------------------

/// Minimum number of distinct users who must agree before a contradiction is
/// automatically resolved.
pub const RESOLUTION_VOTE_THRESHOLD: usize = 3;

/// Check whether a contradiction pair should be resolved.
///
/// Counts votes per candidate memory.  If any candidate has received
/// [`RESOLUTION_VOTE_THRESHOLD`] or more votes from **distinct** users, that
/// candidate wins and its memory ID is returned.
///
/// Returns `None` when no candidate has reached the threshold.
///
/// Only the most recent vote from each user is counted; earlier votes by the
/// same user for a different candidate are ignored.
#[must_use]
pub fn check_resolution(pair: &ContradictionPair) -> Option<String> {
    use std::collections::HashMap;

    // Deduplicate: keep only the last vote cast by each user.
    let mut last_vote_by_user: HashMap<&str, &str> = HashMap::new();
    for vote in &pair.tiebreaker_votes {
        last_vote_by_user.insert(&vote.user_id, &vote.voted_for);
    }

    // Tally deduplicated votes.
    let mut tally: HashMap<&str, usize> = HashMap::new();
    for memory_id in last_vote_by_user.values() {
        *tally.entry(memory_id).or_insert(0) += 1;
    }

    // Return the first candidate that meets the threshold.
    tally
        .into_iter()
        .find(|(_, count)| *count >= RESOLUTION_VOTE_THRESHOLD)
        .map(|(memory_id, _)| memory_id.to_owned())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pair(votes: Vec<(&str, &str)>) -> ContradictionPair {
        ContradictionPair {
            id: "pair-1".to_owned(),
            memory_a_id: "mem-a".to_owned(),
            memory_b_id: "mem-b".to_owned(),
            resolution_status: ResolutionStatus::Open,
            tiebreaker_votes: votes
                .into_iter()
                .map(|(user, mem)| TiebreakerVote {
                    user_id: user.to_owned(),
                    voted_for: mem.to_owned(),
                    context: None,
                    created_at: "2026-01-01T00:00:00Z".to_owned(),
                })
                .collect(),
            detected_at: "2026-01-01T00:00:00Z".to_owned(),
            resolved_at: None,
        }
    }

    #[test]
    fn no_votes_returns_none() {
        let pair = make_pair(vec![]);
        assert_eq!(check_resolution(&pair), None);
    }

    #[test]
    fn two_votes_for_same_memory_returns_none() {
        let pair = make_pair(vec![("u1", "mem-a"), ("u2", "mem-a")]);
        assert_eq!(check_resolution(&pair), None);
    }

    #[test]
    fn three_votes_for_same_memory_resolves() {
        let pair = make_pair(vec![("u1", "mem-a"), ("u2", "mem-a"), ("u3", "mem-a")]);
        assert_eq!(check_resolution(&pair), Some("mem-a".to_owned()));
    }

    #[test]
    fn split_votes_return_none() {
        let pair = make_pair(vec![
            ("u1", "mem-a"),
            ("u2", "mem-a"),
            ("u3", "mem-b"),
            ("u4", "mem-b"),
        ]);
        assert_eq!(check_resolution(&pair), None);
    }

    #[test]
    fn duplicate_user_votes_counted_once() {
        // u1 changes their vote — only the last entry counts.
        // This means effectively u1->mem-a, u2->mem-a, u3->mem-a (3 unique).
        let pair = make_pair(vec![
            ("u1", "mem-b"), // early vote, overridden below
            ("u2", "mem-a"),
            ("u3", "mem-a"),
            ("u1", "mem-a"), // final vote from u1
        ]);
        assert_eq!(check_resolution(&pair), Some("mem-a".to_owned()));
    }

    #[test]
    fn three_votes_for_b_resolves_to_b() {
        let pair = make_pair(vec![("u1", "mem-b"), ("u2", "mem-b"), ("u3", "mem-b")]);
        assert_eq!(check_resolution(&pair), Some("mem-b".to_owned()));
    }
}
