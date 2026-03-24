//! T304 — Memory dynamics end-to-end integration tests.
//!
//! Tests contradiction detection and resolution logic, decay over time with
//! reinforcement, threshold checking, and lineage chain building.

use fixonce_core::memory::{
    contradictions::{
        check_resolution, ContradictionPair, ResolutionStatus, TiebreakerVote,
        RESOLUTION_VOTE_THRESHOLD,
    },
    dynamics::{
        apply_reinforcement, compute_decay, should_soft_delete, DEFAULT_DECAY_THRESHOLD,
        DEFAULT_HALF_LIFE_DAYS,
    },
    lineage::{build_chain, LineageAction, LineageEvent},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_pair(id: &str, mem_a: &str, mem_b: &str, votes: Vec<(&str, &str)>) -> ContradictionPair {
    ContradictionPair {
        id: id.to_owned(),
        memory_a_id: mem_a.to_owned(),
        memory_b_id: mem_b.to_owned(),
        resolution_status: ResolutionStatus::Open,
        tiebreaker_votes: votes
            .into_iter()
            .map(|(user, voted)| TiebreakerVote {
                user_id: user.to_owned(),
                voted_for: voted.to_owned(),
                context: None,
                created_at: "2026-01-01T00:00:00Z".to_owned(),
            })
            .collect(),
        detected_at: "2026-01-01T00:00:00Z".to_owned(),
        resolved_at: None,
    }
}

fn make_event(
    id: &str,
    memory_id: &str,
    parent: Option<&str>,
    action: LineageAction,
) -> LineageEvent {
    LineageEvent {
        id: id.to_owned(),
        memory_id: memory_id.to_owned(),
        parent_id: parent.map(str::to_owned),
        action,
        rationale: None,
        metadata: serde_json::json!({}),
        created_at: "2026-01-01T00:00:00Z".to_owned(),
    }
}

// ---------------------------------------------------------------------------
// T304-a: Contradiction detection → resolution logic
// ---------------------------------------------------------------------------

#[test]
fn contradiction_with_no_votes_remains_open() {
    let pair = make_pair("c1", "mem-a", "mem-b", vec![]);
    assert_eq!(
        check_resolution(&pair),
        None,
        "contradiction with no votes must not resolve"
    );
}

#[test]
fn two_votes_for_same_memory_is_insufficient() {
    let pair = make_pair(
        "c2",
        "mem-a",
        "mem-b",
        vec![("user-1", "mem-a"), ("user-2", "mem-a")],
    );
    assert_eq!(
        check_resolution(&pair),
        None,
        "two votes is below the threshold of {RESOLUTION_VOTE_THRESHOLD}"
    );
}

#[test]
fn three_votes_for_one_memory_resolves_to_that_memory() {
    let pair = make_pair(
        "c3",
        "mem-a",
        "mem-b",
        vec![
            ("user-1", "mem-a"),
            ("user-2", "mem-a"),
            ("user-3", "mem-a"),
        ],
    );
    assert_eq!(
        check_resolution(&pair),
        Some("mem-a".to_owned()),
        "three votes for mem-a must resolve the contradiction"
    );
}

#[test]
fn split_votes_two_two_do_not_resolve() {
    let pair = make_pair(
        "c4",
        "mem-a",
        "mem-b",
        vec![
            ("user-1", "mem-a"),
            ("user-2", "mem-a"),
            ("user-3", "mem-b"),
            ("user-4", "mem-b"),
        ],
    );
    assert_eq!(
        check_resolution(&pair),
        None,
        "split votes must not produce a winner"
    );
}

#[test]
fn user_changing_vote_only_latest_vote_counts() {
    // user-1 initially votes for mem-b, then changes to mem-a
    // Expected: u1→mem-a, u2→mem-a, u3→mem-a (3 unique) → resolves mem-a
    let pair = make_pair(
        "c5",
        "mem-a",
        "mem-b",
        vec![
            ("user-1", "mem-b"), // overridden below
            ("user-2", "mem-a"),
            ("user-3", "mem-a"),
            ("user-1", "mem-a"), // final vote
        ],
    );
    assert_eq!(
        check_resolution(&pair),
        Some("mem-a".to_owned()),
        "only the latest vote per user must count"
    );
}

#[test]
fn resolution_threshold_constant_is_three() {
    assert_eq!(
        RESOLUTION_VOTE_THRESHOLD, 3,
        "resolution threshold must be 3"
    );
}

#[test]
fn contradiction_pair_serializes_round_trip() {
    let pair = make_pair("c6", "mem-x", "mem-y", vec![("user-1", "mem-x")]);
    let json = serde_json::to_string(&pair).expect("serialize");
    let restored: ContradictionPair = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.id, "c6");
    assert_eq!(restored.tiebreaker_votes.len(), 1);
    assert_eq!(restored.resolution_status, ResolutionStatus::Open);
}

