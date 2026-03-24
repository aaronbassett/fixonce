//! Memory signatures and session hot cache.
//!
//! A [`MemorySignature`] is a compact representation of a memory's topical
//! content — expressed as bags of file patterns, error patterns, and SDK
//! methods.  Signatures enable fast cosine-similarity ranking inside the
//! [`SessionHotCache`] without requiring embedding lookups.

use std::collections::HashMap;

use crate::memory::types::Memory;

// ---------------------------------------------------------------------------
// MemorySignature
// ---------------------------------------------------------------------------

/// A lightweight topical fingerprint derived from memory content.
#[derive(Debug, Clone, Default)]
pub struct MemorySignature {
    /// File-name / path patterns mentioned in the content (e.g. `*.ts`, `src/`).
    pub file_patterns: Vec<String>,
    /// Error / exception patterns (e.g. `ENOENT`, `TypeError`).
    pub error_patterns: Vec<String>,
    /// SDK method / function names referenced (e.g. `useState`, `fetch`).
    pub sdk_methods: Vec<String>,
}

// ---------------------------------------------------------------------------
// Signature computation
// ---------------------------------------------------------------------------

/// Compute a [`MemorySignature`] from raw memory content.
///
/// The heuristics used here are intentionally lightweight — they look for
/// common patterns rather than performing full NLP.
#[must_use]
pub fn compute_signature(content: &str) -> MemorySignature {
    let mut file_patterns: Vec<String> = Vec::new();
    let mut error_patterns: Vec<String> = Vec::new();
    let mut sdk_methods: Vec<String> = Vec::new();

    for token in content.split_whitespace() {
        let token = token.trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != '*');

        // File patterns: contain a dot or slash, or start with `*.`
        if (token.contains('.') || token.contains('/'))
            && token.len() > 1
            && !token.starts_with("http")
        {
            file_patterns.push(token.to_owned());
        }

        // Error patterns: `Error`, `Exception`, `FAIL`, screaming-case words ≥ 4 chars
        if token.ends_with("Error")
            || token.ends_with("Exception")
            || token.ends_with("FAIL")
            || (token.len() >= 4 && token.chars().all(|c| c.is_ascii_uppercase() || c == '_'))
        {
            error_patterns.push(token.to_owned());
        }

        // SDK methods: camelCase tokens that look like function calls (lower start, upper mid)
        if token.len() >= 3
            && token.chars().next().is_some_and(char::is_lowercase)
            && token.chars().any(char::is_uppercase)
            && token.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            sdk_methods.push(token.to_owned());
        }
    }

    file_patterns.dedup();
    error_patterns.dedup();
    sdk_methods.dedup();

    MemorySignature {
        file_patterns,
        error_patterns,
        sdk_methods,
    }
}

// ---------------------------------------------------------------------------
// Cosine similarity
// ---------------------------------------------------------------------------

/// Build a term-frequency vector from a string slice.
fn term_frequencies(terms: &[String]) -> HashMap<&str, f64> {
    let mut freq: HashMap<&str, f64> = HashMap::new();
    for term in terms {
        *freq.entry(term.as_str()).or_insert(0.0) += 1.0;
    }
    freq
}

