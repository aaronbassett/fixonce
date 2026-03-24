# TUI Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Redesign the FixOnce TUI with a dashboard (hero, activity heatmap, stats, memory list), live search with type switching, memory editing, auth gating, and async data loading.

**Architecture:** Four phases — backend first (activity logging + RPC functions + edge function), then core TUI refactoring (App struct, input model, async loading), then view implementations, then cleanup. Each phase produces testable, deployable artifacts.

**Tech Stack:** Rust + ratatui 0.29, tui-big-text, tokio mpsc channels, Supabase edge functions (Deno/TypeScript), PostgreSQL RPC functions.

**Spec:** `docs/superpowers/specs/2026-03-24-tui-redesign-design.md`

---

## File Structure

### New Files
- `supabase/migrations/20260324120000_dashboard_rpcs.sql` — 4 RPC functions
- `supabase/functions/dashboard-stats/index.ts` — bundled dashboard data endpoint
- `crates/fixonce-core/src/api/dashboard.rs` — Rust API client for dashboard data
- `crates/fixonce-cli/src/tui/views/search.rs` — Search view renderer
- `crates/fixonce-cli/src/tui/views/splash.rs` — Unauthenticated splash screen
- `crates/fixonce-cli/src/tui/widgets/heatmap.rs` — Activity heatmap widget
- `crates/fixonce-cli/src/tui/widgets/mod.rs` — Widget module
- `crates/fixonce-cli/src/tui/data.rs` — Async data loading (channels, DataState)

### Modified Files
- `supabase/functions/memory-create/index.ts` — rename action `memory.create` → `memory.created`
- `supabase/functions/memory-get/index.ts` — add activity logging
- `supabase/functions/memory-search/index.ts` — add activity logging
- `supabase/functions/feedback-submit/index.ts` — rename action `feedback.submit` → `feedback.submitted`
- `crates/fixonce-core/src/api/mod.rs` — add `pub mod dashboard`
- `crates/fixonce-core/src/memory/types.rs` — add dashboard response types
- `crates/fixonce-cli/Cargo.toml` — add `tui-big-text`
- `crates/fixonce-cli/src/tui/app.rs` — major refactor (input mode, nav history, async, new views)
- `crates/fixonce-cli/src/tui/mod.rs` — add `pub mod widgets`, `pub mod data`
- `crates/fixonce-cli/src/tui/views/mod.rs` — add search, splash; remove activity, secrets, health, too_small, memory_list
- `crates/fixonce-cli/src/tui/views/dashboard.rs` — complete rewrite
- `crates/fixonce-cli/src/tui/views/memory_detail.rs` — add `e` key handler
- `crates/fixonce-cli/src/tui/views/create_form.rs` — edit mode, Ctrl+S submit
- `crates/fixonce-cli/src/tui/tests.rs` — rewrite for new view structure
- `crates/fixonce-cli/src/main.rs` — update view names in transaction_name()

### Deleted Files
- `crates/fixonce-cli/src/tui/views/activity.rs`
- `crates/fixonce-cli/src/tui/views/secrets.rs`
- `crates/fixonce-cli/src/tui/views/health.rs`
- `crates/fixonce-cli/src/tui/views/too_small.rs`
- `crates/fixonce-cli/src/tui/views/memory_list.rs`

---

## Phase 1: Backend

### Task 1: Add activity logging to edge functions

**Files:**
- Modify: `supabase/functions/memory-create/index.ts:170` (rename action)
- Modify: `supabase/functions/memory-get/index.ts:100-110` (add logging)
- Modify: `supabase/functions/memory-search/index.ts:225-230` (add logging)
- Modify: `supabase/functions/feedback-submit/index.ts:190` (rename action)

- [ ] **Step 1: Fix memory-create action name**

In `supabase/functions/memory-create/index.ts`, change the `logActivity` call's action from `"memory.create"` to `"memory.created"`:

```typescript
await logActivity(supabase, {
  userId,
  action: "memory.created",  // was "memory.create"
  entityType: "memory",
  entityId: typedData.id,
  metadata: { memory_type: input.memory_type, source_type: input.source_type },
});
```

- [ ] **Step 2: Add logging to memory-get**

In `supabase/functions/memory-get/index.ts`, add activity logging after the successful memory fetch (before the response). Import `logActivity` and `createServiceClient` from the shared module:

```typescript
import { logActivity } from "../_shared/activity.ts";
```

After the successful data fetch, before returning the response:

```typescript
// Log access (non-fatal)
await logActivity(supabase, {
  userId,
  action: "memory.accessed",
  entityType: "memory",
  entityId: input.id,
});
```