// ---------------------------------------------------------------------------
// T304-b: Decay over time → reinforcement → threshold check
// ---------------------------------------------------------------------------

#[test]
fn score_halves_at_exactly_one_half_life() {
    let score = compute_decay(1.0, DEFAULT_HALF_LIFE_DAYS, DEFAULT_HALF_LIFE_DAYS);
    assert!(
        (score - 0.5).abs() < 1e-10,
        "score after one half-life must be 0.5, got {score}"
    );
}

#[test]
fn decay_is_monotonically_decreasing_over_time() {
    let initial = 1.0;
    let scores: Vec<f64> = (0..10)
        .map(|i| compute_decay(initial, i as f64 * 10.0, DEFAULT_HALF_LIFE_DAYS))
        .collect();

    for i in 1..scores.len() {
        assert!(
            scores[i] <= scores[i - 1],
            "decay must be monotonically non-increasing: scores[{i}]={} > scores[{}]={}",
            scores[i],
            i - 1,
            scores[i - 1]
        );
    }
}

#[test]
fn apply_reinforcement_multiple_times_caps_at_one() {
    let mut score = 0.3;
    for _ in 0..10 {
        score = apply_reinforcement(score, 0.2);
    }
    assert_eq!(
        score, 1.0,
        "score must be clamped at 1.0 after many reinforcements"
    );
}

#[test]
fn decay_then_reinforce_cycle_simulates_real_use() {
    // Simulate a memory that decays, then gets reinforced periodically.
    let initial = 1.0;

    // After 3 months (3 half-lives) the score is ~0.125
    let after_decay = compute_decay(
        initial,
        DEFAULT_HALF_LIFE_DAYS * 3.0,
        DEFAULT_HALF_LIFE_DAYS,
    );
    assert!(after_decay < 0.2, "after 3 half-lives score must be low");

    // A wave of 5 helpful feedback items, each adding 0.1
    let mut score = after_decay;
    for _ in 0..5 {
        score = apply_reinforcement(score, 0.1);
    }
    assert!(
        score > after_decay,
        "score must increase after reinforcement"
    );
    assert!(
        !should_soft_delete(score, DEFAULT_DECAY_THRESHOLD),
        "reinforced score {score} must not trigger soft-delete"
    );
}

#[test]
fn threshold_check_at_exactly_threshold_value_does_not_delete() {
    // should_soft_delete uses strict less-than, so equal to threshold = safe
    assert!(
        !should_soft_delete(DEFAULT_DECAY_THRESHOLD, DEFAULT_DECAY_THRESHOLD),
        "score at exactly threshold must NOT trigger soft-delete"
    );
}

#[test]
fn threshold_check_one_epsilon_below_threshold_triggers_delete() {
    let just_below = DEFAULT_DECAY_THRESHOLD - 1e-10;
    assert!(
        should_soft_delete(just_below, DEFAULT_DECAY_THRESHOLD),
        "score just below threshold ({just_below}) must trigger soft-delete"
    );
}

#[test]
fn zero_score_always_triggers_delete() {
    assert!(
        should_soft_delete(0.0, DEFAULT_DECAY_THRESHOLD),
        "score of 0.0 must always trigger soft-delete"
    );
}

