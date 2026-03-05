# Implementation Plan: fixonce-memory-layer

**Branch**: `001-fixonce-memory-layer` | **Date**: 2026-03-04 | **Spec**: `specs/001-fixonce-memory-layer/spec.md`
**Input**: Feature specification from `discovery/SPEC.md` (7 stories, 42 functional requirements, 23 design decisions)

## Summary

FixOnce is a shared memory layer for LLM coding agents. It captures corrections and discoveries as structured memories, stores them in Supabase (Postgres + pgvector), and surfaces them contextually through Claude Code hooks, MCP tools, CLI, and a Web UI. The system has two core pipelines: a **write path** (quality gate + 4-outcome duplicate detection + async embedding) and a **read path** (LLM query rewriting + hybrid search via Reciprocal Rank Fusion + LLM reranking with two-tier result budgeting). Version-scoped metadata ensures memories are filtered by the Midnight component versions in the agent's environment. Three interfaces (MCP server, CLI, Web UI) share a single service layer per the API-First constitution principle.

## Technical Context

**Language/Version**: TypeScript (strict mode), targeting Node.js 20+ (server/CLI) and modern browsers (Web UI)
**Primary Dependencies**:
- `@modelcontextprotocol/sdk` — MCP server with Zod v4 schema validation
- `@anthropic-ai/claude-agent-sdk` — Claude Code hooks (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop)
- `@supabase/supabase-js` v2+ — Supabase Postgres client
- `voyageai` — Voyage AI `voyage-code-3` embeddings (1024 dimensions)
- `openai` — OpenRouter API client (OpenAI-compatible; `google/gemma-3-4b-it` for cheap tasks, `anthropic/claude-3.5-haiku` for nuanced tasks)
- `react` v19 + `vite` — Web UI framework
- `zod` v4 — Schema validation (MCP peer dependency, shared across all modules)

**Storage**: Supabase Postgres + pgvector (D8). Single store for metadata, FTS (tsvector), and vector search (HNSW index, cosine distance). Hybrid search via SQL RPC function using Reciprocal Rank Fusion.

**Testing**: Vitest for unit/integration tests. Integration tests against real Supabase for storage and search operations.

**Target Platform**: Node.js server (MCP + CLI), local browser (Web UI)

**Project Type**: pnpm monorepo with Turborepo. `packages/` for shared libraries, `apps/` for entry points.

**Realtime**: Server-Sent Events (SSE) for the Web UI activity stream (one-way server-to-client, built-in auto-reconnect).

**Constraints**:
- UserPromptSubmit blocking check must be sub-second
- SessionStart can take 1-2 seconds
- Context window budget respected via two-tier result budgeting (top 5 full + overflow summaries)
- All credentials via environment variables: `SUPABASE_URL`, `SUPABASE_ANON_KEY`, `VOYAGE_API_KEY`, `OPENROUTER_API_KEY`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

### Principle I: API-First Design — PASS

- MCP tool schemas (7 tools) defined in `contracts/mcp-tools.md` BEFORE implementation
- CLI maps 1:1 to MCP tools: 7 MCP tools → 7 CLI equivalents + `fixonce expand` added to close gap identified in compliance review + 2 infrastructure commands (`serve`, `web`)
- Web UI calls same service layer via HTTP API (11 endpoints wrapping service functions)
- Shared Zod schemas in `@fixonce/shared` serve as single source of truth

### Principle II: Modularity — PASS

Six separate modules with explicit interfaces:
- `@fixonce/shared` — Types, Zod schemas, enums, constants (zero runtime deps on other modules)
- `@fixonce/storage` — Supabase client, queries, migrations, embedding generation
- `@fixonce/pipeline` — Write path (quality gate, dedup), Read path (rewrite, search, rerank), OpenRouter client
- `@fixonce/activity` — Activity logging (cross-cutting, consumed by all operations)
- `apps/mcp-server` — MCP tool registration, stdio transport
- `apps/cli` — Command definitions, argument parsing, output formatting
- `apps/web` — React frontend + Express/Fastify backend with HTTP API + SSE

No circular dependencies: `shared` ← `storage` ← `pipeline` ← `activity` ← `apps/*`

### Principle III: KISS & YAGNI — PASS

