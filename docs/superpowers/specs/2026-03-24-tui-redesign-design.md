# TUI Redesign — Design Spec

## Overview

Redesign the FixOnce TUI with a richer dashboard, live search, memory editing, and proper auth gating. Replaces the current minimal views with a polished terminal experience.

## Unauthenticated State

When the user is not authenticated (no token or expired token), the TUI shows a full-screen splash:

- Centered gradient FigLet "FixOnce" logo
- Message: `Exit and login with 'fixonce login' before launching the TUI`
- Any keypress exits the TUI

No views, tabs, or data fetching. Fail fast with a clear message.

## Terminal Size Requirements

Minimum: **120 columns x 36 rows** (up from 80x24).

Compact degradation for smaller terminals:
- Below 36 rows: hide the Hero Row (logo + info panel), show only activity + stats + memory list.
- Below 28 rows: hide the activity heatmap row entirely, show only stats + memory list.
- Below 120 columns: truncate titles more aggressively, reduce heatmap to 3 months.
- Below 80 columns: show "terminal too small" message (existing behavior).

## Input Focus Model

Views operate in two modes: **navigation mode** (default) and **input mode** (when a text field is focused).

- **Navigation mode**: Global keys (`1`–`4`, `q`, `Ctrl+C`) are active. Character keys route to view-specific handlers.
- **Input mode**: All character keys (including `q`, digits) route to the text field. Only `Ctrl+C` quits. `Esc` exits input mode and returns to navigation mode.

Input mode activates automatically:
- **Search view**: Input mode is active when the search bar has focus (default on entering the view). After pressing `Enter` to search, focus moves to results list (navigation mode). Pressing `/` or any character key returns focus to the search bar (input mode).
- **Create/Edit form**: Always in input mode (existing behavior). `Esc` exits to previous view.
- **Dashboard**: Always in navigation mode (no text inputs).

## Navigation History

The `App` struct tracks a `previous_view: Option<View>` field.

- When navigating from Dashboard → Memory Detail, `previous_view = Some(Dashboard)`.
- When navigating from Search → Memory Detail, `previous_view = Some(Search)`.
- `Esc`/`Backspace` in Memory Detail returns to `previous_view` (defaults to Dashboard).
- When navigating from Memory Detail → Create/Edit Form via `e`, `previous_view` is preserved so Esc from the form goes back to detail, and Esc from detail goes back to the original list.

## View Structure

| Key | View | Purpose |
|-----|------|---------|
| `1` | Dashboard | Hero + activity graph + stats + memory list |
| `2` | Search | Live search with type switching |
| `3` | Create | Memory creation form (existing, minor tweaks) |
| `4` | Keys | Key management (existing) |
| `q` | — | Quit (navigation mode only) |

Removed views: Activity, Secrets, Health. The Secrets view provided admin-only info with no interactive features — dropped entirely. Health stats (total memories, decay averages) are folded into the Dashboard stats panel.

Files to delete: `views/activity.rs`, `views/secrets.rs`, `views/health.rs`, `views/too_small.rs` (replaced by inline size check). Enum variants `View::Activity`, `View::Secrets`, `View::Health` removed. Existing tests for these views removed. `View::MemoryList` removed (replaced by Search).

## Dashboard (Tab 1)

### Layout (top to bottom)

**Hero Row** — 2/3 width logo, 1/3 width info panel

- Left: Gradient FigLet ASCII "FixOnce" logo using `tui-big-text` crate. Characters transition through a rainbow gradient (red → orange → yellow → green → cyan → blue → purple) left to right using 24-bit RGB colors. Fallback for 256-color terminals: use closest named colors.
- Right: Info panel bordered box containing:
  - `Version:` CLI version from `CARGO_PKG_VERSION`
  - `OS:` from `std::env::consts::{OS, ARCH}`
  - `User:` from the JWT `preferred_username` claim (or `sub` as fallback)
  - Separator line
  - `Message of the Day` — static string for now, hardcoded. Future: fetch from API.

**Activity Row** — 50/50 split

- Left: Activity heatmap (6 months x 31 days)
  - 6 rows (months), most recent at bottom. Each row = one month, 31 columns (days). Months with <31 days leave trailing cells empty.
  - Month labels (3-letter abbreviation) on the left axis.
  - Cell intensity: 5 levels using block characters (`█`) with varying green colors (matching GitHub contribution graph palette). Empty/zero cells use a dim background character.
  - Header shows current mode label and `[ ] switch [ ]` hint.
  - `[` cycles backward through modes, `]` cycles forward: **Memories Created** → **Memories Read** → **Searches Made**.
  - Data source: `dashboard_activity_heatmap` RPC function (see Backend section).
  - Legend at bottom right: Less ░▒▓█ More
  - Empty state: Grid structure visible with all cells at lowest intensity, "No activity yet" overlay text.