- [ ] **Step 3: Add logging to memory-search**

In `supabase/functions/memory-search/index.ts`, add activity logging after a successful search. Import `logActivity`:

```typescript
import { logActivity } from "../_shared/activity.ts";
```

After the RPC call succeeds and before returning results:

```typescript
// Log search (non-fatal)
await logActivity(supabase, {
  userId,
  action: "memory.searched",
  entityType: "search",
  metadata: { search_type: input.search_type, result_count: rows.length },
});
```

- [ ] **Step 4: Fix feedback-submit action name**

In `supabase/functions/feedback-submit/index.ts`, change the action from `"feedback.submit"` to `"feedback.submitted"`.

- [ ] **Step 5: Deploy edge functions**

```bash
cd supabase && supabase functions deploy memory-create memory-get memory-search feedback-submit --no-verify-jwt
```

- [ ] **Step 6: Verify logging works**

Test with curl that activity_log entries are created:

```bash
# Create a memory, then check activity_log via the activity-stream endpoint
curl -s "https://ddbmdvdgvkmwushfmodj.supabase.co/functions/v1/activity-stream?limit=5" \
  -H "apikey: $FIXONCE_ANON_KEY" \
  -H "Authorization: Bearer $(cat ~/.config/fixonce/credentials.json | python3 -c 'import sys,json;print(json.load(sys.stdin)[\"access_token\"])')"
```

Expected: entries with actions `memory.created`, `memory.accessed`, `memory.searched`.

- [ ] **Step 7: Commit**

```bash
git add supabase/functions/
git commit -m "feat(edge): add activity logging to memory-get, memory-search; fix action names"
```

---

### Task 2: Create database migration for dashboard RPC functions

**Files:**
- Create: `supabase/migrations/20260324120000_dashboard_rpcs.sql`

- [ ] **Step 1: Write migration file**

```sql
-- Dashboard RPC functions for the TUI.

-- 1. Aggregate stats: total memories, 24h searches, 24h reports.
CREATE OR REPLACE FUNCTION public.dashboard_stats()
RETURNS TABLE (
    total_memories  bigint,
    searches_24h    bigint,
    reports_24h     bigint
)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT
        (SELECT COUNT(*) FROM public.memory WHERE deleted_at IS NULL),
        (SELECT COUNT(*) FROM public.activity_log
         WHERE action = 'memory.searched'
           AND created_at > now() - interval '24 hours'),
        (SELECT COUNT(*) FROM public.activity_log
         WHERE action = 'feedback.submitted'
           AND created_at > now() - interval '24 hours');
$$;

-- 2. Activity heatmap: daily counts by action for the last N months.
CREATE OR REPLACE FUNCTION public.dashboard_activity_heatmap(months int DEFAULT 6)
RETURNS TABLE (
    day     date,
    action  text,
    count   bigint
)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT
        DATE(created_at) AS day,
        action,
        COUNT(*) AS count
    FROM public.activity_log
    WHERE created_at > now() - (months || ' months')::interval
      AND action IN ('memory.created', 'memory.accessed', 'memory.searched')
    GROUP BY day, action
    ORDER BY day;
$$;

-- 3. Most recently viewed memories (deduplicated).
CREATE OR REPLACE FUNCTION public.dashboard_recent_views(lim int DEFAULT 20)
RETURNS TABLE (
    memory_id   uuid,
    title       text,
    memory_type text,
    decay_score float8,
    last_viewed timestamptz
)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT DISTINCT ON (a.entity_id)
        a.entity_id   AS memory_id,
        m.title,
        m.memory_type,
        m.decay_score,
        a.created_at   AS last_viewed
    FROM public.activity_log a
    JOIN public.memory m ON m.id = a.entity_id
    WHERE a.action = 'memory.accessed'
      AND m.deleted_at IS NULL
    ORDER BY a.entity_id, a.created_at DESC
    LIMIT lim;
$$;

-- 4. Most frequently accessed memories.
CREATE OR REPLACE FUNCTION public.dashboard_most_accessed(lim int DEFAULT 20)
RETURNS TABLE (
    memory_id    uuid,
    title        text,
    memory_type  text,
    decay_score  float8,
    access_count bigint
)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT
        a.entity_id   AS memory_id,
        m.title,
        m.memory_type,
        m.decay_score,
        COUNT(*)       AS access_count
    FROM public.activity_log a
    JOIN public.memory m ON m.id = a.entity_id
    WHERE a.action = 'memory.accessed'
      AND m.deleted_at IS NULL
    GROUP BY a.entity_id, m.title, m.memory_type, m.decay_score
    ORDER BY access_count DESC
    LIMIT lim;
$$;

-- Grant to authenticated users
GRANT EXECUTE ON FUNCTION public.dashboard_stats() TO authenticated;
GRANT EXECUTE ON FUNCTION public.dashboard_activity_heatmap(int) TO authenticated;
GRANT EXECUTE ON FUNCTION public.dashboard_recent_views(int) TO authenticated;
GRANT EXECUTE ON FUNCTION public.dashboard_most_accessed(int) TO authenticated;
```

