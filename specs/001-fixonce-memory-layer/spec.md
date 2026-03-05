# Feature Specification: fixonce-memory-layer

**Feature Branch**: `feature/fixonce-memory-layer`
**Created**: 2026-03-04
**Last Updated**: 2026-03-04
**Status**: In Progress
**Discovery**: See `discovery/` folder for full context

---

## Problem Statement

LLM coding agents operate in isolated sessions with no persistent memory. Every new session starts from zero — unaware of corrections, gotchas, and lessons from previous sessions. This creates a costly cycle of repeated mistakes, siloed knowledge, and version-sensitive errors that agents can't track. FixOnce is a shared memory layer that captures corrections and discoveries, surfaces them contextually to any agent on the team, and improves over time through reinforcement — turning every mistake into institutional memory.

## Personas

| Persona | Description | Primary Goals |
|---------|-------------|---------------|
| Agent (LLM coding agent) | An LLM-powered coding agent (e.g., Claude Code) working on a codebase | Receive relevant memories before/during work; contribute new memories from corrections and discoveries |
| Human Developer | A developer who works alongside agents and reviews their output | Correct agent mistakes (creating memories); curate and manage the memory store |
| DevRel / Ecosystem Maintainer | Maintains ecosystem tooling (e.g., Midnight Network DevRel team) | Capture institutional knowledge about ecosystem gotchas |

---

## User Scenarios & Testing

<!--
  Stories are ordered by priority (P1 first).
  Each story is independently testable and delivers standalone value.
  Stories may be revised if later discovery reveals gaps - see REVISIONS.md
-->

### Story 1: Memory Storage & Schema ✅ Graduated
**Priority**: P1
**Actor**: System (infrastructure)
**Story**: As the FixOnce system, I need a persistent storage layer that can store, retrieve, filter, and search memories so that all other features have a reliable data foundation.
**Value**: All read/write/search operations depend on a well-designed schema and storage layer.

**Key Decisions**: D1 (local-first app, hosted storage), D5 (authorship tracking), D6 (rich project context), D7 (markdown content), D8 (Supabase), D9 (Voyage AI), D10 (title + memory_type)

#### Schema

| Field | Type | Description |
|-------|------|-------------|
| `id` | uuid (PK) | Unique identifier |
| `title` | text | Short scannable name |
| `content` | text | Full memory in markdown |
| `summary` | text | One-line summary for search results |
| `memory_type` | enum | `guidance`, `anti_pattern` |
| `source_type` | enum | `correction`, `discovery`, `instruction` |
| `created_by` | enum | `ai`, `human`, `human_modified` |
| `source_url` | text? | Link to PR comment, CI run, etc. |
| `tags` | text[] | Freeform tags |
| `language` | text | e.g., `compact`, `typescript` |
| `version_predicates` | jsonb | Version constraints (see Story 4) |
| `project_name` | text? | e.g., `fixonce` |
| `project_repo_url` | text? | e.g., `https://github.com/org/repo` |
| `project_workspace_path` | text? | Local path context |
| `confidence` | float | 0.0–1.0, default 0.5 |
| `surfaced_count` | int | Times returned in queries, default 0 |
| `last_surfaced_at` | timestamptz? | Last query return |
| `enabled` | boolean | Kill switch, default true |
| `created_at` | timestamptz | Auto-set |
| `updated_at` | timestamptz | Auto-updated |
| `embedding` | vector(1024) | Voyage code-3 embeddings |

**Storage**: Supabase Postgres + pgvector (D8). FTS via `tsvector` on `title + content + summary + tags`.

#### Acceptance Scenarios

**Scenario 1: Create a memory record**
- Given valid memory data
- When inserted into the database
- Then all fields are stored correctly and `created_at`/`updated_at` are auto-populated

**Scenario 2: Retrieve a memory by ID**
- Given an existing memory
- When queried by `id`
- Then all fields including `version_predicates` JSON are returned correctly

**Scenario 3: Disable a memory (kill switch)**
- Given an existing memory
- When `enabled` is set to `false`
- Then the memory is excluded from all retrieval queries

**Scenario 4: Full-text search**
- Given memories with various content
- When searching for a keyword
- Then matching memories are returned ranked by relevance using `tsvector`

**Scenario 5: Vector similarity search**
- Given memories with embeddings
- When querying with a vector
- Then the nearest memories are returned ordered by cosine distance

**Scenario 6: Hybrid filtered search**
- Given memories with metadata
- When querying with both a metadata filter (e.g., `language = 'compact'`) and a vector
- Then only matching memories are returned, ordered by vector similarity