- Right: Stats panel, stacked vertically
  - Top (large): **Total Memories** — FigLet ASCII number, green. Uses a monospaced FigLet font to maintain consistent width. Source: from `dashboard_stats` RPC.
  - Bottom row, side by side:
    - **Searches 24h** — large plain number, cyan.
    - **Reports 24h** — large plain number, red.
  - Empty state: Show "0" for all stats.

**Memory List** — full width, remaining height

- Header shows current list mode and `[ ; ] prev mode [ ' ] next mode` hint.
- Three modes, cycled with `;` (backward) and `'` (forward):
  - **Recently Created** — from `list_memories` API (existing), `ORDER BY created_at DESC LIMIT 20`
  - **Recently Viewed** — from `dashboard_recent_views` RPC
  - **Most Accessed** — from `dashboard_most_accessed` RPC
- Each row shows: memory type (color-coded badge), title (truncated), decay score (color-coded).
- `↑`/`↓` navigate the list. `Enter` opens Memory Detail view.
- Selected row is highlighted with a distinct background color.
- Empty state: "No memories yet. Create one with `fixonce create` or press 3."

**Status Bar** — bottom row, full width

- Left: Tab indicators with numbered shortcuts (active tab highlighted in cyan). Context-sensitive hint for the active view shown after tabs (e.g., `[/] graph mode  [;'] list mode  ↑↓ navigate`).
- Right: `fixonce v{version}`

## Search (Tab 2)

Renamed from "List". Performs API searches on submit.

### Layout

**Search Bar** — top row

