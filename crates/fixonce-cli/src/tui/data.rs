//! Async data loading for the TUI.
//!
//! Uses `tokio::sync::mpsc` channels to send results from background tasks
//! back to the main event loop without blocking the UI.

use fixonce_core::{
    api::{dashboard::DashboardData, ApiClient},
    memory::types::{Memory, SearchMemoryResponse},
};
use tokio::sync::mpsc::UnboundedSender;

// ---------------------------------------------------------------------------
// DataState
// ---------------------------------------------------------------------------

/// Generic loading state for async data.
#[derive(Debug, Clone)]
pub enum DataState<T> {
    /// Data is being fetched.
    Loading,
    /// Data was successfully loaded.
    Loaded(T),
    /// An error occurred while fetching.
    Error(String),
}

impl<T> Default for DataState<T> {
    fn default() -> Self {
        Self::Loading
    }
}

impl<T> DataState<T> {
    /// Return a reference to the inner value if loaded, otherwise `None`.
    pub fn as_loaded(&self) -> Option<&T> {
        match self {
            Self::Loaded(v) => Some(v),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// AppMessage
// ---------------------------------------------------------------------------

/// Messages sent from background tasks to the main event loop.
#[derive(Debug)]
pub enum AppMessage {
    DashboardLoaded(Result<DashboardData, String>),
    MemoriesLoaded(Result<Vec<Memory>, String>),
    SearchResults(Result<SearchMemoryResponse, String>),
    SubmitResult(Result<String, String>),
}

// ---------------------------------------------------------------------------
// Async fetch helpers
// ---------------------------------------------------------------------------

/// Fetch dashboard data in a background task.
pub fn fetch_dashboard_async(client: ApiClient, tx: UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let result = fixonce_core::api::dashboard::fetch_dashboard(&client)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(AppMessage::DashboardLoaded(result));
    });
}

/// Fetch the full memories list in a background task.
pub fn fetch_memories_async(client: ApiClient, tx: UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let result = fixonce_core::api::memories::list_memories(&client, 100)
            .await
            .map_err(|e| e.to_string());
        let _ = tx.send(AppMessage::MemoriesLoaded(result));
    });
}

/// Search memories in a background task.
pub fn search_memories_async(
    client: ApiClient,
    query: String,
    search_type: String,
    tx: UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let body = serde_json::json!({
            "query_text": query,
            "search_type": search_type,
            "limit": 20,
        });
        let result = async {
            let response = client
                .post_authenticated("/functions/v1/memory-search")
                .map_err(|e| e.to_string())?
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(format!("{status}: {body}"));
            }

            response
                .json::<SearchMemoryResponse>()
                .await
                .map_err(|e| e.to_string())
        }
        .await;

        let _ = tx.send(AppMessage::SearchResults(result));
    });
}