- [ ] **Step 2: Push migration**

```bash
cd supabase && supabase db push
```

Expected: 1 migration applied.

- [ ] **Step 3: Verify RPC functions exist**

```bash
# Quick test via PostgREST
curl -s "https://ddbmdvdgvkmwushfmodj.supabase.co/rest/v1/rpc/dashboard_stats" \
  -H "apikey: $FIXONCE_ANON_KEY" \
  -H "Authorization: Bearer $(cat ~/.config/fixonce/credentials.json | python3 -c 'import sys,json;print(json.load(sys.stdin)[\"access_token\"])')" \
  -H "Content-Type: application/json" \
  -d '{}'
```

Expected: `[{"total_memories":5,"searches_24h":...,"reports_24h":...}]`

- [ ] **Step 4: Commit**

```bash
git add supabase/migrations/
git commit -m "feat(db): add dashboard RPC functions for stats, heatmap, views"
```

---

### Task 3: Create dashboard-stats edge function

**Files:**
- Create: `supabase/functions/dashboard-stats/index.ts`

- [ ] **Step 1: Create the edge function**

```typescript
/**
 * dashboard-stats — POST /functions/v1/dashboard-stats
 *
 * Bundles all dashboard data into a single response:
 * stats, activity heatmap, recent views, most accessed.
 */
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";

Deno.serve(async (req: Request): Promise<Response> => {
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  if (req.method !== "POST") {
    return errorResponse(405, "METHOD_NOT_ALLOWED", "Only POST requests are accepted.", "Send a POST request.");
  }

  let supabase: Awaited<ReturnType<typeof verifyAuth>>["supabase"];
  try {
    ({ supabase } = await verifyAuth(req));
  } catch (err) {
    const status = (err as Error & { status?: number }).status ?? 401;
    return errorResponse(status, status === 401 ? "UNAUTHORIZED" : "INTERNAL_ERROR",
      (err as Error).message,
      status === 401 ? "Provide a valid Bearer token." : "Contact support.");
  }

  // Fetch all dashboard data in parallel
  const [statsRes, heatmapRes, recentRes, accessedRes] = await Promise.all([
    supabase.rpc("dashboard_stats"),
    supabase.rpc("dashboard_activity_heatmap", { months: 6 }),
    supabase.rpc("dashboard_recent_views", { lim: 20 }),
    supabase.rpc("dashboard_most_accessed", { lim: 20 }),
  ]);

  if (statsRes.error) {
    console.error("dashboard-stats: stats error", statsRes.error);
    return errorResponse(500, "STATS_FAILED", "Failed to fetch stats.", "Retry.");
  }

  const stats = Array.isArray(statsRes.data) ? statsRes.data[0] : statsRes.data;

  return new Response(
    JSON.stringify({
      stats: stats ?? { total_memories: 0, searches_24h: 0, reports_24h: 0 },
      heatmap: heatmapRes.data ?? [],
      recent_views: recentRes.data ?? [],
      most_accessed: accessedRes.data ?? [],
    }),
    { status: 200, headers: { "Content-Type": "application/json", ...corsHeaders } },
  );
});
```

- [ ] **Step 2: Deploy**

```bash
supabase functions deploy dashboard-stats --no-verify-jwt
```

- [ ] **Step 3: Verify**

```bash
curl -s -X POST "https://ddbmdvdgvkmwushfmodj.supabase.co/functions/v1/dashboard-stats" \
  -H "apikey: $FIXONCE_ANON_KEY" \
  -H "Authorization: Bearer $(cat ~/.config/fixonce/credentials.json | python3 -c 'import sys,json;print(json.load(sys.stdin)[\"access_token\"])')" \
  -H "Content-Type: application/json" | python3 -m json.tool
```

Expected: JSON with `stats`, `heatmap`, `recent_views`, `most_accessed` keys.

- [ ] **Step 4: Commit**

```bash
git add supabase/functions/dashboard-stats/
git commit -m "feat(edge): add dashboard-stats bundled endpoint"
```

---