Deferred features remain deferred:
- ~~Reinforcement scoring~~ (post-v1)
- ~~Contradiction detection~~ (post-v1)
- ~~Memory clustering~~ (post-v1)
- ~~Team scoping~~ (post-v1, D3)
- ~~Bulk operations~~ (post-v1, D23)
- ~~Exportable memory packs~~ (post-v1)

No abstract providers for Supabase, Voyage AI, or OpenRouter (single implementation each). No pluggable pipeline stages (spec defines exactly 3 fixed stages). No premature caching layers.

### Principle IV: Test Critical Paths — PASS

MUST test (integration tests against real Supabase):
- Retrieval pipeline: query rewriting → hybrid search → reranking
- Write pipeline: quality gate accept/reject + 4-outcome duplicate detection
- Version predicate filtering: all 7 scenarios from Story 4
- MCP tool contracts: all 7 tools accept valid input, reject invalid input

SHOULD test:
- CLI argument parsing and validation
- Web UI data fetching
- Two-tier result budgeting + cache key expansion

### Principle V: Fail Fast with Actionable Errors — PASS

Structured error type: `{ stage, reason, suggestion }` defined in service layer contracts. Every pipeline stage wraps errors with stage identification. External API failures include actionable suggestions (check API key, check connectivity). No swallowed exceptions in async ops (embedding generation, async retrieval).

### Principle VI: Validate at System Boundaries — PASS

- MCP: Zod schemas validate all tool inputs before service layer calls
- CLI: Argument parsing validates flags before service layer calls
- Web UI: HTTP API validates request bodies
- Internal module calls: NO redundant validation

### Principle VII: Protect Secrets — PASS

All credentials via env vars. Upstream API error messages sanitized before surfacing. Quality gate includes credential pattern check on memory content.

### Principle VIII: Semantic Versioning — PASS

Version bump triggers documented. MCP schema changes = MAJOR. New tools/optional params = MINOR.

### Principle IX: Conventional Commits — PASS

Scopes defined: `mcp`, `cli`, `web`, `pipeline`, `storage`, `schema`, `activity`. Pre-commit hook enforces format.

## Project Structure

### Documentation (this feature)

```text
specs/001-fixonce-memory-layer/
├── spec.md                  # Feature specification (from discovery/SPEC.md)
├── plan.md                  # This file
├── spec-analysis.md         # Spec deep-dive synthesis
├── constitution-compliance.md # Constitution compliance framework
├── research.md              # Phase 0 output — all technical unknowns resolved
├── data-model.md            # Phase 1 output — entities, indexes, state machines
├── quickstart.md            # Phase 1 output — dev setup instructions
├── contracts/
│   ├── mcp-tools.md         # All 7 MCP tool schemas
│   ├── cli-commands.md      # All 9 CLI command specs
│   └── service-layer.md     # Service layer TypeScript interfaces
└── tasks.md                 # Generated by /sdd:tasks command
```

### Source Code (repository root)

