//! Memory lineage tracking.
//!
//! Every mutation to a memory (creation, update, replacement, merge, or user
//! feedback) is recorded as a [`LineageEvent`].  Events form a directed
//! acyclic graph via their [`parent_id`](LineageEvent::parent_id) field,
//! allowing the full history of a memory to be reconstructed.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A single node in a memory's lineage graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEvent {
    /// Unique identifier for this event.
    pub id: String,
    /// The memory this event applies to.
    pub memory_id: String,
    /// The preceding event in the lineage chain, if any.
    pub parent_id: Option<String>,
    /// The kind of mutation that occurred.
    pub action: LineageAction,
    /// Optional human-readable explanation for the change.
    pub rationale: Option<String>,
    /// Arbitrary structured metadata (e.g. diff, reviewer, tool version).
    pub metadata: serde_json::Value,
    /// ISO 8601 timestamp when the event was recorded.
    pub created_at: String,
}

/// The type of lineage mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageAction {
    /// A new memory was created from scratch.
    Create,
    /// An existing memory's fields were updated.
    Update,
    /// One memory replaced another (e.g. superseded by a newer version).
    Replace,
    /// Two memories were merged into one.
    Merge,
    /// A user submitted feedback that mutated the memory's scores.
    Feedback,
}

impl std::fmt::Display for LineageAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Replace => "replace",
            Self::Merge => "merge",
            Self::Feedback => "feedback",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// Chain traversal helpers
// ---------------------------------------------------------------------------

/// Build an ordered lineage chain for `memory_id` from a flat slice of events.
///
/// The chain starts at the `Create` event (root) and follows `parent_id`
/// links forward in time, ending at the most recent event.
///
/// Events that do not belong to `memory_id` are silently ignored.
///
/// If `events` contains a cycle (malformed data) traversal stops once the
/// visited-set prevents infinite recursion.
#[must_use]
pub fn build_chain<'a>(memory_id: &str, events: &'a [LineageEvent]) -> Vec<&'a LineageEvent> {
    use std::collections::{HashMap, HashSet};

    // Index events by their id for O(1) parent lookup.
    let by_id: HashMap<&str, &LineageEvent> = events
        .iter()
        .filter(|e| e.memory_id == memory_id)
        .map(|e| (e.id.as_str(), e))
        .collect();

    // Find root(s): events with no parent (or whose parent is not in the set).
    let root = by_id.values().find(|e| {
        e.parent_id
            .as_deref()
            .is_none_or(|p| !by_id.contains_key(p))
    });

    let Some(root) = root else {
        return vec![];
    };

    // Walk forward: for each event find the child that references it as parent.
    // Build a parent→child map.
    let child_of: HashMap<&str, &LineageEvent> = by_id
        .values()
        .filter_map(|e| e.parent_id.as_deref().map(|p| (p, *e)))
        .collect();

    let mut chain = Vec::new();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut current: &LineageEvent = root;

    loop {
        if !visited.insert(current.id.as_str()) {
            break; // cycle guard
        }
        chain.push(current);
        match child_of.get(current.id.as_str()) {
            Some(next) => current = next,
            None => break,
        }
    }

    chain
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(id: &str, memory_id: &str, parent: Option<&str>, action: LineageAction) -> LineageEvent {
        LineageEvent {
            id: id.to_owned(),
            memory_id: memory_id.to_owned(),
            parent_id: parent.map(str::to_owned),
            action,
            rationale: None,
            metadata: json!({}),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
        }
    }

    #[test]
    fn empty_events_returns_empty_chain() {
        assert!(build_chain("mem-1", &[]).is_empty());
    }

    #[test]
    fn single_create_event_is_chain_of_one() {
        let events = vec![ev("e1", "mem-1", None, LineageAction::Create)];
        let chain = build_chain("mem-1", &events);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].id, "e1");
    }

    #[test]
    fn chain_ordered_root_to_tip() {
        let events = vec![
            ev("e3", "mem-1", Some("e2"), LineageAction::Update),
            ev("e1", "mem-1", None, LineageAction::Create),
            ev("e2", "mem-1", Some("e1"), LineageAction::Update),
        ];
        let chain = build_chain("mem-1", &events);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].id, "e1");
        assert_eq!(chain[1].id, "e2");
        assert_eq!(chain[2].id, "e3");
    }

    #[test]
    fn unrelated_events_excluded() {
        let events = vec![
            ev("e1", "mem-1", None, LineageAction::Create),
            ev("e2", "mem-2", None, LineageAction::Create),
        ];
        let chain = build_chain("mem-1", &events);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].memory_id, "mem-1");
    }

    #[test]
    fn lineage_action_display() {
        assert_eq!(LineageAction::Create.to_string(), "create");
        assert_eq!(LineageAction::Replace.to_string(), "replace");
        assert_eq!(LineageAction::Merge.to_string(), "merge");
        assert_eq!(LineageAction::Feedback.to_string(), "feedback");
    }
}