## Phase 2: Core Refactoring

### Task 4: Add tui-big-text dependency and dashboard API types

**Files:**
- Modify: `crates/fixonce-cli/Cargo.toml`
- Create: `crates/fixonce-core/src/api/dashboard.rs`
- Modify: `crates/fixonce-core/src/api/mod.rs`

- [ ] **Step 1: Add tui-big-text dependency**

In `crates/fixonce-cli/Cargo.toml`, add:

```toml
tui-big-text = "0.7"
```

- [ ] **Step 2: Create dashboard API types and client**

Create `crates/fixonce-core/src/api/dashboard.rs`:

```rust
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
```

- [ ] **Step 3: Register module**

In `crates/fixonce-core/src/api/mod.rs`, add:

```rust
pub mod dashboard;
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build --release 2>&1
```

- [ ] **Step 5: Commit**

```bash
git add crates/fixonce-cli/Cargo.toml crates/fixonce-core/src/api/dashboard.rs crates/fixonce-core/src/api/mod.rs
git commit -m "feat: add tui-big-text dep and dashboard API types"
```

---

### Task 5: Refactor App struct — input mode, navigation history, data state

**Files:**
- Modify: `crates/fixonce-cli/src/tui/app.rs`
- Create: `crates/fixonce-cli/src/tui/data.rs`
- Modify: `crates/fixonce-cli/src/tui/mod.rs`

This is the core structural change. The App struct gains input mode tracking, navigation history, form mode, and DataState wrappers.

- [ ] **Step 1: Create data.rs with DataState and async loading**

Create `crates/fixonce-cli/src/tui/data.rs`:

```rust
//! Async data loading for the TUI.
//!
//! Data is fetched in background tasks and delivered to the App via an mpsc channel.

use fixonce_core::{
    api::{dashboard::DashboardData, memories::list_memories, ApiClient},
    memory::types::Memory,
};
use tokio::sync::mpsc;

/// Wrapper for data that may be loading, loaded, or failed.
#[derive(Debug, Clone)]
pub enum DataState<T> {
    Loading,
    Loaded(T),
    Error(String),
}

impl<T: Default> Default for DataState<T> {
    fn default() -> Self {
        Self::Loading
    }
}

impl<T> DataState<T> {
    pub fn as_loaded(&self) -> Option<&T> {
        match self {
            Self::Loaded(data) => Some(data),
            _ => None,
        }
    }
}

/// Messages sent from background tasks to the App event loop.
#[derive(Debug)]
pub enum AppMessage {
    DashboardLoaded(Result<DashboardData, String>),
    MemoriesLoaded(Result<Vec<Memory>, String>),
    SearchResults(Result<fixonce_core::memory::types::SearchMemoryResponse, String>),
    SubmitResult(Result<String, String>), // Ok(memory_id) or Err(message)
}

/// Spawn a background task to fetch dashboard data.
pub fn fetch_dashboard_async(client: ApiClient, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let result = fixonce_core::api::dashboard::fetch_dashboard(&client).await;
        let msg = AppMessage::DashboardLoaded(result.map_err(|e| e.to_string()));
        let _ = tx.send(msg);
    });
}

/// Spawn a background task to fetch recent memories.
pub fn fetch_memories_async(client: ApiClient, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let result = list_memories(&client, 20).await;
        let msg = AppMessage::MemoriesLoaded(result.map_err(|e| e.to_string()));
        let _ = tx.send(msg);
    });
}

/// Spawn a background task to search memories.
pub fn search_memories_async(
    client: ApiClient,
    query: String,
    search_type: String,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let body = serde_json::json!({
            "query_text": query,
            "search_type": search_type,
            "limit": 50,
        });
        let result = async {
            let response = client
                .post_authenticated("/functions/v1/memory-search")?
                .json(&body)
                .send()
                .await?;
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                return Err(fixonce_core::api::ApiError::ServerError { status, body });
            }
            response
                .json::<fixonce_core::memory::types::SearchMemoryResponse>()
                .await
                .map_err(|e| fixonce_core::api::ApiError::UnexpectedResponse(e.to_string()))
        }
        .await;
        let msg = AppMessage::SearchResults(result.map_err(|e| e.to_string()));
        let _ = tx.send(msg);
    });
}
```

- [ ] **Step 2: Register data module**

In `crates/fixonce-cli/src/tui/mod.rs`, add:

```rust
pub mod data;
pub mod widgets;
```

Create empty `crates/fixonce-cli/src/tui/widgets/mod.rs`:

```rust
pub mod heatmap;
```