```text
fixonce/
├── package.json              # Root: pnpm workspace config
├── pnpm-workspace.yaml       # packages/* and apps/*
├── turbo.json                # Turborepo task pipeline config
├── tsconfig.base.json        # Shared TypeScript config (strict: true)
├── .env.example              # Template for required env vars
├── .gitignore                # Includes .env*
│
├── packages/
│   ├── shared/               # @fixonce/shared — Types, schemas, constants
│   │   ├── src/
│   │   │   ├── index.ts      # Barrel export
│   │   │   ├── schema.ts     # Zod schemas for Memory, Feedback, ActivityLog
│   │   │   ├── enums.ts      # All enum types and constants
│   │   │   ├── types.ts      # TypeScript interfaces (Memory, Feedback, etc.)
│   │   │   ├── errors.ts     # ServiceError type and error factory
│   │   │   └── version-keys.ts # 12 Midnight component keys + VersionPredicates type
│   │   ├── package.json
│   │   └── tsconfig.json
│   │
│   ├── storage/              # @fixonce/storage — Supabase client, queries, embeddings
│   │   ├── src/
│   │   │   ├── index.ts      # Public API: createStorage()
│   │   │   ├── client.ts     # Supabase client creation (env var validation)
│   │   │   ├── memories.ts   # CRUD operations for memories table
│   │   │   ├── feedback.ts   # CRUD for feedback table
│   │   │   ├── activity.ts   # Append-only activity log operations
│   │   │   ├── search.ts     # Hybrid search, FTS, vector search (calls Supabase RPC)
│   │   │   ├── embeddings.ts # Voyage AI client, async embedding generation
│   │   │   └── version-filter.ts # Version predicate query builder
│   │   ├── migrations/       # SQL migration files
│   │   │   ├── 001_extensions.sql
│   │   │   ├── 002_enums.sql
│   │   │   ├── 003_memories.sql
│   │   │   ├── 004_feedback.sql
│   │   │   ├── 005_activity_log.sql
│   │   │   ├── 006_indexes.sql
│   │   │   ├── 007_fts_column.sql
│   │   │   ├── 008_triggers.sql
│   │   │   └── 009_hybrid_search_rpc.sql
│   │   ├── package.json
│   │   └── tsconfig.json
│   │
│   ├── pipeline/             # @fixonce/pipeline — Write + Read path business logic
│   │   ├── src/
│   │   │   ├── index.ts      # Public API: createPipeline()
│   │   │   ├── write/
│   │   │   │   ├── index.ts
│   │   │   │   ├── quality-gate.ts    # LLM quality evaluation (OpenRouter)
│   │   │   │   ├── duplicate-detection.ts # Similarity search + LLM 4-outcome dedup
│   │   │   │   └── credential-check.ts # Reject memories containing credential patterns
│   │   │   ├── read/
│   │   │   │   ├── index.ts
│   │   │   │   ├── query-rewriter.ts  # LLM query reformulation
│   │   │   │   ├── reranker.ts        # LLM consolidation, dedup, ranking
│   │   │   │   └── cache.ts           # In-memory cache for overflow results
│   │   │   ├── llm.ts                 # OpenRouter client wrapper (model config per task)
│   │   │   └── service.ts             # Service layer: 7 functions matching MCP tools
│   │   ├── package.json
│   │   └── tsconfig.json
│   │
│   └── activity/             # @fixonce/activity — Activity logging (cross-cutting)
│       ├── src/
│       │   ├── index.ts      # Public API: logActivity()
│       │   └── stream.ts     # SSE event emitter for Web UI realtime
│       ├── package.json
│       └── tsconfig.json
│
├── apps/
│   ├── mcp-server/           # MCP server entry point (Story 5)
│   │   ├── src/
│   │   │   ├── index.ts      # McpServer + 7 tool registrations + stdio transport
│   │   │   └── tools/        # One file per tool (delegates to service layer)
│   │   │       ├── create-memory.ts
│   │   │       ├── query.ts
│   │   │       ├── expand.ts
│   │   │       ├── get-memory.ts
│   │   │       ├── update-memory.ts
│   │   │       ├── feedback.ts
│   │   │       └── detect-environment.ts
│   │   ├── package.json
│   │   └── tsconfig.json
│   │
│   ├── cli/                  # CLI entry point (Story 6)
│   │   ├── src/
│   │   │   ├── index.ts      # CLI entrypoint, command registration
│   │   │   ├── commands/     # One file per command (delegates to service layer)
│   │   │   │   ├── create.ts
│   │   │   │   ├── query.ts
│   │   │   │   ├── expand.ts
│   │   │   │   ├── get.ts
│   │   │   │   ├── update.ts
│   │   │   │   ├── feedback.ts
│   │   │   │   ├── detect.ts
│   │   │   │   ├── serve.ts
│   │   │   │   └── web.ts
│   │   │   └── formatters/   # Human-readable output formatting
│   │   │       └── index.ts
│   │   ├── package.json      # fixonce (bin: "fixonce")
│   │   └── tsconfig.json
│   │
│   └── web/                  # Web UI (Story 7)
│       ├── src/
│       │   ├── main.tsx
│       │   ├── App.tsx       # Router setup (6 views)
│       │   ├── api/          # HTTP client for backend API
│       │   │   └── client.ts
│       │   ├── components/   # Shared UI components
│       │   ├── views/
│       │   │   ├── Dashboard.tsx        # Stats + flagged memories + quick actions
│       │   │   ├── MemoryQuery.tsx      # Query form + results
│       │   │   ├── MemoryDetail.tsx     # View/edit + feedback history
│       │   │   ├── CreateMemory.tsx     # Form + live duplicate suggestions
│       │   │   ├── RecentFeedback.tsx   # Filterable feedback list
│       │   │   └── RecentActivity.tsx   # Realtime stream (SSE)
│       │   └── hooks/
│       │       └── useActivityStream.ts # SSE EventSource hook
│       ├── server/            # Web backend
│       │   ├── index.ts       # HTTP server + SSE endpoint
│       │   └── routes.ts      # 11 API routes wrapping service layer
│       ├── package.json
│       ├── vite.config.ts
│       └── tsconfig.json
│
├── tests/                     # Integration tests (against real Supabase)
│   ├── pipeline/
│   │   ├── write-pipeline.test.ts
│   │   └── read-pipeline.test.ts
│   ├── storage/
│   │   ├── memories.test.ts
│   │   ├── search.test.ts
│   │   └── version-filter.test.ts
│   └── mcp/
│       └── tool-contracts.test.ts
│
└── .claude/
    └── settings.json          # Claude Code hook registration
```

