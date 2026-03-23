//! T301 — Memory lifecycle end-to-end integration tests.
//!
//! Tests the full memory lifecycle: create request → serialize → validate →
//! format output, decay score changes on feedback simulation, and flagging
//! memories with low decay for deletion.
//!
//! No external network calls are made; all logic is exercised through
//! in-process pure functions.

use fixonce_core::{
    memory::{
        dynamics::{
            apply_reinforcement, compute_decay, should_soft_delete, DEFAULT_DECAY_THRESHOLD,
            DEFAULT_HALF_LIFE_DAYS,
        },
        types::{
            CreateMemoryRequest, EmbeddingStatus, FeedbackRating, Memory, MemoryType,
            PipelineStatus, SourceType,
        },
    },
    output::text::{format_memory_list_text, format_memory_text},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_create_request(title: &str, content: &str) -> CreateMemoryRequest {
    CreateMemoryRequest {
        title: title.to_owned(),
        content: content.to_owned(),
        summary: "Integration test summary.".to_owned(),
        memory_type: MemoryType::BestPractice,
        source_type: SourceType::Manual,
        language: Some("rust".to_owned()),
        embedding: None,
        compact_pragma: None,
        compact_compiler: None,
        midnight_js: None,
        indexer_version: None,
        node_version: None,
        source_url: Some("https://example.com/docs".to_owned()),
        repo_url: None,
        task_summary: None,
        session_id: Some("session-test-001".to_owned()),
    }
}

fn make_memory(id: &str, title: &str, decay_score: f64) -> Memory {
    Memory {
        id: id.to_owned(),
        title: title.to_owned(),
        content: "Memory content for integration test.".to_owned(),
        summary: "Short summary for testing.".to_owned(),
        memory_type: MemoryType::Gotcha,
        source_type: SourceType::Observation,
        language: Some("rust".to_owned()),
        compact_pragma: None,
        compact_compiler: None,
        midnight_js: None,
        indexer_version: None,
        node_version: None,
        source_url: None,
        repo_url: None,
        task_summary: None,
        session_id: None,
        decay_score,
        reinforcement_score: 1.0,
        last_accessed_at: None,
        embedding_status: EmbeddingStatus::Complete,
        pipeline_status: PipelineStatus::Complete,
        deleted_at: None,
        created_at: "2026-01-01T00:00:00Z".to_owned(),
        updated_at: "2026-01-02T00:00:00Z".to_owned(),
        created_by: "user-lifecycle-test".to_owned(),
        anti_memory: None,
    }
}

// ---------------------------------------------------------------------------
// T301-a: Create request → serialize → deserialize round-trip
// ---------------------------------------------------------------------------

#[test]
fn create_request_round_trips_through_json() {
    let req = make_create_request(
        "Always use Arc<Mutex<T>> for shared mutable state",
        "When sharing mutable state across async tasks use Arc<Mutex<T>>.",
    );

    let serialized = serde_json::to_string(&req).expect("serialization must succeed");
    let deserialized: CreateMemoryRequest =
        serde_json::from_str(&serialized).expect("deserialization must succeed");

    assert_eq!(req.title, deserialized.title);
    assert_eq!(req.content, deserialized.content);
    assert_eq!(req.summary, deserialized.summary);
    assert_eq!(req.language, deserialized.language);
    assert_eq!(req.session_id, deserialized.session_id);
}

#[test]
fn create_request_optional_fields_serialise_as_null_or_absent() {
    let req = make_create_request("Title", "Content");
    let json_val: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&req).unwrap()).unwrap();

    // embedding is skip_serializing_if = Option::is_none
    assert!(
        json_val.get("embedding").is_none(),
        "embedding should be absent when None"
    );

    // repo_url should be null when absent
    assert_eq!(json_val.get("repo_url").and_then(|v| v.as_null()), Some(()));
}

// ---------------------------------------------------------------------------
// T301-b: Memory validation via output formatting
// ---------------------------------------------------------------------------

#[test]
fn format_memory_text_contains_all_key_fields() {
    let m = make_memory("mem-lifecycle-001", "Rust ownership gotcha", 0.9);
    let output = format_memory_text(&m);

    assert!(
        output.contains("mem-lifecycle-001"),
        "must contain memory id"
    );
    assert!(
        output.contains("Rust ownership gotcha"),
        "must contain title"
    );
    assert!(output.contains("gotcha"), "must contain memory type");
    assert!(output.contains("0.9000"), "must contain decay score");
    assert!(
        output.contains("integration test"),
        "must contain content snippet"
    );
}

#[test]
fn format_memory_list_returns_numbered_entries() {
    let memories = vec![
        make_memory("m1", "First memory", 0.9),
        make_memory("m2", "Second memory", 0.7),
        make_memory("m3", "Third memory", 0.5),
    ];

    let output = format_memory_list_text(&memories);
    assert!(output.contains("1."), "must have first entry numbered");
    assert!(output.contains("2."), "must have second entry numbered");
    assert!(output.contains("3."), "must have third entry numbered");
    assert!(output.contains("First memory"));
    assert!(output.contains("Second memory"));
    assert!(output.contains("Third memory"));
}

#[test]
fn format_empty_memory_list_returns_placeholder() {
    let output = format_memory_list_text(&[]);
    assert_eq!(output, "No memories found.\n");
}

// ---------------------------------------------------------------------------
// T301-c: Decay score changes on feedback simulation
// ---------------------------------------------------------------------------

