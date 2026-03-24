//! Dashboard data fetching for the TUI.

use serde::Deserialize;
use tracing::instrument;

use super::{ApiClient, ApiError};

/// Aggregate stats from the dashboard endpoint.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DashboardStats {
    pub total_memories: i64,
    pub searches_24h: i64,
    pub reports_24h: i64,
}

/// A single day's activity count for one action type.
#[derive(Debug, Clone, Deserialize)]
pub struct HeatmapEntry {
    pub day: String,
    pub action: String,
    pub count: i64,
}

/// A recently viewed memory summary.
#[derive(Debug, Clone, Deserialize)]
pub struct RecentView {
    pub memory_id: String,
    pub title: String,
    pub memory_type: String,
    pub decay_score: f64,
    pub last_viewed: String,
}

/// A most-accessed memory summary.
#[derive(Debug, Clone, Deserialize)]
pub struct MostAccessed {
    pub memory_id: String,
    pub title: String,
    pub memory_type: String,
    pub decay_score: f64,
    pub access_count: i64,
}

/// Full dashboard response from the edge function.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DashboardData {
    pub stats: DashboardStats,
    #[serde(default)]
    pub heatmap: Vec<HeatmapEntry>,
    #[serde(default)]
    pub recent_views: Vec<RecentView>,
    #[serde(default)]
    pub most_accessed: Vec<MostAccessed>,
}

/// Fetch all dashboard data in one request.
///
/// # Errors
///
/// Returns [`ApiError`] on network failure, authentication problems, or if the
/// server rejects the request.
#[instrument(skip(client))]
pub async fn fetch_dashboard(client: &ApiClient) -> Result<DashboardData, ApiError> {
    let response = client
        .post_authenticated("/functions/v1/dashboard-stats")?
        .json(&serde_json::json!({}))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(ApiError::ServerError { status, body });
    }

    response
        .json::<DashboardData>()
        .await
        .map_err(|e| ApiError::UnexpectedResponse(e.to_string()))
}