**Scenario 7: Update a memory**
- Given an existing memory
- When fields are modified
- Then `updated_at` is refreshed and the embedding is regenerated if `content` changed

### Story 2: Memory Creation (Write Path) ✅ Graduated
**Priority**: P1
**Actor**: Agent or Human Developer
**Story**: As an agent or human, I want to create memories from corrections, discoveries, and explicit instructions so that lessons are captured and available for future sessions.
**Value**: Every correction becomes institutional memory rather than a one-time fix.

**Key Decisions**: D11 (quality gate AI-only, humans bypass), D12 (async embeddings), D13 (LLM-driven duplicate detection with four outcomes), D14 (cheap model via OpenRouter)

#### Write Pipeline

```
Memory submitted
  ├── created_by: human → Store immediately, trigger async embedding
  └── created_by: ai →
        ├── Quality Gate LLM (cheap model via OpenRouter)
        │     ├── Reject (vague, too specific, obvious) → Return rejection reason
        │     └── Accept →
        │           ├── Similarity search against existing memories
        │           │     ├── No similar → Store, trigger async embedding
        │           │     ├── Duplicate → Discard incoming
        │           │     ├── Better version → Replace existing
        │           │     ├── Additional details → Update existing
        │           │     └── Complementary → Merge into new combined memory
        │           └── Store result, trigger async embedding
```

#### Quality Gate Criteria

**Reject when**:
- Too vague (e.g., "be careful with types")
- Too specific to a single line of code with no generalizable lesson
- Duplicates an existing memory (semantic match)
- Trivially obvious to any developer

**Accept when**:
- Actionable — tells the agent what to do or avoid
- Generalizable — applies beyond the specific instance
- Contains a "why" not just a "what"

#### Acceptance Scenarios

**Scenario 1: Agent creates a memory (quality gate passes)**
- Given an agent submits a memory with `source_type: correction` and `created_by: ai`
- When the quality gate LLM evaluates it as actionable, generalizable, and non-duplicate
- Then the memory is stored with all metadata, and embedding generation is triggered async

**Scenario 2: Agent creates a memory (quality gate rejects)**
- Given an agent submits a vague memory like "be careful with types"
- When the quality gate LLM evaluates it
- Then the memory is rejected with a reason, and nothing is stored

**Scenario 3: Human creates a memory (bypasses gate)**
- Given a human submits a memory with `created_by: human`
- When submitted
- Then the memory is stored immediately without quality gate evaluation, and embedding is triggered async

**Scenario 4: Duplicate detected — discard**
- Given a memory exists: "Compact 0.18 doesn't support string concatenation"
- When an agent submits a semantically identical memory
- Then the ingest LLM discards the incoming memory

**Scenario 5: Duplicate detected — replace**
- Given a memory exists with outdated information
- When an agent submits a more accurate version of the same lesson
- Then the ingest LLM replaces the existing memory with the incoming one, preserving the original `created_at`

**Scenario 6: Duplicate detected — update**
- Given a memory exists about a topic
- When an agent submits a memory with additional details on the same topic
- Then the ingest LLM updates the existing memory, incorporating new details

**Scenario 7: Duplicate detected — merge**
- Given two related but distinct memories
- When the ingest LLM determines they complement each other
- Then a new combined memory is created and the originals are disabled

**Scenario 8: Memory is immediately metadata-searchable**
- Given a newly created memory
- When queried by tags, language, or project
- Then it appears in results even before embedding generation completes

**Scenario 9: Memory becomes vector-searchable after embedding**
- Given a newly created memory with pending embedding
- When embedding generation completes async
- Then the memory appears in vector similarity searches

### Story 3: Memory Retrieval (Read Path) ✅ Graduated
**Priority**: P1
**Actor**: Agent (via hooks/MCP) or FixOnce Monitor Agent
**Story**: As an agent, I want relevant memories surfaced at the right moments during my work so that I avoid known pitfalls and follow established patterns.
**Value**: Agents benefit from institutional memory without manual lookup.

**Key Decisions**: D15 (multi-hook integration), D16 (two-tier result budgeting), D17 (Agent Teams monitor pattern)

#### Retrieval Pipeline

```
Context available at hook point
  → Stage 1: Query Rewriting (LLM reformulates context into search queries)
  → Stage 2: Hybrid Search (Supabase: metadata filters + vector similarity)
  → Stage 3: Reranking (LLM consolidates, deduplicates, ranks)
  → Two-tier response:
       ├── Top 5: Full memory content
       └── Next 10-20: Summary + relevancy score + cache key
```