Create placeholder `crates/fixonce-cli/src/tui/widgets/heatmap.rs`:

```rust
//! Activity heatmap widget — renders a 6-month x 31-day calendar grid.
//! Implemented in Task 11.
```

- [ ] **Step 3: Refactor App struct in app.rs**

Replace the `View` enum, `FormField` enum, and `App` struct. Key changes:

1. New `View` enum — remove Activity, Secrets, Health, MemoryList; add Search.
2. Add `InputMode` enum — `Navigation` and `Input`.
3. Add `FormMode` enum — `Create` and `Edit { memory_id: String }`.
4. Add `SearchType` enum — `Hybrid`, `Fts`, `Vector`.
5. Add `ListMode` enum — `RecentlyCreated`, `RecentlyViewed`, `MostAccessed`.
6. Add `HeatmapMode` enum — `Created`, `Read`, `Searched`.
7. App struct gains: `input_mode`, `previous_view`, `form_mode`, `search_type`, `list_mode`, `heatmap_mode`, `dashboard_data: DataState<DashboardData>`, `search_results: DataState<SearchMemoryResponse>`, `api_client: Option<ApiClient>`, `tx: UnboundedSender<AppMessage>`, `rx: UnboundedReceiver<AppMessage>`.

This is a large change. The full refactored `App` struct, enums, and key handler dispatch should be written as a complete replacement of the top ~460 lines of `app.rs`. The `run_tui` function and event loop at the bottom are updated in a later task.

- [ ] **Step 4: Update key event dispatch**

Rewrite `handle_key_event()` to check `input_mode` first:

```rust
pub fn handle_key_event(&mut self, key: KeyEvent) {
    // Ctrl+C always quits
    if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
        self.should_quit = true;
        return;
    }

    match self.input_mode {
        InputMode::Navigation => self.handle_navigation_key(key),
        InputMode::Input => self.handle_input_key(key),
    }
}
```

In `handle_navigation_key`: global nav (1-4, q) + view-specific dispatch.
In `handle_input_key`: character routing to text fields, Esc exits to Navigation.

- [ ] **Step 5: Update run_tui to be async with channel setup**

```rust
pub async fn run_tui(api_url: &str) -> Result<()> {
    // Check TTY
    if !crossterm::tty::IsTty::is_tty(&io::stdout()) {
        anyhow::bail!("fixonce tui requires an interactive terminal (TTY).");
    }

    // Check auth before entering TUI
    let mgr = TokenManager::new();
    let token = match mgr.load_token() {
        Ok(Some(t)) if !mgr.is_expired(&t) => t,
        _ => {
            // Show splash and exit
            show_unauthenticated_splash()?;
            return Ok(());
        }
    };

    let client = ApiClient::new(api_url)?.with_token(&token);

    // Setup channels
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(api_url.to_owned(), client.clone(), tx.clone(), rx);

    // Kick off initial data fetch
    data::fetch_dashboard_async(client.clone(), tx.clone());
    data::fetch_memories_async(client, tx);

    let result = run_event_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}
```

- [ ] **Step 6: Update event loop to drain channel**

In `run_event_loop`, after polling for key events, drain the mpsc channel:

```rust
// Process any pending async messages
while let Ok(msg) = app.rx.try_recv() {
    app.handle_message(msg);
}
```

The `handle_message` method on App updates `DataState` fields based on the message variant.

- [ ] **Step 7: Verify it compiles**

```bash
cargo build --release 2>&1
```

The TUI won't render correctly yet (views reference old types), but the core structure must compile.

- [ ] **Step 8: Commit**

```bash
git add crates/fixonce-cli/src/tui/ crates/fixonce-core/src/api/
git commit -m "refactor(tui): input mode, nav history, async data loading, new view structure"
```

---

## Phase 3: TUI Views

### Task 6: Unauthenticated splash screen

**Files:**
- Create: `crates/fixonce-cli/src/tui/views/splash.rs`

- [ ] **Step 1: Create splash.rs**

Renders a centered FigLet "FixOnce" logo with rainbow gradient and the login message. Uses `tui-big-text` for the large text. Any keypress sets `should_quit = true`.

The `show_unauthenticated_splash()` function in `app.rs` enters raw mode, renders the splash, waits for one keypress, then restores terminal and returns.

- [ ] **Step 2: Verify it compiles and test manually**

```bash
cargo build --release && ./target/release/fixonce tui
```

Without a valid token, should show the splash and exit on any key.