/// Compute cosine similarity between two [`MemorySignature`]s.
///
/// The signatures are flattened into combined term-frequency vectors
/// (`file_patterns` + `error_patterns` + `sdk_methods`) and compared via the
/// standard cosine formula.  Returns a value in `[0.0, 1.0]`; returns `0.0`
/// when either signature is empty.
#[must_use]
pub fn signature_similarity(a: &MemorySignature, b: &MemorySignature) -> f64 {
    let all_a: Vec<String> = a
        .file_patterns
        .iter()
        .chain(&a.error_patterns)
        .chain(&a.sdk_methods)
        .cloned()
        .collect();
    let all_b: Vec<String> = b
        .file_patterns
        .iter()
        .chain(&b.error_patterns)
        .chain(&b.sdk_methods)
        .cloned()
        .collect();

    if all_a.is_empty() || all_b.is_empty() {
        return 0.0;
    }

    let freq_a = term_frequencies(&all_a);
    let freq_b = term_frequencies(&all_b);

    // Dot product over shared terms.
    let dot: f64 = freq_a
        .iter()
        .map(|(term, &wa)| freq_b.get(term).map_or(0.0, |&wb| wa * wb))
        .sum();

    let mag_a: f64 = freq_a.values().map(|w| w * w).sum::<f64>().sqrt();
    let mag_b: f64 = freq_b.values().map(|w| w * w).sum::<f64>().sqrt();

    if mag_a == 0.0 || mag_b == 0.0 {
        return 0.0;
    }

    (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// SessionHotCache
// ---------------------------------------------------------------------------

/// Default maximum number of memories kept in the hot cache.
pub const DEFAULT_HOT_CACHE_CAP: usize = 50;

/// An in-memory LRU cache of recently accessed or highly relevant memories
/// for the current agent session.
///
/// Insertion evicts the least-recently-used entry when the cache is at
/// capacity.  [`query_by_relevance`](SessionHotCache::query_by_relevance)
/// returns all cached entries ranked by cosine similarity to a provided
/// session profile.
pub struct SessionHotCache {
    /// Primary storage: memory-id → Memory.
    cache: HashMap<String, Memory>,
    /// Access order: front = most recently used, back = least recently used.
    access_order: Vec<String>,
    /// Maximum number of entries.
    cap: usize,
}

impl SessionHotCache {
    /// Create a new cache with the given capacity.
    ///
    /// `cap` is clamped to a minimum of 1.
    #[must_use]
    pub fn new(cap: usize) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: Vec::new(),
            cap: cap.max(1),
        }
    }

    /// Insert a memory.  If the memory is already cached its position in the
    /// LRU order is refreshed.  When the cache is full the least-recently-used
    /// entry is evicted first.
    pub fn insert(&mut self, memory: Memory) {
        let id = memory.id.clone();

        // If already present, remove from current position in access_order.
        if self.cache.contains_key(&id) {
            self.access_order.retain(|x| x != &id);
        } else if self.cache.len() >= self.cap {
            self.evict_lru();
        }

        self.cache.insert(id.clone(), memory);
        self.access_order.insert(0, id); // most-recently-used at front
    }

    /// Retrieve a memory by ID, marking it as most-recently-used.
    ///
    /// Returns `None` if the memory is not in the cache.
    pub fn get(&mut self, id: &str) -> Option<&Memory> {
        if self.cache.contains_key(id) {
            // Refresh LRU position.
            self.access_order.retain(|x| x != id);
            self.access_order.insert(0, id.to_owned());
            self.cache.get(id)
        } else {
            None
        }
    }

    /// Return references to cached memories sorted by cosine similarity
    /// (highest first) with respect to `profile`.
    ///
    /// Memories whose signature similarity to `profile` is exactly `0.0` are
    /// still included (at the tail) so callers always get the full cache
    /// contents.
    #[must_use]
    pub fn query_by_relevance<'a>(&'a self, profile: &MemorySignature) -> Vec<&'a Memory> {
        let mut scored: Vec<(&'a Memory, f64)> = self
            .cache
            .values()
            .map(|m| {
                let sig = compute_signature(&m.content);
                let score = signature_similarity(profile, &sig);
                (m, score)
            })
            .collect();

        scored.sort_by(|(_, sa), (_, sb)| sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Equal));

        scored.into_iter().map(|(m, _)| m).collect()
    }

    /// Return the number of memories currently in the cache.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Return `true` when the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Evict the least-recently-used entry.
    fn evict_lru(&mut self) {
        if let Some(lru_id) = self.access_order.pop() {
            self.cache.remove(&lru_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{EmbeddingStatus, Memory, MemoryType, PipelineStatus, SourceType};

    fn make_memory(id: &str, content: &str) -> Memory {
        Memory {
            id: id.to_owned(),
            title: format!("Memory {id}"),
            content: content.to_owned(),
            summary: String::new(),
            memory_type: MemoryType::Gotcha,
            source_type: SourceType::Manual,
            language: None,
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
            created_by: "user-1".to_owned(),
            anti_memory: None,
        }
    }

    // --- compute_signature ---

    #[test]
    fn signature_detects_file_patterns() {
        let sig = compute_signature("Check src/lib.rs and *.toml files");
        assert!(sig.file_patterns.iter().any(|p| p.contains("lib.rs")));
    }

    #[test]
    fn signature_detects_error_patterns() {
        let sig = compute_signature("Throws TypeError when value is null");
        assert!(sig.error_patterns.iter().any(|p| p == "TypeError"));
    }

    #[test]
    fn signature_detects_sdk_methods() {
        let sig = compute_signature("Call useState and useEffect hooks");
        assert!(sig.sdk_methods.iter().any(|m| m == "useState"));
        assert!(sig.sdk_methods.iter().any(|m| m == "useEffect"));
    }

    // --- signature_similarity ---

    #[test]
    fn identical_signatures_have_similarity_one() {
        let sig = compute_signature("useState useEffect TypeError src/app.ts");
        let sim = signature_similarity(&sig, &sig);
        assert!((sim - 1.0).abs() < 1e-10);
    }

    #[test]
    fn empty_signature_similarity_is_zero() {
        let a = MemorySignature::default();
        let b = MemorySignature::default();
        assert_eq!(signature_similarity(&a, &b), 0.0);
    }

    #[test]
    fn disjoint_signatures_have_similarity_zero() {
        let a = compute_signature("useState useEffect");
        let b = compute_signature("TypeError ENOENT");
        // The two sigs share no tokens, so cosine similarity should be 0.
        let sim = signature_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    // --- SessionHotCache insertion ---

    #[test]
    fn cache_insert_and_get() {
        let mut cache = SessionHotCache::new(10);
        cache.insert(make_memory("m1", "useState hook"));
        assert!(cache.get("m1").is_some());
        assert!(cache.get("missing").is_none());
    }

    #[test]
    fn cache_evicts_lru_at_capacity() {
        let mut cache = SessionHotCache::new(3);
        cache.insert(make_memory("m1", "content 1"));
        cache.insert(make_memory("m2", "content 2"));
        cache.insert(make_memory("m3", "content 3"));
        // m1 is LRU; inserting m4 should evict it.
        cache.insert(make_memory("m4", "content 4"));
        assert_eq!(cache.len(), 3);
        assert!(cache.get("m1").is_none(), "m1 should have been evicted");
        assert!(cache.get("m4").is_some());
    }

    #[test]
    fn cache_refreshes_lru_on_get() {
        let mut cache = SessionHotCache::new(3);
        cache.insert(make_memory("m1", "content 1"));
        cache.insert(make_memory("m2", "content 2"));
        cache.insert(make_memory("m3", "content 3"));
        // Access m1 so it becomes MRU.
        let _ = cache.get("m1");
        // Insert m4 — should evict m2 (now LRU).
        cache.insert(make_memory("m4", "content 4"));
        assert!(cache.get("m1").is_some(), "m1 was accessed, should survive");
        assert!(cache.get("m2").is_none(), "m2 should have been evicted");
    }

    #[test]
    fn reinserting_existing_key_does_not_grow_cache() {
        let mut cache = SessionHotCache::new(5);
        cache.insert(make_memory("m1", "original content"));
        cache.insert(make_memory("m1", "updated content"));
        assert_eq!(cache.len(), 1);
    }

    // --- query_by_relevance ---

    #[test]
    fn query_by_relevance_returns_all_entries() {
        let mut cache = SessionHotCache::new(10);
        cache.insert(make_memory("m1", "useState useEffect"));
        cache.insert(make_memory("m2", "fetchData apiCall"));

        let profile = compute_signature("useState hook");
        let results = cache.query_by_relevance(&profile);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn query_by_relevance_orders_by_similarity() {
        let mut cache = SessionHotCache::new(10);
        // m1 matches the profile closely (same sdk_methods).
        cache.insert(make_memory("m1", "use useState and useEffect hooks here"));
        // m2 is unrelated.
        cache.insert(make_memory("m2", "ENOENT file not found error occurred"));

        let profile = compute_signature("useState and useEffect");
        let results = cache.query_by_relevance(&profile);
        // m1 should rank first.
        assert_eq!(results[0].id, "m1");
    }

    // --- Performance ---

    #[test]
    fn hot_cache_query_50_items_under_50ms() {
        use std::time::Instant;

        let mut cache = SessionHotCache::new(DEFAULT_HOT_CACHE_CAP);
        for i in 0..DEFAULT_HOT_CACHE_CAP {
            cache.insert(make_memory(
                &format!("m{i}"),
                &format!("useState useEffect fetchData error{i} src/file{i}.ts"),
            ));
        }

        let profile = compute_signature("useState useEffect TypeError src/app.ts");

        let start = Instant::now();
        let results = cache.query_by_relevance(&profile);
        let elapsed = start.elapsed();

        assert_eq!(results.len(), DEFAULT_HOT_CACHE_CAP);
        assert!(
            elapsed.as_millis() < 50,
            "query_by_relevance took {}ms, expected <50ms",
            elapsed.as_millis()
        );
    }
}
