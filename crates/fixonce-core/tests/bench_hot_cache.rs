//! T307 — Performance benchmarks for hot cache operations.
//!
//! Uses `std::time::Instant` to assert timing constraints without requiring
//! the `criterion` crate or nightly Rust features.
//!
//! Assertions:
//! - Hot cache insert + query for 50 items < 50ms
//! - Signature computation for a single memory < 10ms

use std::time::Instant;

use fixonce_core::memory::{
    signatures::{compute_signature, signature_similarity, SessionHotCache, DEFAULT_HOT_CACHE_CAP},
    types::{EmbeddingStatus, Memory, MemoryType, PipelineStatus, SourceType},
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_memory(id: &str, content: &str) -> Memory {
    Memory {
        id: id.to_owned(),
        title: format!("Memory {id}"),
        content: content.to_owned(),
        summary: "Benchmark memory summary.".to_owned(),
        memory_type: MemoryType::BestPractice,
        source_type: SourceType::Manual,
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
        decay_score: 1.0,
        reinforcement_score: 1.0,
        last_accessed_at: None,
        embedding_status: EmbeddingStatus::Complete,
        pipeline_status: PipelineStatus::Complete,
        deleted_at: None,
        created_at: "2026-01-01T00:00:00Z".to_owned(),
        updated_at: "2026-01-01T00:00:00Z".to_owned(),
        created_by: "bench-user".to_owned(),
        anti_memory: None,
    }
}

fn rich_content(i: usize) -> String {
    format!(
        "useState useEffect fetchData apiCall handleError TypeError ENOENT src/components/Widget{i}.tsx \
        src/hooks/use{i}.ts src/utils/format{i}.ts src/api/client{i}.ts \
        error{i} FAIL_{i} fetchUser{i} processData{i} validateInput{i}",
        i = i
    )
}

// ---------------------------------------------------------------------------
// T307-a: Hot cache insert + query 50 items < 50ms
// ---------------------------------------------------------------------------

#[test]
fn hot_cache_insert_50_items_within_50ms() {
    let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);

    let start = Instant::now();
    for i in 0..DEFAULT_HOT_CACHE_CAP {
        cache.insert(make_memory(&format!("m{i}"), &rich_content(i)));
    }
    let elapsed = start.elapsed();

    assert_eq!(
        cache.len(),
        DEFAULT_HOT_CACHE_CAP,
        "cache must contain exactly {DEFAULT_HOT_CACHE_CAP} items"
    );
    assert!(
        elapsed.as_millis() < 50,
        "inserting {DEFAULT_HOT_CACHE_CAP} items took {}ms, must be <50ms",
        elapsed.as_millis()
    );
}

#[test]
fn hot_cache_query_50_items_within_50ms() {
    let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);
    for i in 0..DEFAULT_HOT_CACHE_CAP {
        cache.insert(make_memory(&format!("m{i}"), &rich_content(i)));
    }

    let profile =
        compute_signature("useState useEffect TypeError src/components/App.tsx fetchUser");

    let start = Instant::now();
    let results = cache.query_by_relevance(&profile);
    let elapsed = start.elapsed();

    assert_eq!(
        results.len(),
        DEFAULT_HOT_CACHE_CAP,
        "must return all {DEFAULT_HOT_CACHE_CAP} results"
    );
    assert!(
        elapsed.as_millis() < 50,
        "querying {DEFAULT_HOT_CACHE_CAP} items took {}ms, must be <50ms",
        elapsed.as_millis()
    );
}

#[test]
fn hot_cache_insert_and_query_combined_within_50ms() {
    // Benchmark the combined insert + query cycle as a single measurement.
    let start = Instant::now();

    let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);
    for i in 0..DEFAULT_HOT_CACHE_CAP {
        cache.insert(make_memory(&format!("m{i}"), &rich_content(i)));
    }

    let profile = compute_signature("useState useEffect fetchData TypeError");
    let _results = cache.query_by_relevance(&profile);

    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "combined insert+query of {DEFAULT_HOT_CACHE_CAP} items took {}ms, must be <50ms",
        elapsed.as_millis()
    );
}