**Structure Decision**: pnpm workspaces + Turborepo monorepo. Packages (`shared`, `storage`, `pipeline`, `activity`) are internal libraries. Apps (`mcp-server`, `cli`, `web`) are entry points. The `@fixonce/pipeline/service.ts` is the single service layer consumed by all three apps (Constitution Principle I). Dependency flow is strictly unidirectional: `shared` ← `storage` ← `pipeline` ← `activity` ← `apps/*`.

## Implementation Phases

### Phase 1: Foundation — Storage Layer + Schema (Story 1)

**Goal**: Establish the database schema, Supabase client, and all storage operations so that subsequent phases have a reliable data foundation.

**Tasks**:
1. Initialize monorepo structure (pnpm workspace, Turborepo, tsconfig.base.json)
2. Create `@fixonce/shared` package with all TypeScript types, Zod schemas, enums
3. Set up Supabase project and write SQL migrations:
   - Extensions: `vector`, `uuid-ossp`
   - Enums: `memory_type`, `source_type`, `created_by`, `feedback_tag`, `suggested_action`, `operation_type`
   - Tables: `memory`, `feedback`, `activity_log`
   - Indexes: GIN (tsvector, JSONB, tags), HNSW (pgvector cosine), btree (partial on enabled)
   - FTS column: Weighted tsvector generated from title (A) + summary (B) + content (C) + tags (D)
   - Triggers: `updated_at` auto-refresh
   - RPC: `hybrid_search` function using Reciprocal Rank Fusion
4. Create `@fixonce/storage` package:
   - Supabase client creation with env var validation (fail fast if missing)
   - Memory CRUD operations
   - Feedback CRUD operations
   - Activity log append operations
   - Hybrid search function (calls Supabase RPC)
   - Version predicate query builder