- [ ] **Step 3: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/splash.rs crates/fixonce-cli/src/tui/app.rs
git commit -m "feat(tui): unauthenticated splash screen with gradient logo"
```

---

### Task 7: Dashboard — Hero Row

**Files:**
- Modify: `crates/fixonce-cli/src/tui/views/dashboard.rs`

- [ ] **Step 1: Rewrite dashboard.rs**

Complete rewrite. Layout structure:

```
Vertical split:
  [Hero Row]        — Length(8)
  [Activity Row]    — Length(12)
  [Memory List]     — Min(0)
  [Status Bar]      — Length(1)

Hero Row horizontal split:
  [Logo]            — Percentage(66)
  [Info Panel]      — Percentage(34)
```

The logo uses `tui-big-text::BigText` with per-character `Style::fg(Color::Rgb(r,g,b))` cycling through the rainbow. The info panel is a `Paragraph` inside a `Block` with borders showing version, OS, user, and MOTD.

- [ ] **Step 2: Verify renders**

```bash
cargo build --release && ./target/release/fixonce login && ./target/release/fixonce tui
```

Should show the hero row at the top. Activity and list areas will be placeholders.

- [ ] **Step 3: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/dashboard.rs
git commit -m "feat(tui): dashboard hero row with gradient logo and info panel"
```

---

### Task 8: Dashboard — Activity heatmap widget

**Files:**
- Modify: `crates/fixonce-cli/src/tui/widgets/heatmap.rs`
- Modify: `crates/fixonce-cli/src/tui/views/dashboard.rs`

- [ ] **Step 1: Implement heatmap widget**

The heatmap widget takes a `&[HeatmapEntry]`, a `HeatmapMode` (which action to show), and renders a 6-row x 31-column grid of colored block characters.

Color levels (matching GitHub):
- Level 0: `Color::Rgb(22, 42, 22)` — `░`
- Level 1: `Color::Rgb(14, 68, 41)` — `▒`
- Level 2: `Color::Rgb(0, 109, 50)` — `▓`
- Level 3: `Color::Rgb(38, 166, 65)` — `█`
- Level 4: `Color::Rgb(57, 211, 83)` — `█`

The widget computes the max count across all days, then maps each day's count to a level (0-4). Month labels on the left, legend at bottom.

Implement as a `ratatui::widgets::Widget` trait impl for a `Heatmap` struct.

- [ ] **Step 2: Wire into dashboard.rs**

In the Activity Row's left half, render the `Heatmap` widget with data from `app.dashboard_data`.

- [ ] **Step 3: Add `[`/`]` key handling in dashboard handler**

```rust
KeyCode::Char('[') => { self.heatmap_mode = self.heatmap_mode.prev(); }
KeyCode::Char(']') => { self.heatmap_mode = self.heatmap_mode.next(); }
```

- [ ] **Step 4: Verify renders with mock data**

- [ ] **Step 5: Commit**

```bash
git add crates/fixonce-cli/src/tui/widgets/ crates/fixonce-cli/src/tui/views/dashboard.rs
git commit -m "feat(tui): activity heatmap widget with 6-month calendar grid"
```

---

### Task 9: Dashboard — Stats panel and memory list

**Files:**
- Modify: `crates/fixonce-cli/src/tui/views/dashboard.rs`

- [ ] **Step 1: Stats panel (right half of Activity Row)**

Render three stats boxes stacked vertically:
- Top: "Total Memories" using `BigText` for the number (green).
- Bottom left: "Searches 24h" — large styled number (cyan).
- Bottom right: "Reports 24h" — large styled number (red).

Data from `app.dashboard_data.as_loaded()`. Show "—" when loading.

- [ ] **Step 2: Memory list (bottom section)**

Render a `List` of memories from the active list mode. Header shows mode name + `[;] prev ['] next` hint.

Mode data sources:
- `RecentlyCreated` → `app.memories` (existing list_memories data)
- `RecentlyViewed` → `app.dashboard_data.recent_views`
- `MostAccessed` → `app.dashboard_data.most_accessed`

Each row: type badge (color-coded), title (truncated), score (decay or access count).

- [ ] **Step 3: Add `;`/`'` key handling**

```rust
KeyCode::Char(';') => { self.list_mode = self.list_mode.prev(); self.selected_index = 0; }
KeyCode::Char('\'') => { self.list_mode = self.list_mode.next(); self.selected_index = 0; }
```

- [ ] **Step 4: Add arrow and Enter handling for memory list**

`↑`/`↓` navigate, `Enter` opens `View::MemoryDetail(id)` with `previous_view = Some(Dashboard)`.

- [ ] **Step 5: Verify full dashboard renders**