- Left: Text input with cursor indicator. In input mode: typing appends, Backspace removes, Esc exits to navigation mode. `Enter` executes search and moves focus to results.
- Right (inline): Search type pills — `Hybrid` | `FTS` | `Vector`. Active type highlighted in cyan, others dimmed. `Tab` cycles to next search type (no conflict with text input since there's only one field). Vector search is shown but disabled in v1 (displays "(requires embedding)" hint when selected, falls back to Hybrid).

**Results List** — remaining height

- Populated by calling the `memory-search` edge function with `query_text` and `search_type`.
- Search fires on `Enter` while in input mode.
- In navigation mode (results focused): `Enter` opens Memory Detail for the selected result. `↑`/`↓` navigate results. Typing any character returns to input mode.
- Each result row shows:
  - Memory type badge (color-coded)
  - Title
  - Relevance/RRF score (right-aligned, cyan)
  - Summary (dimmed, below title)
  - Metadata line: decay score, language, truncated ID
- Selected row: cyan left border accent + highlighted background.
- Empty state (no query yet): "Type a query and press Enter to search".
- Empty state (no results): "No results found for '{query}'".
- Loading state: "Searching..." with spinner.

**Status Bar** — same pattern as Dashboard, with search-specific hints.

## Memory Detail

Existing scrollable detail view with these additions:

- **`e` key** opens the memory for editing — navigates to the Create Form view with all fields pre-populated from the current memory. The form switches to "edit mode."
- `Esc`/`Backspace` returns to `previous_view` (Dashboard or Search, whichever opened it).
- All other keybindings unchanged: `↑`/`↓`/`j`/`k` scroll.

## Create/Edit Form

Existing form with these changes:

- `App` stores `form_mode: FormMode` enum (`Create` | `Edit { memory_id: String }`).
- When opened via `e` from detail view: fields pre-populated, title bar shows "Edit Memory", `Ctrl+S` calls `memory-update` edge function.
- When opened via Tab 3: fields empty, title bar shows "Create Memory", `Ctrl+S` calls `memory-create` edge function.
- `Enter` inserts newline in Content and Summary fields (already implemented).
- Submit lifecycle:
  - On `Ctrl+S`: show "Saving..." status message, make API call.
  - On success: navigate to Memory Detail for the created/updated memory, show "Memory saved" status.
  - On error: show error in status bar, keep form populated for retry.
- `Esc` cancels and returns to previous view.

## Data Loading Architecture

The TUI uses **async message passing** for data loading to avoid blocking the UI.

- `App` receives data via a `tokio::sync::mpsc` channel.
- On startup and view transitions, data fetch tasks are spawned with `tokio::spawn`.
- The event loop checks the channel on each tick (250ms) and updates `App` state when data arrives.
- Loading states: each data section in `App` is wrapped in an enum: `DataState<T> { Loading, Loaded(T), Error(String) }`.
- Dashboard data is fetched once on startup and refreshed:
  - After any form submission (create/edit).
  - When switching back to Dashboard from another tab.
  - On a 60-second periodic timer.

## Backend Prerequisites

### Activity Log Inserts (edge function changes)

These edge functions must add `logActivity` calls:

| Edge Function | Action String | Entity Type | Notes |
|---------------|--------------|-------------|-------|
| `memory-create` | `memory.created` | `memory` | Currently logs `memory.create` — rename to `memory.created` |
| `memory-get` | `memory.accessed` | `memory` | Currently has no logging — add it |
| `memory-search` | `memory.searched` | `search` | Currently has no logging — add it |
| `feedback-submit` | `feedback.submitted` | `feedback` | Verify current action string matches |

### New Postgres RPC Functions

**`dashboard_stats()`** — returns a single row:

```sql
CREATE OR REPLACE FUNCTION public.dashboard_stats()
RETURNS TABLE (
    total_memories  bigint,
    searches_24h    bigint,
    reports_24h     bigint
)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT
        (SELECT COUNT(*) FROM memory WHERE deleted_at IS NULL),
        (SELECT COUNT(*) FROM activity_log WHERE action = 'memory.searched' AND created_at > now() - interval '24 hours'),
        (SELECT COUNT(*) FROM activity_log WHERE action = 'feedback.submitted' AND created_at > now() - interval '24 hours');
$$;
```

**`dashboard_activity_heatmap(months int DEFAULT 6)`** — returns daily counts:

```sql
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
    FROM activity_log
    WHERE created_at > now() - (months || ' months')::interval
      AND action IN ('memory.created', 'memory.accessed', 'memory.searched')
    GROUP BY day, action
    ORDER BY day;
$$;
```

**`dashboard_recent_views(lim int DEFAULT 20)`** — most recently viewed memories:

```sql
CREATE OR REPLACE FUNCTION public.dashboard_recent_views(lim int DEFAULT 20)
RETURNS TABLE (memory_id uuid, title text, memory_type text, decay_score float8, last_viewed timestamptz)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT DISTINCT ON (a.entity_id)
        a.entity_id AS memory_id,
        m.title,
        m.memory_type,
        m.decay_score,
        a.created_at AS last_viewed
    FROM activity_log a
    JOIN memory m ON m.id = a.entity_id
    WHERE a.action = 'memory.accessed'
      AND m.deleted_at IS NULL
    ORDER BY a.entity_id, a.created_at DESC
    LIMIT lim;
$$;
```

**`dashboard_most_accessed(lim int DEFAULT 20)`** — most frequently accessed memories:

```sql
CREATE OR REPLACE FUNCTION public.dashboard_most_accessed(lim int DEFAULT 20)
RETURNS TABLE (memory_id uuid, title text, memory_type text, decay_score float8, access_count bigint)
LANGUAGE sql STABLE SECURITY INVOKER AS $$
    SELECT
        a.entity_id AS memory_id,
        m.title,
        m.memory_type,
        m.decay_score,
        COUNT(*) AS access_count
    FROM activity_log a
    JOIN memory m ON m.id = a.entity_id
    WHERE a.action = 'memory.accessed'
      AND m.deleted_at IS NULL
    GROUP BY a.entity_id, m.title, m.memory_type, m.decay_score
    ORDER BY access_count DESC
    LIMIT lim;
$$;
```

Grant `EXECUTE` on all four functions to `authenticated`.

### New Edge Function

**`dashboard-stats`** — wraps the RPC calls above into a single endpoint:

```
POST /functions/v1/dashboard-stats
Authorization: Bearer <token>

Response 200:
{
  "stats": { "total_memories": 132, "searches_24h": 47, "reports_24h": 2 },
  "heatmap": [ { "day": "2026-03-01", "action": "memory.created", "count": 5 }, ... ],
  "recent_views": [ { "memory_id": "...", "title": "...", ... } ],
  "most_accessed": [ { "memory_id": "...", "title": "...", ... } ]
}
```

This bundles all dashboard data into one request to minimize latency.

## Dependencies

New Rust crate dependencies:

- `tui-big-text` — FigLet-style large text rendering for ratatui (by ratatui maintainer)

## Keybinding Summary

### Global (navigation mode only)
| Key | Action |
|-----|--------|
| `1`–`4` | Switch to tab |
| `q`, `Ctrl+C` | Quit |

### Global (input mode)
| Key | Action |
|-----|--------|
| `Ctrl+C` | Quit |
| `Esc` | Exit input mode / cancel |

### Dashboard (navigation mode)
| Key | Action |
|-----|--------|
| `[` / `]` | Cycle activity graph mode |
| `;` / `'` | Cycle memory list mode |
| `↑` / `↓` | Navigate memory list |
| `Enter` | Open selected memory detail |

### Search
| Key | Mode | Action |
|-----|------|--------|
| Type | input | Append to search query |
| `Backspace` | input | Delete last character |
| `Enter` | input | Execute search, switch to navigation |
| `Tab` | input | Cycle search type (Hybrid → FTS → Vector) |
| `Esc` | input | Clear search, switch to navigation |
| `↑` / `↓` | navigation | Navigate results |
| `Enter` | navigation | Open selected memory detail |
| Any char | navigation | Return to input mode with that character |

### Memory Detail (navigation mode)
| Key | Action |
|-----|--------|
| `↑` / `↓` / `j` / `k` | Scroll |
| `e` | Edit this memory |
| `Esc` / `Backspace` | Go back to previous view |

### Create/Edit Form (input mode)
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next/prev field |
| `Enter` | Newline (Content/Summary only) |
| `Ctrl+S` | Submit (create or update) |
| `Esc` | Cancel, return to previous view |