5. Create `@fixonce/storage` embedding module:
   - Voyage AI client (`voyageai` npm package)
   - Async embedding generation (fire-and-forget from caller's perspective)
   - Uses `input_type: "document"` for writes, `"query"` for searches
6. Write integration tests against real Supabase:
   - Memory CRUD (all 7 scenarios from Story 1)
   - FTS search
   - Vector similarity search
   - Hybrid filtered search
   - Version predicate filtering (all 7 scenarios from Story 4)

**Acceptance**: All Story 1 acceptance scenarios pass. Memories are CRUD-able, searchable via FTS and vector, and filterable by version predicates.

### Phase 2: Write Pipeline (Story 2 + Story 4 write-side)

**Goal**: Implement the quality gate, 4-outcome duplicate detection, and async embedding flow.

**Dependencies**: Phase 1 (storage layer)

**Tasks**:
1. Create `@fixonce/pipeline` package structure
2. Implement OpenRouter LLM client wrapper:
   - Uses `openai` npm package pointed at OpenRouter base URL
   - Model config per task type (gemma-3-4b-it for quality gate/rewriting, claude-3.5-haiku for dedup)
   - Timeouts (10s), structured error handling with `{ stage, reason, suggestion }`
   - `X-Title: fixonce` header for OpenRouter analytics
3. Implement quality gate (`pipeline/write/quality-gate.ts`):
   - LLM evaluation: accept (actionable, generalizable, has "why") or reject (vague, too specific, obvious) with reason
   - Credential pattern check on content and title fields
   - Only applies to `created_by: "ai"` (human bypass per D11)
4. Implement duplicate detection (`pipeline/write/duplicate-detection.ts`):
   - Similarity search against existing memories (uses storage search)
   - LLM-driven 4-outcome decision: discard, replace, update, merge (D13)
   - Merge: creates new combined memory, disables originals
5. Implement write path orchestration (`pipeline/write/index.ts`):
   - Human path: store immediately → trigger async embedding
   - AI path: quality gate → dedup → store/reject → trigger async embedding
   - Version predicates set at creation time (D18)
6. Create `@fixonce/activity` package:
   - `logActivity()` function for cross-cutting activity logging
   - SSE event emitter for Web UI realtime stream
7. Implement `createMemory` service function (`pipeline/service.ts`)
8. Write tests:
   - Quality gate: accept actionable + reject vague (at least 5 scenarios)
   - Duplicate detection: all 4 outcomes
   - Human bypass: direct store without quality gate
   - Async embedding: memory metadata-searchable immediately, vector-searchable after embedding completes
   - Credential pattern rejection

**Acceptance**: All Story 2 acceptance scenarios pass (9 scenarios). AI memories filtered, human memories bypass. Dedup works with all 4 outcomes.

### Phase 3: Read Pipeline (Story 3 + Story 4 read-side)

**Goal**: Implement the three-stage retrieval pipeline with two-tier result budgeting and hook integration.

**Dependencies**: Phase 1 (storage for search), Phase 2 (pipeline package structure, LLM client)

**Tasks**:
1. Implement query rewriter (`pipeline/read/query-rewriter.ts`):
   - LLM reformulates user context into optimized search queries
   - Skippable via `rewrite=false`
2. Implement reranker (`pipeline/read/reranker.ts`):
   - LLM consolidates, deduplicates, ranks candidate memories
   - Assigns relevancy scores
   - De-ranks memories with negative feedback (but still surfaces with warning per D19)
   - Skippable via `rerank=false`
3. Implement read path orchestration (`pipeline/read/index.ts`):
   - Stage 1: Query rewriting (optional)
   - Stage 2: Hybrid search via storage layer (metadata filters + vector similarity)
   - Stage 3: Reranking (optional)
   - Two-tier response: top N full memories + overflow summaries with cache keys (D16)
   - Version predicate filtering applied during search stage
   - `surfaced_count` and `last_surfaced_at` updated for returned memories
4. Implement cache key system (`pipeline/read/cache.ts`):
   - In-memory cache mapping `ck_<hash>` → memory ID
   - TTL-based expiry (e.g., 1 hour)
   - `expandCacheKey` service function
5. Implement remaining service functions in `pipeline/service.ts`:
   - `queryMemories` with configurable pipeline (rewrite/type/rerank toggles)
   - `expandCacheKey`
   - `getMemory`
   - `updateMemory` (triggers async embedding regen on content change)
   - `submitFeedback`
   - `detectEnvironment`
6. Implement environment detection:
   - Scan `package.json` / `package-lock.json` for SDK versions
   - Scan `compact.toml` or compiler config for compiler version
   - Return `DetectedVersions` matching version predicate key format
7. Write tests:
   - Full pipeline end-to-end (rewrite → search → rerank)
   - Simple mode (no LLM calls)
   - Two-tier result budgeting
   - Cache key expansion
   - Version filtering in search results
   - Pipeline graceful degradation (rewrite fails → continue without rewriting)

**Acceptance**: All Story 3 acceptance scenarios pass (scenarios 1-9; scenario 10 Agent Teams monitor deferred as experimental). All Story 4 acceptance scenarios pass (7 scenarios).

### Phase 4: MCP Server (Story 5)

**Goal**: Expose all service layer functions as MCP tools.

**Dependencies**: Phases 2 and 3 (service layer functions)

**Tasks**:
1. Create `apps/mcp-server` package
2. Register 7 MCP tools with `@modelcontextprotocol/sdk`:
   - `fixonce_create_memory` → `createMemory`
   - `fixonce_query` → `queryMemories`
   - `fixonce_expand` → `expandCacheKey`
   - `fixonce_get_memory` → `getMemory`
   - `fixonce_update_memory` → `updateMemory`
   - `fixonce_feedback` → `submitFeedback`
   - `fixonce_detect_environment` → `detectEnvironment`
3. Define Zod v4 input schemas for each tool (imported from `@fixonce/shared`)
4. Implement stdio transport via `StdioServerTransport`
5. Implement structured error responses per contracts/mcp-tools.md
6. Write MCP tool contract tests:
   - Each tool accepts valid input and returns expected output shape
   - Each tool rejects invalid input with structured error
   - Verbosity levels return correct field sets

**Acceptance**: All Story 5 acceptance scenarios pass (10 scenarios). All 7 tools functional with proper validation and error handling.

### Phase 5: CLI (Story 6)

**Goal**: Provide command-line interface mirroring MCP tools.

**Dependencies**: Phase 4 (service layer proven via MCP tests)

**Tasks**:
1. Create `apps/cli` package with `bin` entry for `fixonce`
2. Implement 9 commands (using `commander` or `yargs`):
   - `fixonce create` — flags + stdin pipe support
   - `fixonce query` — positional query arg + filter/pipeline control flags
   - `fixonce expand` — positional cache_key arg (added per constitution compliance)
   - `fixonce get` — positional UUID arg
   - `fixonce update` — positional UUID arg + partial field flags + stdin
   - `fixonce feedback` — positional UUID arg + feedback flags
   - `fixonce detect` — optional path arg
   - `fixonce serve` — start MCP server process
   - `fixonce web` — start Web UI server process
3. Implement output formatters:
   - Human-readable default (table/card format)
   - `--json` flag outputs MCP tool response shape
4. Implement stdin pipe detection and reading for `create` and `update`
5. Validate CLI arguments at boundary before calling service layer
6. Write tests:
   - Argument parsing for each command
   - stdin pipe reading
   - JSON output matches MCP response shape

**Acceptance**: All Story 6 acceptance scenarios pass (10 scenarios). All commands functional, pipe support works, JSON output matches MCP.

### Phase 6: Web UI (Story 7)

**Goal**: Provide a local web interface for memory management with realtime activity stream.

**Dependencies**: Phases 2 and 3 (service layer), Phase 5 (CLI for `fixonce web` launcher)

**Tasks**:
1. Create `apps/web` package with React + Vite
2. Implement Web UI backend (Express or Fastify):
   - 11 HTTP API routes wrapping service layer (per contracts/service-layer.md)
   - `PATCH /api/memories/:id` sets `created_by = "human_modified"` before calling service
   - `POST /api/memories/preview-duplicates` for live duplicate suggestions
   - `GET /api/activity/stream` SSE endpoint for realtime updates
   - `DELETE /api/memories/:id` hard delete (Web UI only)
3. Implement 6 views:
   - **Dashboard**: Overview stats + flagged memories list (prominent) + quick actions
   - **Memory Query**: Form mirroring `fixonce_query` params + results cards with relevancy scores
   - **Memory Detail**: Full editable view + feedback history + version predicates display
   - **Create Memory**: Form for all fields + live duplicate suggestions (debounced)
   - **Recent Feedback**: Filterable list of feedback entries
   - **Recent Activity**: Realtime stream via SSE (`useActivityStream` hook)
4. Implement SSE client hook (`useActivityStream.ts`):
   - `EventSource` API with auto-reconnect
   - `Last-Event-ID` support for resume-from-last-event
5. Write tests:
   - Data fetching for each view
   - SSE connection and event handling
   - Duplicate suggestion debouncing

**Acceptance**: All Story 7 acceptance scenarios pass (9 scenarios). Dashboard shows flagged memories. Query matches CLI/MCP results. Realtime stream updates without page refresh.

### Phase 7: Hook Integration (Story 3 hooks)

**Goal**: Wire FixOnce into Claude Code's hook system for automatic memory surfacing.

**Dependencies**: Phase 4 (MCP server), Phase 3 (retrieval pipeline)

**Tasks**:
1. Implement Claude Code hooks via `@anthropic-ai/claude-agent-sdk`:
   - `SessionStart`: Detect environment → query project-level critical memories (blocking, 1-2s budget)
   - `UserPromptSubmit`: Blocking quick check (sub-second) + async deep search with mid-run injection via `query.streamInput(AsyncIterable<SDKUserMessage>)`
   - `PreToolUse` (matcher: `Write|Edit`): Check content against `anti_pattern` memories (blocking)
   - `PostToolUse` (matcher: `Write|Edit`): Check written content, flag anti-patterns (blocking)
   - `Stop`: Final critical error check against session changes (blocking)
2. Implement environment detection at SessionStart (calls `detectEnvironment`)
3. Implement dual-mode UserPromptSubmit hook:
   - Fast blocking check returns immediate results via `additionalContext`
   - Background deep search injects results via `streamInput()`
4. Document hook configuration for end users

**Acceptance**: Story 3 scenarios 1-9 pass. Memories surface at appropriate hook points with correct context. (Scenario 10 — Agent Teams monitor — deferred as experimental per D17.)

### Phase 8: Integration Testing + Polish

**Goal**: End-to-end validation, cross-interface consistency, and documentation.

**Tasks**:
1. End-to-end integration tests:
   - Create memory via CLI → query via MCP → feedback via Web UI → verify in dashboard
   - Version-filtered queries return correct subset across all interfaces
   - Activity stream captures operations from all three interfaces
2. Cross-interface consistency verification:
   - Same query params produce same results in MCP, CLI, and Web UI (SC-017)
   - JSON output from CLI matches MCP response shape
3. Create `quickstart.md` with development setup instructions
4. Create `.env.example` with all required environment variables
5. Verify all 19 success criteria from spec pass

**Acceptance**: All 42 functional requirements verified. All 19 success criteria pass. Development documentation complete.

## Complexity Tracking

> **No constitution violations requiring justification.**

The plan follows all 9 principles without deviation:
- API-First: Single service layer, MCP-first schemas
- Modularity: 7 separate packages with unidirectional deps
- KISS: No abstract providers, no deferred features, 3 fixed pipeline stages
- Test Critical Paths: Integration tests against real Supabase
- Fail Fast: Structured errors with stage identification
- Validate at Boundaries: Zod at MCP/CLI/HTTP, trusted internally
- Protect Secrets: Env vars only, credential pattern detection
- Semver: Version bump triggers documented
- Conventional Commits: Scopes defined per module

## Key Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| AsyncIterable mid-run injection API changes | Medium | Confirmed available in SDK. Fallback: return all results in blocking phase |
| Agent Teams monitor (Story 3, Scenario 10) | Low | Deferred from v1 implementation. Isolated experimental feature |
| OpenRouter model quality for quality gate | Medium | Configurable model per task. Start with gemma-3-4b-it, upgrade if needed |
| Supabase free tier limits | Low | Affordable paid tier. Low usage in early adoption |
| Multi-stage pipeline latency | Medium | Pipeline fully configurable. Simple mode skips all LLM calls |

## Dependencies Summary

| Package | Version | Module | Purpose |
|---------|---------|--------|---------|
| `@modelcontextprotocol/sdk` | latest | `apps/mcp-server` | MCP server |
| `@anthropic-ai/claude-agent-sdk` | latest | hooks | Claude Code hook registration |
| `zod` | v4 | `@fixonce/shared` | Schema validation |
| `@supabase/supabase-js` | v2+ | `@fixonce/storage` | Supabase client |
| `voyageai` | latest | `@fixonce/storage` | Embedding generation |
| `openai` | latest | `@fixonce/pipeline` | OpenRouter API (OpenAI-compatible) |
| `react` | v19 | `apps/web` | Web UI framework |
| `vite` | latest | `apps/web` | Web UI build tool |
| `vitest` | latest | root | Test framework |
| `tsup` | latest | packages/* | Library build tool |
| `pnpm` | v9+ | root | Package manager |
| `turbo` | latest | root | Monorepo task runner |

## Artifacts Generated

| Artifact | Path | Content |
|----------|------|---------|
| Spec Analysis | `specs/001-fixonce-memory-layer/spec-analysis.md` | Deep synthesis of all discovery docs |
| Constitution Compliance | `specs/001-fixonce-memory-layer/constitution-compliance.md` | Principle-by-principle compliance framework |
| Research | `specs/001-fixonce-memory-layer/research.md` | All 7 technical unknowns resolved |
| Data Model | `specs/001-fixonce-memory-layer/data-model.md` | 3 entities, 8 enums, indexes, state machines |
| MCP Tool Contracts | `specs/001-fixonce-memory-layer/contracts/mcp-tools.md` | All 7 tool schemas |
| CLI Command Contracts | `specs/001-fixonce-memory-layer/contracts/cli-commands.md` | All 9 command specs |
| Service Layer Contracts | `specs/001-fixonce-memory-layer/contracts/service-layer.md` | TypeScript interfaces + HTTP API |