#[test]
fn helpful_feedback_reinforces_decay_score() {
    let initial_score = 0.6;
    let reinforcement_points = 0.2; // simulating "helpful" feedback

    let new_score = apply_reinforcement(initial_score, reinforcement_points);
    assert!(
        new_score > initial_score,
        "helpful feedback must increase score: {initial_score} → {new_score}"
    );
    assert!(
        (new_score - 0.8).abs() < 1e-10,
        "expected 0.8 got {new_score}"
    );
}

#[test]
fn damaging_feedback_reduces_score() {
    let initial_score = 0.6;
    let penalty = -0.3; // simulating "damaging" feedback

    let new_score = apply_reinforcement(initial_score, penalty);
    assert!(
        new_score < initial_score,
        "damaging feedback must reduce score: {initial_score} → {new_score}"
    );
    assert!(
        (new_score - 0.3).abs() < 1e-10,
        "expected 0.3 got {new_score}"
    );
}

#[test]
fn outdated_feedback_simulation_does_not_clamp_above_zero() {
    // Simulate "outdated" feedback as a moderate negative reinforcement
    let initial_score = 0.15;
    let penalty = -0.2;

    let new_score = apply_reinforcement(initial_score, penalty);
    assert!(
        new_score >= 0.0,
        "score must not drop below 0.0, got {new_score}"
    );
    assert_eq!(new_score, 0.0, "clamped to 0.0");
}

#[test]
fn feedback_rating_display_matches_expected_strings() {
    assert_eq!(FeedbackRating::Helpful.to_string(), "helpful");
    assert_eq!(FeedbackRating::Outdated.to_string(), "outdated");
    assert_eq!(FeedbackRating::Damaging.to_string(), "damaging");
}

// ---------------------------------------------------------------------------
// T301-d: Decay over time → threshold check → soft-delete flag
// ---------------------------------------------------------------------------

#[test]
fn freshly_created_memory_does_not_get_flagged_for_deletion() {
    let initial_score = 1.0;
    let days_elapsed = 0.0;

    let decayed = compute_decay(initial_score, days_elapsed, DEFAULT_HALF_LIFE_DAYS);
    assert!(
        !should_soft_delete(decayed, DEFAULT_DECAY_THRESHOLD),
        "fresh memory must not be flagged for deletion"
    );
}

#[test]
fn memory_after_two_half_lives_is_at_quarter_score() {
    let initial_score = 1.0;
    let decayed = compute_decay(
        initial_score,
        DEFAULT_HALF_LIFE_DAYS * 2.0,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert!(
        (decayed - 0.25).abs() < 1e-10,
        "after two half-lives score should be 0.25, got {decayed}"
    );
    assert!(
        !should_soft_delete(decayed, DEFAULT_DECAY_THRESHOLD),
        "0.25 is above threshold 0.1, should not be flagged"
    );
}

#[test]
fn very_old_memory_gets_flagged_for_deletion() {
    let initial_score = 1.0;
    // After ~10 half-lives the score is ~1/1024 ≈ 0.001, well below threshold
    let decayed = compute_decay(
        initial_score,
        DEFAULT_HALF_LIFE_DAYS * 10.0,
        DEFAULT_HALF_LIFE_DAYS,
    );

    assert!(
        should_soft_delete(decayed, DEFAULT_DECAY_THRESHOLD),
        "very old memory (score {decayed}) must be flagged for soft-delete"
    );
}

#[test]
fn reinforcement_rescues_memory_from_deletion_threshold() {
    let initial_score = 1.0;
    // Decay enough to go below threshold
    let decayed = compute_decay(
        initial_score,
        DEFAULT_HALF_LIFE_DAYS * 10.0,
        DEFAULT_HALF_LIFE_DAYS,
    );
    assert!(
        should_soft_delete(decayed, DEFAULT_DECAY_THRESHOLD),
        "precondition: memory should be below threshold before reinforcement"
    );

    // Apply strong reinforcement (simulating a user flagging it as still helpful)
    let rescued = apply_reinforcement(decayed, 0.5);
    assert!(
        !should_soft_delete(rescued, DEFAULT_DECAY_THRESHOLD),
        "after reinforcement score {rescued} should be above threshold"
    );
}

// ---------------------------------------------------------------------------
// T301-e: Memory type and source type round-trip
// ---------------------------------------------------------------------------

#[test]
fn all_memory_types_serialize_and_display_correctly() {
    let cases = [
        (MemoryType::Gotcha, "gotcha"),
        (MemoryType::BestPractice, "best_practice"),
        (MemoryType::Correction, "correction"),
        (MemoryType::AntiPattern, "anti_pattern"),
        (MemoryType::Discovery, "discovery"),
    ];

    for (variant, expected_display) in &cases {
        assert_eq!(
            variant.to_string(),
            *expected_display,
            "MemoryType display mismatch for {expected_display}"
        );

        let json = serde_json::to_string(variant).unwrap();
        let deserialized: MemoryType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            *variant, deserialized,
            "MemoryType round-trip failed for {expected_display}"
        );
    }
}

#[test]
fn all_source_types_serialize_and_display_correctly() {
    let cases = [
        (SourceType::Correction, "correction"),
        (SourceType::Observation, "observation"),
        (SourceType::PrFeedback, "pr_feedback"),
        (SourceType::Manual, "manual"),
        (SourceType::Harvested, "harvested"),
    ];

    for (variant, expected_display) in &cases {
        assert_eq!(
            variant.to_string(),
            *expected_display,
            "SourceType display mismatch"
        );

        let json = serde_json::to_string(variant).unwrap();
        let deserialized: SourceType = serde_json::from_str(&json).unwrap();
        assert_eq!(
            *variant, deserialized,
            "SourceType round-trip failed for {expected_display}"
        );
    }
}
