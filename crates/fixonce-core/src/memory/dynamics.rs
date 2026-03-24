//! Memory decay and reinforcement functions.
//!
//! Memories naturally lose relevance over time (decay) but can be refreshed
//! through access or explicit positive feedback (reinforcement).

/// Default half-life in days: a memory loses half its score every 30 days.
pub const DEFAULT_HALF_LIFE_DAYS: f64 = 30.0;

/// Score threshold below which a memory is considered stale and eligible for
/// soft-deletion.
pub const DEFAULT_DECAY_THRESHOLD: f64 = 0.1;

/// Compute the decayed score using exponential decay.
///
/// `decay_score = initial_score * 0.5^(days_elapsed / half_life_days)`
///
/// The result is clamped to `[0.0, initial_score]`.
///
/// # Panics
///
/// Does not panic. Returns `0.0` when `half_life_days` is zero or negative to
/// avoid division-by-zero.
#[must_use]
pub fn compute_decay(initial_score: f64, days_elapsed: f64, half_life_days: f64) -> f64 {
    if half_life_days <= 0.0 {
        return 0.0;
    }
    let decayed = initial_score * (0.5_f64).powf(days_elapsed / half_life_days);
    decayed.clamp(0.0, initial_score)
}

/// Apply reinforcement by adding `points` to the current score.
///
/// The result is clamped to `[0.0, 1.0]` because scores are treated as
/// probabilities / normalised relevance weights.
#[must_use]
pub fn apply_reinforcement(current_score: f64, points: f64) -> f64 {
    (current_score + points).clamp(0.0, 1.0)
}

/// Return `true` when the decayed score has fallen below `threshold` and the
/// memory should be soft-deleted.
#[must_use]
pub fn should_soft_delete(decay_score: f64, threshold: f64) -> bool {
    decay_score < threshold
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Decay ---

    #[test]
    fn decay_at_zero_days_is_initial_score() {
        let score = compute_decay(1.0, 0.0, DEFAULT_HALF_LIFE_DAYS);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn decay_at_one_half_life_is_half_initial() {
        let score = compute_decay(1.0, DEFAULT_HALF_LIFE_DAYS, DEFAULT_HALF_LIFE_DAYS);
        assert!((score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn decay_at_two_half_lives_is_quarter_initial() {
        let score = compute_decay(1.0, DEFAULT_HALF_LIFE_DAYS * 2.0, DEFAULT_HALF_LIFE_DAYS);
        assert!((score - 0.25).abs() < 1e-10);
    }

    #[test]
    fn decay_with_non_unit_initial_score() {
        let score = compute_decay(0.8, DEFAULT_HALF_LIFE_DAYS, DEFAULT_HALF_LIFE_DAYS);
        assert!((score - 0.4).abs() < 1e-10);
    }

    #[test]
    fn decay_never_goes_below_zero() {
        let score = compute_decay(1.0, 1_000.0, DEFAULT_HALF_LIFE_DAYS);
        assert!(score >= 0.0);
    }

    #[test]
    fn decay_zero_half_life_returns_zero() {
        let score = compute_decay(1.0, 5.0, 0.0);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn decay_negative_half_life_returns_zero() {
        let score = compute_decay(1.0, 5.0, -10.0);
        assert_eq!(score, 0.0);
    }

    // --- Reinforcement ---

    #[test]
    fn reinforcement_increases_score() {
        let new_score = apply_reinforcement(0.5, 0.2);
        assert!((new_score - 0.7).abs() < 1e-10);
    }

    #[test]
    fn reinforcement_clamps_at_one() {
        let new_score = apply_reinforcement(0.9, 0.5);
        assert_eq!(new_score, 1.0);
    }

    #[test]
    fn reinforcement_clamps_at_zero_for_negative_points() {
        let new_score = apply_reinforcement(0.1, -0.5);
        assert_eq!(new_score, 0.0);
    }

    // --- Soft-delete threshold ---

    #[test]
    fn should_soft_delete_below_threshold() {
        assert!(should_soft_delete(0.05, DEFAULT_DECAY_THRESHOLD));
    }

    #[test]
    fn should_not_soft_delete_above_threshold() {
        assert!(!should_soft_delete(0.5, DEFAULT_DECAY_THRESHOLD));
    }

    #[test]
    fn should_not_soft_delete_at_exact_threshold() {
        assert!(!should_soft_delete(
            DEFAULT_DECAY_THRESHOLD,
            DEFAULT_DECAY_THRESHOLD
        ));
    }
}