// ---------------------------------------------------------------------------
// T304-c: Lineage chain building
// ---------------------------------------------------------------------------

#[test]
fn lineage_chain_for_single_create_event() {
    let events = vec![make_event("e1", "mem-1", None, LineageAction::Create)];
    let chain = build_chain("mem-1", &events);
    assert_eq!(chain.len(), 1, "chain of single event must have length 1");
    assert_eq!(chain[0].id, "e1");
    assert_eq!(chain[0].action, LineageAction::Create);
}

#[test]
fn lineage_chain_ordered_root_to_most_recent() {
    let events = vec![
        make_event("e3", "mem-1", Some("e2"), LineageAction::Feedback),
        make_event("e1", "mem-1", None, LineageAction::Create),
        make_event("e2", "mem-1", Some("e1"), LineageAction::Update),
    ];

    let chain = build_chain("mem-1", &events);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[0].id, "e1", "root must be first");
    assert_eq!(chain[1].id, "e2");
    assert_eq!(chain[2].id, "e3", "most recent must be last");
}

#[test]
fn lineage_chain_excludes_unrelated_memory_events() {
    let events = vec![
        make_event("e1", "mem-1", None, LineageAction::Create),
        make_event("e2", "mem-2", None, LineageAction::Create),
        make_event("e3", "mem-2", Some("e2"), LineageAction::Update),
    ];

    let chain_1 = build_chain("mem-1", &events);
    let chain_2 = build_chain("mem-2", &events);

    assert_eq!(chain_1.len(), 1, "chain for mem-1 must have 1 event");
    assert_eq!(chain_2.len(), 2, "chain for mem-2 must have 2 events");
    assert!(
        chain_1.iter().all(|e| e.memory_id == "mem-1"),
        "mem-1 chain must only contain mem-1 events"
    );
}

#[test]
fn lineage_chain_empty_for_unknown_memory_id() {
    let events = vec![make_event("e1", "mem-1", None, LineageAction::Create)];
    let chain = build_chain("mem-unknown", &events);
    assert!(chain.is_empty(), "chain for unknown memory must be empty");
}

#[test]
fn lineage_chain_for_replace_and_merge_actions() {
    let events = vec![
        make_event("e1", "mem-1", None, LineageAction::Create),
        make_event("e2", "mem-1", Some("e1"), LineageAction::Replace),
        make_event("e3", "mem-1", Some("e2"), LineageAction::Merge),
    ];

    let chain = build_chain("mem-1", &events);
    assert_eq!(chain.len(), 3);
    assert_eq!(chain[2].action, LineageAction::Merge);
}

#[test]
fn lineage_action_display_values_are_correct() {
    assert_eq!(LineageAction::Create.to_string(), "create");
    assert_eq!(LineageAction::Update.to_string(), "update");
    assert_eq!(LineageAction::Replace.to_string(), "replace");
    assert_eq!(LineageAction::Merge.to_string(), "merge");
    assert_eq!(LineageAction::Feedback.to_string(), "feedback");
}

#[test]
fn contradiction_resolution_chain_building_integration() {
    // Simulate: a contradiction is detected, votes are cast, resolved,
    // and the winning memory gets a Feedback lineage event.
    let pair = make_pair(
        "contradiction-001",
        "mem-a",
        "mem-b",
        vec![("alice", "mem-a"), ("bob", "mem-a"), ("charlie", "mem-a")],
    );

    let winner = check_resolution(&pair);
    assert_eq!(winner, Some("mem-a".to_owned()));

    // After resolution, the winning memory should get a Feedback event in its lineage.
    let events = vec![
        make_event("e1", "mem-a", None, LineageAction::Create),
        make_event("e2", "mem-a", Some("e1"), LineageAction::Feedback),
    ];

    let chain = build_chain("mem-a", &events);
    assert_eq!(chain.len(), 2);
    assert_eq!(chain[1].action, LineageAction::Feedback);
}