#### Hook Integration Points

| Hook | Context Available | Retrieval Mode | Use Case |
|------|-------------------|----------------|----------|
| SessionStart | Project deps, SDK versions, compiler version | Blocking, project-level | Surface critical version-specific memories |
| UserPromptSubmit | User prompt + project context | Blocking quick check + async deep search (mid-run injection) | Task-specific memories |
| PreToolUse | Tool name + arguments (e.g., file content being written) | Blocking | Check writes against known anti-patterns |
| PostToolUse | Tool result (e.g., written file content) | Blocking | Flag anti-patterns in what was just written |
| Stop | Session context | Blocking | Final check for critical errors before agent halts |
| Agent-initiated | Whatever context agent provides | On-demand via MCP tool | Agent explicitly queries for memories |
| Agent Teams monitor | Watches other agents' activity | Proactive via SendMessage | Continuous monitoring, proactive surfacing |

#### Acceptance Scenarios

**Scenario 1: SessionStart — project-level memories**
- Given a project with `compact@0.18` and `compiler@0.3.2` in dependencies
- When a session starts
- Then critical memories scoped to those versions are surfaced

**Scenario 2: UserPromptSubmit — blocking quick check**
- Given a user asks "implement a voting contract"
- When the prompt is submitted
- Then a fast surface-level check returns immediately relevant memories (sub-second)

**Scenario 3: UserPromptSubmit — async deep search with mid-run injection**
- Given a user asks "implement a voting contract"
- When the blocking check completes
- Then a deeper async search runs and injects additional relevant memories into the running session via `AsyncIterable<SDKUserMessage>`

**Scenario 4: PreToolUse — anti-pattern detection on writes**
- Given an agent is about to write code using a deprecated Compact pattern
- When the Write/Edit tool is invoked
- Then the content is checked against `anti_pattern` memories and a warning is surfaced before the write executes

**Scenario 5: PostToolUse — anti-pattern detection on results**
- Given an agent has just written a file
- When the tool completes
- Then the written content is checked and the agent is told to revise if anti-patterns are detected

**Scenario 6: Stop — final critical check**
- Given an agent is about to halt
- When the Stop hook fires
- Then a check for critical errors/anti-patterns runs against the session's changes

**Scenario 7: Agent-initiated query via MCP**
- Given an agent wants memories about "Compact ledger state management"
- When it calls the FixOnce MCP `query` tool with that context
- Then the three-stage pipeline returns top 5 full memories + summaries for overflow

**Scenario 8: Two-tier result response**
- Given a query matches 15 relevant memories
- When results are returned
- Then the top 5 include full content; the next 10 include summary, relevancy score, and cache key

**Scenario 9: Cache key expansion**
- Given an agent received a summary with cache key `mem_abc123`
- When the agent requests expansion via MCP
- Then the full memory content is returned without re-running the retrieval pipeline

**Scenario 10: Agent Teams monitor — proactive surfacing**
- Given a FixOnce monitor agent is watching a builder agent via Agent Teams
- When the builder agent writes code matching a known anti-pattern
- Then the monitor proactively sends the relevant memory via SendMessage

**Scenario 11: Context gathering for query rewriting**
- Given a hook fires with limited context
- When the retrieval pipeline runs
- Then the agent can supplement with inferred context (SDK versions from package files, compiler version, target network)

### Story 4: Version-Scoped Metadata ✅ Graduated
**Priority**: P1
**Actor**: Agent or System
**Story**: As an agent working on a Midnight project, I want memories filtered by the specific component versions in my environment so that I only receive advice relevant to my setup.
**Value**: Prevents version-inappropriate guidance — what's valid in Compact 0.28 may be wrong in 0.29.

**Key Decisions**: D18 (jsonb with version arrays, OR within AND across)

#### Version Predicate Format

```json
{
  "compact_compiler": ["0.28.0", "0.29.0"],
  "compact_runtime": ["0.14.0"],
  "network": ["preprod", "preview"],
  "midnight_js": ["3.0.0", "3.1.0"]
}
```

- Keys are component names from the Midnight support matrix
- Values are arrays of version strings the memory applies to
- OR logic within a component (any listed version matches)
- AND logic across components (all constrained components must match)
- Missing key = no constraint on that component
- `null` = applies to all versions (same as missing key)
- Query uses JSONB `?` operator with GIN index

#### Supported Component Keys