- [ ] **Step 6: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/dashboard.rs
git commit -m "feat(tui): dashboard stats panel and switchable memory list"
```

---

### Task 10: Search view

**Files:**
- Create: `crates/fixonce-cli/src/tui/views/search.rs`
- Modify: `crates/fixonce-cli/src/tui/views/mod.rs`
- Modify: `crates/fixonce-cli/src/tui/app.rs`

- [ ] **Step 1: Create search.rs**

Layout:
```
Vertical:
  [Search bar + type pills]  — Length(3)
  [Results list]              — Min(0)
  [Status bar]                — Length(1)
```

Search bar: `Paragraph` with border, showing `app.search_query` + cursor. Type pills rendered as `Span`s with active pill in cyan.

Results: render `app.search_results` as a `List` with:
- Type badge + title on first line
- Summary (dimmed) on second line
- Metadata (decay, language, ID) on third line
- Selected item: cyan left border via `Block` styling

Empty states: "Type a query and press Enter to search" / "No results found" / "Searching..."

- [ ] **Step 2: Register in views/mod.rs**

```rust
pub mod search;
```

Remove: `pub mod memory_list`, `pub mod activity`, `pub mod secrets`, `pub mod health`, `pub mod too_small`.

- [ ] **Step 3: Wire into app.rs event dispatch**

In `handle_navigation_key` for `View::Search`: `↑`/`↓` navigate results, `Enter` opens detail, any printable char enters input mode.

In `handle_input_key` for `View::Search`: chars append to `search_query`, Backspace removes, `Enter` fires search (calls `data::search_memories_async`), `Tab` cycles `search_type`, `Esc` clears and exits to navigation.

- [ ] **Step 4: Wire into render dispatch**

```rust
View::Search => views::search::render(f, app),
```

- [ ] **Step 5: Verify search works end-to-end**

```bash
cargo build --release && ./target/release/fixonce tui
```

Press `2` to switch to Search, type a query, press Enter, see results.

- [ ] **Step 6: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/search.rs crates/fixonce-cli/src/tui/views/mod.rs crates/fixonce-cli/src/tui/app.rs
git commit -m "feat(tui): search view with live API search and type switching"
```

---

### Task 11: Memory Detail — edit key and navigation history

**Files:**
- Modify: `crates/fixonce-cli/src/tui/views/memory_detail.rs`
- Modify: `crates/fixonce-cli/src/tui/app.rs`

- [ ] **Step 1: Add `e` key to memory detail handler**

In the memory detail key handler, add:

```rust
KeyCode::Char('e') => {
    if let View::MemoryDetail(ref id) = self.current_view {
        // Find the memory and pre-populate form fields
        if let Some(memory) = self.memories.iter().find(|m| m.id == *id) {
            self.form_title = memory.title.clone();
            self.form_content = memory.content.clone();
            self.form_summary = memory.summary.clone();
            self.form_memory_type = memory.memory_type.to_string();
            self.form_source = memory.source_type.to_string();
            self.form_language = memory.language.clone().unwrap_or_default();
            self.form_mode = FormMode::Edit { memory_id: id.clone() };
            self.form_field = FormField::Title;
            self.input_mode = InputMode::Input;
            self.navigate_to(View::CreateForm);
        }
    }
}
```

- [ ] **Step 2: Fix Esc to use previous_view**

```rust
KeyCode::Esc | KeyCode::Backspace => {
    let target = self.previous_view.take().unwrap_or(View::Dashboard);
    self.navigate_to(target);
}
```

- [ ] **Step 3: Verify**

Navigate Dashboard → memory → press `e` → form shows pre-populated fields with "Edit Memory" title. Esc goes back to detail, Esc again goes to Dashboard.