// ---------------------------------------------------------------------------
// T307-b: Signature computation < 10ms per memory
// ---------------------------------------------------------------------------

#[test]
fn single_signature_computation_within_10ms() {
    let content = rich_content(0);

    let start = Instant::now();
    let sig = compute_signature(&content);
    let elapsed = start.elapsed();

    // Sanity check — signature was actually computed.
    assert!(
        !sig.file_patterns.is_empty() || !sig.sdk_methods.is_empty(),
        "signature must not be completely empty for rich content"
    );

    assert!(
        elapsed.as_millis() < 10,
        "single signature computation took {}ms, must be <10ms",
        elapsed.as_millis()
    );
}

#[test]
fn fifty_signature_computations_within_50ms() {
    let contents: Vec<String> = (0..50).map(rich_content).collect();

    let start = Instant::now();
    for content in &contents {
        let _ = compute_signature(content);
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 50,
        "50 signature computations took {}ms, must be <50ms",
        elapsed.as_millis()
    );
}

#[test]
fn signature_computation_of_large_content_within_10ms() {
    // Generate a very large content string (~10KB) to stress-test the parser.
    let large_content = (0..200)
        .map(|i| rich_content(i))
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        large_content.len() > 5_000,
        "test requires large content for stress-testing"
    );

    let start = Instant::now();
    let sig = compute_signature(&large_content);
    let elapsed = start.elapsed();

    assert!(
        !sig.sdk_methods.is_empty(),
        "large content must produce non-empty signature"
    );
    assert!(
        elapsed.as_millis() < 10,
        "large content signature took {}ms, must be <10ms",
        elapsed.as_millis()
    );
}

// ---------------------------------------------------------------------------
// T307-c: Signature similarity computation performance
// ---------------------------------------------------------------------------

#[test]
fn signature_similarity_computation_within_1ms() {
    let content_a = rich_content(0);
    let content_b = rich_content(1);

    let sig_a = compute_signature(&content_a);
    let sig_b = compute_signature(&content_b);

    let start = Instant::now();
    let sim = signature_similarity(&sig_a, &sig_b);
    let elapsed = start.elapsed();

    assert!(
        sim >= 0.0 && sim <= 1.0,
        "similarity must be in [0, 1], got {sim}"
    );
    assert!(
        elapsed.as_micros() < 1_000, // 1ms in microseconds
        "similarity computation took {}µs, must be <1ms",
        elapsed.as_micros()
    );
}

#[test]
fn hot_cache_eviction_on_overflow_stays_within_capacity() {
    // Insert 60 items into a cap-50 cache; verify no overflow and timing.
    let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);

    let start = Instant::now();
    for i in 0..60 {
        cache.insert(make_memory(&format!("m{i}"), &rich_content(i)));
    }
    let elapsed = start.elapsed();

    assert_eq!(
        cache.len(),
        DEFAULT_HOT_CACHE_CAP,
        "cache must not exceed capacity after overflow"
    );
    assert!(
        elapsed.as_millis() < 50,
        "60-item insert into cap-50 cache took {}ms, must be <50ms",
        elapsed.as_millis()
    );
}

#[test]
fn repeated_get_on_hot_cache_stays_fast() {
    let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);
    for i in 0..DEFAULT_HOT_CACHE_CAP {
        cache.insert(make_memory(&format!("m{i}"), &rich_content(i)));
    }

    let start = Instant::now();
    for i in 0..DEFAULT_HOT_CACHE_CAP {
        let _ = cache.get(&format!("m{i}"));
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 10,
        "50 cache.get() calls took {}ms, must be <10ms",
        elapsed.as_millis()
    );
}