| Key | Component |
|-----|-----------|
| `network` | Network (preview, preprod) |
| `node` | Midnight Node |
| `compact_compiler` | Compact Compiler |
| `compact_runtime` | Compact Runtime |
| `compact_js` | Compact JS |
| `onchain_runtime` | On-chain Runtime |
| `ledger` | Ledger |
| `wallet_sdk` | Wallet SDK |
| `midnight_js` | Midnight.js |
| `dapp_connector_api` | DApp Connector API |
| `midnight_indexer` | Midnight Indexer |
| `proof_server` | Proof Server |

#### Environment Detection Sources

- `package.json` / `package-lock.json` — SDK and JS library versions
- `compact.toml` or compiler config — Compact compiler version
- Local devnet MCP (`midnight-local-devnet`) — installed tool versions
- Compact CLI — installed Midnight tools and versions

#### Acceptance Scenarios

**Scenario 1: Memory matches environment**
- Given a memory with `{"compact_compiler": ["0.28.0", "0.29.0"]}`
- When the agent's environment has `compact_compiler=0.29.0`
- Then the memory is included in retrieval results

**Scenario 2: Memory does not match environment**
- Given a memory with `{"compact_compiler": ["0.28.0"]}`
- When the agent's environment has `compact_compiler=0.29.0`
- Then the memory is excluded from retrieval results

**Scenario 3: Memory with no version predicates (universal)**
- Given a memory with `version_predicates: null`
- When any agent queries
- Then the memory is always included regardless of environment