- [ ] **Step 4: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/memory_detail.rs crates/fixonce-cli/src/tui/app.rs
git commit -m "feat(tui): memory detail edit key and navigation history"
```

---

### Task 12: Create/Edit form — edit mode and Ctrl+S submission

**Files:**
- Modify: `crates/fixonce-cli/src/tui/views/create_form.rs`
- Modify: `crates/fixonce-cli/src/tui/app.rs`

- [ ] **Step 1: Update create_form.rs title based on form_mode**

```rust
let title = match app.form_mode {
    FormMode::Create => " Create Memory",
    FormMode::Edit { .. } => " Edit Memory",
};
```

- [ ] **Step 2: Implement Ctrl+S submission**

In the create form key handler, replace the hint message with actual API calls:

```rust
if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('s') {
    self.status_message = Some("Saving...".to_owned());
    match &self.form_mode {
        FormMode::Create => {
            // Build create request, spawn async task
            let body = serde_json::json!({
                "title": self.form_title,
                "content": self.form_content,
                "summary": self.form_summary,
                "memory_type": self.form_memory_type,
                "source_type": self.form_source,
                "language": if self.form_language.is_empty() { None } else { Some(&self.form_language) },
            });
            // spawn create task via data module
        }
        FormMode::Edit { memory_id } => {
            // Build update request, spawn async task
        }
    }
    return;
}
```

- [ ] **Step 3: Handle SubmitResult message**

In `handle_message`, on `AppMessage::SubmitResult`:
- Ok: navigate to memory detail, set success status message, refresh dashboard data.
- Err: show error in status bar, stay on form.

- [ ] **Step 4: Verify create and edit flows**

- [ ] **Step 5: Commit**

```bash
git add crates/fixonce-cli/src/tui/views/create_form.rs crates/fixonce-cli/src/tui/app.rs
git commit -m "feat(tui): create/edit form with Ctrl+S async submission"
```

---

## Phase 4: Cleanup

### Task 13: Remove old views and update status bar

**Files:**
- Delete: `views/activity.rs`, `views/secrets.rs`, `views/health.rs`, `views/too_small.rs`, `views/memory_list.rs`
- Modify: `crates/fixonce-cli/src/tui/app.rs` (terminal size handling)
- Modify: `crates/fixonce-cli/src/main.rs` (transaction_name)

- [ ] **Step 1: Delete old view files**

```bash
rm crates/fixonce-cli/src/tui/views/activity.rs
rm crates/fixonce-cli/src/tui/views/secrets.rs
rm crates/fixonce-cli/src/tui/views/health.rs
rm crates/fixonce-cli/src/tui/views/too_small.rs
rm crates/fixonce-cli/src/tui/views/memory_list.rs
```

- [ ] **Step 2: Update MIN_COLS/MIN_ROWS**

```rust
pub const MIN_COLS: u16 = 120;
pub const MIN_ROWS: u16 = 36;
```

Add compact mode degradation in the render dispatch:
- Below 36 rows: skip hero row.
- Below 28 rows: skip activity row.

- [ ] **Step 3: Update status bar**

Add context-sensitive keybinding hints after tab indicators:

```rust
let hints = match app.current_view {
    View::Dashboard => "[/] graph  [;'] list  ↑↓ nav  Enter open",
    View::Search => "Type to search  Tab switch type  ↑↓ nav  Enter open",
    View::CreateForm => "Tab next field  Ctrl+S submit  Esc cancel",
    // ...
};
```

- [ ] **Step 4: Update transaction_name in main.rs**

Remove old view variants, add Search.

- [ ] **Step 5: Verify everything compiles with no dead code**

```bash
cargo build --release 2>&1
```

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "refactor(tui): remove old views, update terminal size, context hints"
```

---

### Task 14: Update tests

**Files:**
- Modify: `crates/fixonce-cli/src/tui/tests.rs`

- [ ] **Step 1: Remove tests for deleted views**

Delete all tests referencing `View::Activity`, `View::Secrets`, `View::Health`, `View::MemoryList`.

- [ ] **Step 2: Update remaining tests for new view structure**

- Navigation tests: update numeric shortcuts (1-4 only, not 1-7).
- Input mode tests: add tests for `q` in input mode does NOT quit, `q` in navigation mode quits.
- Search view tests: typing enters input mode, Esc returns to navigation, Enter triggers search.
- Dashboard tests: `[`/`]` cycles heatmap mode, `;`/`'` cycles list mode.
- Navigation history tests: Detail → Esc goes back to the correct previous view.
- Form mode tests: opening form from detail pre-populates fields.

- [ ] **Step 3: Run tests**

```bash
cargo test -p fixonce-cli 2>&1
```

All tests should pass.

- [ ] **Step 4: Commit**

```bash
git add crates/fixonce-cli/src/tui/tests.rs
git commit -m "test(tui): update test suite for redesigned views and input model"
```

---

## Execution Notes

- **Phase 1 (Tasks 1-3)** is fully independent and can be deployed to Supabase before any Rust work.
- **Phase 2 (Tasks 4-5)** is the structural foundation — nothing in Phase 3 works without it.
- **Phase 3 (Tasks 6-12)** tasks are mostly sequential (each builds on the previous), but Task 6 (splash) is independent.
- **Phase 4 (Tasks 13-14)** is cleanup — do last.
- Login before testing any TUI changes: `./target/release/fixonce login`
- The `FIXONCE_API_URL` and `FIXONCE_ANON_KEY` env vars must be set for all TUI testing.
