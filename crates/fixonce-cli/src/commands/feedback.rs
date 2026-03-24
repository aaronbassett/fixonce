/// `fixonce feedback <id> <rating>` — submit feedback on a memory.
use anyhow::{Context, Result};
use fixonce_core::{
    api::{feedback::submit_feedback, ApiClient},
    auth::token::TokenManager,
    memory::types::FeedbackRating,
};

/// Execute `fixonce feedback`.
///
/// # Errors
///
/// Propagates errors from token loading or the feedback API call.
pub async fn run_feedback(
    api_url: &str,
    memory_id: &str,
    rating: FeedbackRatingArg,
    context: Option<String>,
) -> Result<()> {
    // 1. Load token
    let mgr = TokenManager::new();
    let token = mgr
        .load_token()
        .context("Failed to read authentication token")?
        .context("Not authenticated — run `fixonce login` first")?;

    let client = ApiClient::new(api_url)
        .context("Failed to create API client")?
        .with_token(token);

    // 2. Submit feedback
    let fb = submit_feedback(&client, memory_id, rating.into(), context.as_deref())
        .await
        .context("Failed to submit feedback")?;

    println!("Feedback recorded.");
    println!("  id         : {}", fb.id);
    println!("  memory_id  : {}", fb.memory_id);
    println!("  rating     : {}", fb.rating);

    Ok(())
}

/// Clap-friendly wrapper for [`FeedbackRating`].
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum FeedbackRatingArg {
    Helpful,
    Outdated,
    Damaging,
}

impl From<FeedbackRatingArg> for FeedbackRating {
    fn from(a: FeedbackRatingArg) -> Self {
        match a {
            FeedbackRatingArg::Helpful => FeedbackRating::Helpful,
            FeedbackRatingArg::Outdated => FeedbackRating::Outdated,
            FeedbackRatingArg::Damaging => FeedbackRating::Damaging,
        }
    }
}