**Scenario 4: Memory constrained on multiple components (AND logic)**
- Given a memory with `{"compact_compiler": ["0.29.0"], "network": ["preprod"]}`
- When the agent's environment has `compact_compiler=0.29.0` AND `network=preview`
- Then the memory is excluded (network doesn't match)

**Scenario 5: Memory with unconstrained component**
- Given a memory with `{"compact_compiler": ["0.29.0"]}` (no `network` key)
- When the agent's environment has any network value
- Then the memory matches (missing key = no constraint)

**Scenario 6: Environment detection at SessionStart**
- Given a Midnight project with dependencies in `package.json` and a Compact CLI installed
- When the SessionStart hook fires
- Then the agent's environment versions are detected and used to filter retrieval queries

**Scenario 7: Version predicate set at memory creation**
- Given an agent discovers a gotcha specific to `compact_compiler` 0.29.0
- When the memory is created
- Then the ingest process sets `version_predicates: {"compact_compiler": ["0.29.0"]}`

### Story 5: MCP Server Interface ✅ Graduated
**Priority**: P1
**Actor**: Agent (LLM coding agent)
**Story**: As an agent, I want a set of MCP tools to create, query, expand, update, and provide feedback on memories so that I can programmatically interact with FixOnce during my work.
**Value**: Agents can read from and write to institutional memory without human intervention.

**Key Decisions**: D19 (feedback model replaces disable/flag), D20 (configurable query pipeline + verbosity), D21 (merge list into query)

#### MCP Tools

| Tool | Description |
|------|-------------|
| `fixonce_create_memory` | Submit a new memory (quality gate + dedup for AI; direct store for human) |
| `fixonce_query` | Query memories with configurable pipeline, filters, and result budgeting |
| `fixonce_expand` | Expand a cache key to full memory content |
| `fixonce_get_memory` | Get a specific memory by ID |
| `fixonce_update_memory` | Update an existing memory's fields |
| `fixonce_feedback` | Provide feedback on a memory (tags + suggested action + text) |
| `fixonce_detect_environment` | Scan project files and return detected Midnight component versions |

#### `fixonce_query` Parameters

```
fixonce_query:
  # What to search for
  query: string              # Context or search text

  # Pipeline control
  rewrite: bool = true       # LLM query rewriting
  type: simple|vector|hybrid = hybrid
  rerank: bool = true        # LLM reranking pass

  # Filters
  tags: string[]?
  language: string?
  project_name: string?
  memory_type: guidance|anti_pattern?
  created_after: datetime?
  updated_after: datetime?

  # Result budgeting (one or the other)
  max_results: int = 5
  max_tokens: int?           # Approx token budget (overrides max_results)

  # Result shape
  verbosity: small|medium|large = small
```

#### Verbosity Levels

| Level | Fields Returned |
|-------|----------------|
| `small` | id, title, content, summary, memory_type, relevancy_score |
| `medium` | small + tags, language, version_predicates, created_by, source_type, created_at, updated_at |
| `large` | All fields including source_url, project details, confidence, surfaced_count, feedback summary |

#### `fixonce_feedback` Parameters

```
fixonce_feedback:
  memory_id: uuid
  text: string?                              # Free-text feedback
  tags: enum[]?                              # helpful, not_helpful, damaging, accurate,
                                             # somewhat_accurate, somewhat_inaccurate,
                                             # inaccurate, outdated
  suggested_action: keep|remove|fix?
```

- Memories can accumulate multiple feedback entries from different agents/sessions
- Memories with any `remove` or `fix` feedback are flagged for immediate human review in Web UI
- Memories with negative feedback are de-ranked in query results but still surfaced with a warning

#### Acceptance Scenarios

**Scenario 1: Create memory via MCP**
- Given an agent calls `fixonce_create_memory` with content and metadata
- When the tool executes
- Then the memory goes through the write pipeline (quality gate + dedup) and the agent receives a confirmation with the memory ID or a rejection reason

**Scenario 2: Full pipeline query**
- Given an agent calls `fixonce_query` with defaults (rewrite=true, type=hybrid, rerank=true)
- When the query executes
- Then the three-stage pipeline runs and returns top results at `small` verbosity

**Scenario 3: Simple filtered list (no pipeline)**
- Given an agent calls `fixonce_query` with `rewrite=false, type=simple, rerank=false, tags=["compact"]`
- When the query executes
- Then a simple metadata-filtered list is returned without LLM calls

**Scenario 4: Token-budgeted query**
- Given an agent calls `fixonce_query` with `max_tokens=1000`
- When results are assembled
- Then the total response fits within approximately 1000 tokens, returning as many memories as fit

**Scenario 5: Expand cache key**
- Given an agent received an overflow summary with cache key
- When it calls `fixonce_expand` with the cache key
- Then the full memory content is returned at the requested verbosity

**Scenario 6: Provide positive feedback**
- Given an agent found a memory helpful
- When it calls `fixonce_feedback` with `tags=["helpful", "accurate"], suggested_action=keep`
- Then the feedback is recorded against the memory

**Scenario 7: Provide negative feedback — triggers human review**
- Given an agent found a memory damaging
- When it calls `fixonce_feedback` with `tags=["damaging", "outdated"], suggested_action=remove, text="This pattern causes runtime errors in compiler 0.29.0"`
- Then the feedback is recorded and the memory is flagged for immediate human review in Web UI

**Scenario 8: Querying a memory with negative feedback**
- Given a memory has feedback with `suggested_action=fix`
- When it appears in query results
- Then it is de-ranked and returned with a warning indicating it has been flagged

**Scenario 9: Detect environment**
- Given a Midnight project directory
- When an agent calls `fixonce_detect_environment`
- Then detected component versions are returned as a JSON object matching the version predicate key format

**Scenario 10: Update memory**
- Given an existing memory
- When an agent calls `fixonce_update_memory` with changed fields
- Then the memory is updated and embedding is regenerated if content changed

### Story 6: CLI Interface ✅ Graduated
**Priority**: P2
**Actor**: Human Developer or automation script
**Story**: As a developer, I want a CLI tool to interact with FixOnce from the terminal, hooks, and CI pipelines so that I can create, query, and manage memories without needing the Web UI or MCP.
**Value**: Enables automation, hook integration, and quick manual lookups.

**Key Decisions**: D22 (pipe support, human-readable default, separate servers, detect not env)

#### Commands

```
fixonce create     # Create a memory (flags, stdin, or interactive)
fixonce query      # Query memories (mirrors MCP fixonce_query options)
fixonce get        # Get memory by ID
fixonce update     # Update a memory
fixonce feedback   # Provide feedback on a memory
fixonce detect     # Detect Midnight component versions in current project
fixonce serve      # Start the MCP server
fixonce web        # Start the Web UI server
```

#### Output Modes
- Default: human-readable formatted output
- `--json`: machine-readable JSON (for piping, scripting, CI)

#### Acceptance Scenarios

**Scenario 1: Create memory from flags**
- Given a developer runs `fixonce create --title "..." --content "..." --language compact --source-type instruction`
- Then the memory is created (human-created, bypasses quality gate)

**Scenario 2: Create memory from stdin (pipe)**
- Given a hook script pipes content: `echo "Don't use deprecated ledger syntax" | fixonce create --source-type instruction --language compact --title "Avoid deprecated ledger"`
- Then the memory is created from piped content

**Scenario 3: Query with filters**
- Given a developer runs `fixonce query "ledger state" --language compact --max-results 3`
- Then matching memories are displayed in human-readable format

**Scenario 4: Query with JSON output**
- Given a script runs `fixonce query "ledger state" --json`
- Then results are returned as JSON for machine consumption

**Scenario 5: Query with pipeline control**
- Given a developer runs `fixonce query "voting" --no-rewrite --type simple --no-rerank`
- Then a simple metadata/FTS query runs without LLM calls

**Scenario 6: Get memory by ID**
- Given a developer runs `fixonce get <uuid>`
- Then the full memory is displayed

**Scenario 7: Provide feedback via CLI**
- Given a developer runs `fixonce feedback <uuid> --tags outdated,inaccurate --action fix --text "No longer valid for compiler 0.29"`
- Then the feedback is recorded against the memory

**Scenario 8: Detect environment**
- Given a developer runs `fixonce detect` in a Midnight project directory
- Then detected component versions are displayed

**Scenario 9: Start MCP server**
- Given a developer runs `fixonce serve`
- Then the MCP server starts and listens for agent connections

**Scenario 10: Start Web UI**
- Given a developer runs `fixonce web`
- Then the Web UI server starts and opens in the default browser

### Story 7: Web UI for Memory Management ✅ Graduated
**Priority**: P2
**Actor**: Human Developer
**Story**: As a developer, I want a local web interface to review, create, edit, and curate memories so that I can manage the memory store and respond to flagged items.
**Value**: Humans maintain quality control over institutional memory with full visibility into agent activity.

**Key Decisions**: D11 (duplicate suggestions on create), D19 (feedback-flagged memories), D23 (React + Vite, 6 views)

#### Tech Stack
- React + Vite
- Started via `fixonce web`
- Realtime updates via WebSocket or SSE for activity stream

#### Views

**1. Dashboard**
- Overview stats: total memories, enabled/disabled count, recent creates/updates
- **Flagged memories list (prominent)**: memories with `remove` or `fix` feedback, sorted by urgency
- Quick actions: review, disable, delete flagged items directly from dashboard

**2. Memory Query**
- GUI form mirroring `fixonce_query` parameters: query text, pipeline toggles (rewrite, type, rerank), filters (tags, language, project, memory_type, dates), max results, verbosity
- Results displayed as cards/rows with relevancy scores
- Click through to Memory Detail

**3. Memory Detail**
- Full memory view with all fields editable
- Feedback history (all feedback entries with tags, text, suggested actions)
- Version predicates displayed with component names
- Edit/disable/delete actions
- When editing content, `created_by` changes to `human_modified`

**4. Create Memory**
- Form for all memory fields
- **Live duplicate suggestions** as user types (heavily debounced) — shows similar existing memories
- Created with `created_by: human`, bypasses quality gate

**5. Recent Feedback**
- Filterable list of all feedback entries across memories
- Filter by: feedback tags, suggested action, date range
- Shows both positive and negative feedback
- Click through to Memory Detail

**6. Recent Activity**
- Realtime stream of all FixOnce operations: queries, creates, updates, feedback, environment detections
- Filterable by operation type, timestamp
- Realtime updates (WebSocket/SSE) — new events appear without page refresh
- Shows which agent/session triggered each action

#### Acceptance Scenarios

**Scenario 1: Dashboard shows flagged memories**
- Given memories exist with `remove` or `fix` feedback
- When the dashboard loads
- Then flagged memories are displayed prominently with feedback details and quick actions

**Scenario 2: Query via GUI**
- Given the Memory Query view
- When a user sets filters and runs a query
- Then results match what CLI/MCP would return with the same parameters

**Scenario 3: Edit a memory**
- Given a memory in the Detail view
- When the user edits content and saves
- Then `created_by` changes to `human_modified`, `updated_at` refreshes, and embedding is regenerated

**Scenario 4: Create with duplicate suggestions**
- Given the Create Memory form
- When the user types content
- Then similar existing memories are shown (debounced) to prevent duplicates

**Scenario 5: Disable a memory from dashboard**
- Given a flagged memory on the dashboard
- When the user clicks disable
- Then the memory's `enabled` is set to `false` and it no longer appears in agent queries

**Scenario 6: Delete a memory**
- Given a memory in the Detail view
- When the user clicks delete and confirms
- Then the memory is permanently removed from the database

**Scenario 7: View feedback history**
- Given the Recent Feedback view
- When filtered by `suggested_action=fix`
- Then only feedback entries with fix action are shown, linked to their memories

**Scenario 8: Realtime activity stream**
- Given the Recent Activity view is open
- When an agent creates a memory or runs a query
- Then the event appears in the stream without page refresh

**Scenario 9: Filter activity by type**
- Given the Recent Activity view
- When filtered to show only "create" operations
- Then only memory creation events are displayed

---

## Edge Cases

| ID | Scenario | Handling | Stories Affected |
|----|----------|----------|------------------|

---

## Requirements

### Functional Requirements

| ID | Requirement | Stories | Confidence |
|----|-------------|---------|------------|
| FR-001 | System must store memories with all schema fields in a single Supabase Postgres database | Story 1 | 100% |
| FR-002 | System must support vector similarity search via pgvector with Voyage code-3 1024d embeddings | Story 1 | 100% |
| FR-003 | System must support full-text search via tsvector across title, content, summary, and tags | Story 1 | 100% |
| FR-004 | System must support hybrid queries combining metadata filters with vector similarity in a single SQL query | Story 1 | 100% |
| FR-005 | System must exclude disabled memories (`enabled = false`) from all retrieval queries | Story 1 | 100% |
| FR-006 | System must regenerate embeddings when memory content is updated | Story 1 | 100% |
| FR-007 | AI-created memories must pass LLM quality gate before storage | Story 2 | 100% |
| FR-008 | Human-created memories must bypass quality gate and store immediately | Story 2 | 100% |
| FR-009 | Embedding generation must be async — memory is metadata-searchable immediately, vector-searchable after embedding completes | Story 2 | 100% |
| FR-010 | Ingest LLM must perform similarity search and resolve duplicates with one of four outcomes: discard, replace, update, merge | Story 2 | 100% |
| FR-011 | Quality gate and duplicate detection must use a cheap model via OpenRouter | Story 2 | 100% |
| FR-012 | Retrieval must support five hook integration points: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop | Story 3 | 100% |
| FR-013 | UserPromptSubmit must support blocking quick check + async deep search with mid-run injection via AsyncIterable | Story 3 | 100% |
| FR-014 | Retrieval pipeline must execute three stages: query rewriting, hybrid search, reranking | Story 3 | 100% |
| FR-015 | Results must use two-tier budgeting: top 5 full memories + summaries/scores/cache keys for next 10-20 | Story 3 | 100% |
| FR-016 | Cache keys must allow expanding a summary to full content without re-running the pipeline | Story 3 | 100% |
| FR-017 | PreToolUse and PostToolUse hooks must check written content against anti_pattern memories | Story 3 | 100% |
| FR-018 | Agent Teams monitor agent must proactively surface memories via SendMessage (experimental) | Story 3 | 100% |
| FR-019 | Version predicates must use jsonb with arrays of version strings per component key | Story 4 | 100% |
| FR-020 | Version filtering must use OR logic within components, AND logic across components | Story 4 | 100% |
| FR-021 | Memories with null/missing version_predicates must match all environments | Story 4 | 100% |
| FR-022 | Environment detection must resolve versions from package.json, compact.toml, Compact CLI, and local devnet MCP | Story 4 | 100% |
| FR-023 | Version predicate queries must use GIN-indexed JSONB `?` operator for performance | Story 4 | 100% |
| FR-024 | MCP server must expose 7 tools: create_memory, query, expand, get_memory, update_memory, feedback, detect_environment | Story 5 | 100% |
| FR-025 | fixonce_query must support configurable pipeline (rewrite, type, rerank toggles) | Story 5 | 100% |
| FR-026 | fixonce_query must support result budgeting via max_results OR max_tokens | Story 5 | 100% |
| FR-027 | fixonce_query must support three verbosity levels: small, medium, large | Story 5 | 100% |
| FR-028 | fixonce_feedback must support tags (helpful, not_helpful, damaging, accurate, somewhat_accurate, somewhat_inaccurate, inaccurate, outdated) and suggested actions (keep, remove, fix) | Story 5 | 100% |
| FR-029 | Memories with remove/fix feedback must be flagged for immediate human review in Web UI | Story 5, Story 7 | 100% |
| FR-030 | Memories with negative feedback must be de-ranked but still surfaced with a warning | Story 5 | 100% |
| FR-031 | CLI must mirror MCP tool functionality: create, query, get, update, feedback, detect | Story 6 | 100% |
| FR-032 | CLI create must support both flags and stdin piping for content | Story 6 | 100% |
| FR-033 | CLI output must default to human-readable with `--json` flag for machine consumption | Story 6 | 100% |
| FR-034 | CLI query must support same pipeline control flags as MCP (--no-rewrite, --type, --no-rerank) | Story 6 | 100% |
| FR-035 | `fixonce serve` and `fixonce web` must be separate commands starting independent processes | Story 6 | 100% |
| FR-036 | Web UI must be built with React + Vite, started via `fixonce web` | Story 7 | 100% |
| FR-037 | Dashboard must prominently display flagged memories (those with remove/fix feedback) with quick actions | Story 7 | 100% |
| FR-038 | Memory Query view must provide GUI for all `fixonce_query` parameters (pipeline, filters, budgeting, verbosity) | Story 7 | 100% |
| FR-039 | Create Memory view must show live duplicate suggestions (debounced) as user types | Story 7 | 100% |
| FR-040 | Editing memory content must change `created_by` to `human_modified` | Story 7 | 100% |
| FR-041 | Recent Activity view must support realtime updates via WebSocket/SSE | Story 7 | 100% |
| FR-042 | All FixOnce operations (queries, creates, updates, feedback, detections) must be logged for the activity stream | Story 7 | 100% |

### Key Entities

- **Memory**: The core entity. See Story 1 schema for full field definition.
- **Feedback**: Feedback on a memory from an agent or session. Fields: `id` (uuid), `memory_id` (uuid FK), `text` (text?), `tags` (text[]), `suggested_action` (enum: keep/remove/fix), `created_at` (timestamptz). See Story 5.
- **Activity Log**: Record of all FixOnce operations. Fields: `id` (uuid), `operation` (enum: query/create/update/feedback/detect), `memory_id` (uuid? FK), `details` (jsonb), `created_at` (timestamptz). See Story 7.

---

## Success Criteria

| ID | Criterion | Measurement | Stories |
|----|-----------|-------------|---------|
| SC-001 | Memory CRUD operations work correctly | All 7 acceptance scenarios pass | Story 1 |
| SC-002 | Hybrid search returns relevant results | Filtered vector query returns correct subset ordered by similarity | Story 1 |
| SC-003 | Quality gate filters low-value AI memories | Vague/trivial submissions rejected with reason; actionable ones accepted | Story 2 |
| SC-004 | Duplicate detection prevents redundant memories | Semantically identical submissions are discarded; related ones are merged/updated | Story 2 |
| SC-005 | Human memories stored without delay | Human-created memories queryable by metadata immediately after submission | Story 2 |
| SC-006 | Memories surface at appropriate hook points | Each of the 5 hook types triggers retrieval with correct context and mode | Story 3 |
| SC-007 | Two-tier budgeting respects context window | Top 5 full + overflow summaries returned; cache key expansion works | Story 3 |
| SC-008 | Async mid-run injection delivers results | Deep search results injected via AsyncIterable after blocking check completes | Story 3 |
| SC-009 | Version filtering correctly includes/excludes memories | All 7 version scenarios pass — match, no-match, universal, AND, unconstrained, detection, creation | Story 4 |
| SC-010 | Environment detection resolves versions from project files | SDK, compiler, CLI, and devnet MCP versions detected at SessionStart | Story 4 |
| SC-011 | All 7 MCP tools functional | Each tool completes its described operation correctly | Story 5 |
| SC-012 | Query pipeline is configurable | Toggling rewrite/type/rerank changes pipeline behavior; simple mode skips LLM calls | Story 5 |
| SC-013 | Feedback escalation works | Memories with remove/fix feedback appear flagged in Web UI within 1 session | Story 5, Story 7 |
| SC-014 | CLI commands functional | All 8 operational commands (create, query, get, update, feedback, detect, serve, web) complete successfully | Story 6 |
| SC-015 | CLI pipe support works | Memory created from stdin pipe produces same result as flag-based creation | Story 6 |
| SC-016 | Flagged memories visible on dashboard | Memories with remove/fix feedback appear prominently with quick actions | Story 7 |
| SC-017 | Web UI query matches CLI/MCP results | Same query parameters produce same results across all three interfaces | Story 7 |
| SC-018 | Realtime activity stream updates | New operations appear in activity view without page refresh | Story 7 |
| SC-019 | Duplicate suggestions shown on create | Typing in create form surfaces similar existing memories | Story 7 |

---

## Appendix: Story Revision History

*Major revisions to graduated stories. Full details in `archive/REVISIONS.md`*

| Date | Story | Change | Reason |
|------|-------|--------|--------|
| *No revisions yet* | - | - | - |
