# Tasks: fixonce-memory-layer

**Input**: Design documents from `/specs/001-fixonce-memory-layer/`
**Prerequisites**: plan.md, spec.md, data-model.md, research.md, contracts/

**Tests**: Tests are included as requested in the plan (integration tests against real Supabase, contract tests for MCP tools).

**Organization**: Tasks grouped by user story from spec.md. Stories 1-5 are P1, Stories 6-7 are P2. Story 4 (Version-Scoped Metadata) is cross-cutting and woven into relevant phases.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1-US7)
- Exact file paths included per plan.md project structure

## Path Conventions

pnpm monorepo:
- `packages/shared/src/` — @fixonce/shared
- `packages/storage/src/` — @fixonce/storage
- `packages/pipeline/src/` — @fixonce/pipeline
- `packages/activity/src/` — @fixonce/activity
- `apps/mcp-server/src/` — MCP server
- `apps/cli/src/` — CLI
- `apps/web/src/` — Web UI frontend
- `apps/web/server/` — Web UI backend
- `tests/` — Integration tests

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Initialize monorepo structure, shared TypeScript config, and workspace tooling

- [ ] T001 [GIT] Verify on main branch and working tree is clean
- [ ] T002 [GIT] Pull latest changes from origin/main
- [ ] T003 [GIT] Create feature branch: 001-fixonce-memory-layer
- [ ] T004 Initialize pnpm monorepo with pnpm-workspace.yaml defining packages/* and apps/* (use dev-specialisms:init-local-tooling skill)
- [ ] T005 [GIT] Commit: initialize pnpm workspace
- [ ] T006 [P] Create turbo.json with build/lint/test task pipelines (use dev-specialisms:init-local-tooling skill)
- [ ] T007 [P] Create tsconfig.base.json with strict mode, ES2022 target, NodeNext module resolution (use dev-specialisms:init-local-tooling skill)
- [ ] T008 [GIT] Commit: add Turborepo and base TypeScript config
- [ ] T009 Create .env.example with SUPABASE_URL, SUPABASE_ANON_KEY, VOYAGE_API_KEY, OPENROUTER_API_KEY
- [ ] T010 [P] Create .gitignore with node_modules, dist, .env*, .turbo entries
- [ ] T011 [GIT] Commit: add environment template and gitignore
- [ ] T012 Create root package.json with pnpm workspace config, turbo scripts, and vitest as dev dependency (use dev-specialisms:init-local-tooling skill)
- [ ] T013 [GIT] Commit: configure root package.json

### Phase Completion
- [ ] T014 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T015 [GIT] Create/update PR to main with phase summary
- [ ] T016 [GIT] Verify all CI checks pass
- [ ] T017 [GIT] Report PR ready status

---

## Phase 2: Foundational — Shared Types + Storage Layer (Blocking Prerequisites)

**Purpose**: @fixonce/shared schemas and @fixonce/storage CRUD + search — the data foundation ALL stories depend on. Covers Story 1 (Storage & Schema) and Story 4 schema portions (Version Predicates).

**CRITICAL**: No user story work can begin until this phase is complete.

### Phase Start
- [ ] T018 [GIT] Verify working tree is clean before starting Phase 2
- [ ] T019 [GIT] Pull and rebase on origin/main if needed
- [ ] T020 Create retro/P2.md for this phase

### @fixonce/shared Package
- [ ] T021 [GIT] Commit: initialize phase 2 retro
- [ ] T022 Scaffold packages/shared/ with package.json (@fixonce/shared), tsconfig.json extending base (use devs:typescript-dev agent)
- [ ] T023 [GIT] Commit: scaffold @fixonce/shared package
- [ ] T024 [P] Implement packages/shared/src/enums.ts with MemoryType, SourceType, CreatedBy, FeedbackTag, SuggestedAction, OperationType, SearchType, Verbosity (use devs:typescript-dev agent)
- [ ] T025 [P] Implement packages/shared/src/version-keys.ts with 12 ComponentKey types, VersionPredicates, DetectedVersions (use devs:typescript-dev agent)
- [ ] T026 [P] Implement packages/shared/src/errors.ts with ServiceError interface and error factory functions (use devs:typescript-dev agent)
- [ ] T027 [GIT] Commit: add shared enums, version keys, and error types
- [ ] T028 Implement packages/shared/src/types.ts with Memory, Feedback, ActivityLog, MemorySmall/Medium/Large, OverflowEntry, FeedbackSummary interfaces (use devs:typescript-dev agent)
- [ ] T029 [GIT] Commit: add shared TypeScript interfaces
- [ ] T030 Implement packages/shared/src/schema.ts with Zod v4 schemas for all types — CreateMemoryInput, QueryMemoriesInput, SubmitFeedbackInput, etc. per contracts/service-layer.md (use devs:typescript-dev agent)
- [ ] T031 [GIT] Commit: add Zod validation schemas
- [ ] T032 Create packages/shared/src/index.ts barrel export for all types, schemas, enums (use devs:typescript-dev agent)
- [ ] T033 [GIT] Commit: add shared barrel export

### @fixonce/storage Package — Client + Migrations
- [ ] T034 Scaffold packages/storage/ with package.json (@fixonce/storage, depends on @fixonce/shared, @supabase/supabase-js), tsconfig.json (use devs:typescript-dev agent)
- [ ] T035 [GIT] Commit: scaffold @fixonce/storage package
- [ ] T036 Implement packages/storage/src/client.ts with createSupabaseClient() using env var validation — fail fast if SUPABASE_URL or SUPABASE_ANON_KEY missing (use devs:typescript-dev agent)
- [ ] T037 [GIT] Commit: add Supabase client with env validation
- [ ] T038 [P] Create packages/storage/migrations/001_extensions.sql enabling vector and uuid-ossp extensions (use devs:typescript-dev agent)
- [ ] T039 [P] Create packages/storage/migrations/002_enums.sql with all 6 database enums per data-model.md (use devs:typescript-dev agent)
- [ ] T040 [P] Create packages/storage/migrations/003_memories.sql with full memory table schema per data-model.md (use devs:typescript-dev agent)
- [ ] T041 [P] Create packages/storage/migrations/004_feedback.sql with feedback table and FK to memory (use devs:typescript-dev agent)
- [ ] T042 [P] Create packages/storage/migrations/005_activity_log.sql with activity_log table (use devs:typescript-dev agent)
- [ ] T043 [GIT] Commit: add SQL migrations for extensions, enums, and tables
- [ ] T044 [P] Create packages/storage/migrations/006_indexes.sql with all indexes per data-model.md — GIN (tsvector, JSONB, tags), HNSW (pgvector cosine), btree (partial on enabled, language, memory_type) (use devs:typescript-dev agent)
- [ ] T045 [P] Create packages/storage/migrations/007_fts_column.sql with weighted tsvector generated column per data-model.md (use devs:typescript-dev agent)
- [ ] T046 [P] Create packages/storage/migrations/008_triggers.sql with updated_at auto-refresh trigger (use devs:typescript-dev agent)
- [ ] T047 [GIT] Commit: add indexes, FTS column, and triggers
- [ ] T048 Create packages/storage/migrations/009_hybrid_search_rpc.sql with Reciprocal Rank Fusion function per research.md — uses full_text CTE + semantic CTE with RRF scoring (use devs:typescript-dev agent)
- [ ] T049 [GIT] Commit: add hybrid search RPC function

### @fixonce/storage Package — CRUD + Search Operations
- [ ] T050 Implement packages/storage/src/memories.ts with Memory CRUD: create, getById, update, delete, listEnabled (use devs:typescript-dev agent)
- [ ] T051 [GIT] Commit: add memory CRUD operations
- [ ] T052 Implement packages/storage/src/feedback.ts with Feedback CRUD: create, listByMemoryId, listFlagged (use devs:typescript-dev agent)
- [ ] T053 [GIT] Commit: add feedback CRUD operations
- [ ] T054 Implement packages/storage/src/activity.ts with ActivityLog append and list with pagination (use devs:typescript-dev agent)
- [ ] T055 [GIT] Commit: add activity log operations
- [ ] T056 Implement packages/storage/src/version-filter.ts with version predicate query builder — OR within component, AND across components per data-model.md (use devs:typescript-dev agent)
- [ ] T057 [GIT] Commit: add version predicate query builder
- [ ] T058 Implement packages/storage/src/search.ts with hybrid search (calls Supabase RPC), FTS-only search, vector-only search, and metadata-filtered search (use devs:typescript-dev agent)
- [ ] T059 [GIT] Commit: add search operations
- [ ] T060 Implement packages/storage/src/embeddings.ts with Voyage AI client using voyageai npm package — async embedding generation with input_type document/query (use devs:typescript-dev agent)
- [ ] T061 [GIT] Commit: add Voyage AI embedding module
- [ ] T062 Create packages/storage/src/index.ts barrel export with createStorage() factory (use devs:typescript-dev agent)
- [ ] T063 [GIT] Commit: add storage barrel export

### Integration Tests (Story 1 + Story 4)
- [ ] T064 [P] Write tests/storage/memories.test.ts — Memory CRUD for all 7 Story 1 scenarios (use devs:typescript-dev agent)
- [ ] T065 [P] Write tests/storage/search.test.ts — FTS, vector similarity, and hybrid filtered search (use devs:typescript-dev agent)
- [ ] T066 [P] Write tests/storage/version-filter.test.ts — all 7 Story 4 version predicate scenarios (use devs:typescript-dev agent)
- [ ] T067 [GIT] Commit: add storage integration tests
- [ ] T068 Run codebase mapping for Phase 2 changes (/sdd:map incremental)
- [ ] T069 [GIT] Commit: update codebase documents for phase 2
- [ ] T070 Review retro/P2.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T071 [GIT] Commit: finalize phase 2 retro

**Checkpoint**: All Story 1 acceptance scenarios pass. Memories CRUD-able, searchable via FTS/vector/hybrid, filterable by version predicates.

### Phase Completion
- [ ] T072 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T073 [GIT] Create/update PR to main with phase summary
- [ ] T074 [GIT] Verify all CI checks pass
- [ ] T075 [GIT] Report PR ready status

---

## Phase 3: Write Pipeline [US1] (Priority: P1)

**Goal**: Quality gate, 4-outcome duplicate detection, async embedding flow — Story 2 + Story 4 write-side.

**Independent Test**: Submit memories via service layer. AI memories filtered by quality gate, human memories bypass. Dedup works with all 4 outcomes.

### Phase Start
- [ ] T076 [GIT] Verify working tree is clean before starting Phase 3
- [ ] T077 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T078 [US1] Create retro/P3.md for this phase
- [ ] T079 [GIT] Commit: initialize phase 3 retro
- [ ] T080 [US1] Scaffold packages/pipeline/ with package.json (@fixonce/pipeline, depends on @fixonce/shared, @fixonce/storage, openai), tsconfig.json (use devs:typescript-dev agent)
- [ ] T081 [GIT] Commit: scaffold @fixonce/pipeline package
- [ ] T082 [US1] Implement packages/pipeline/src/llm.ts — OpenRouter client wrapper using openai npm package, model config per task type (gemma-3-4b-it cheap, claude-3.5-haiku nuanced), X-Title header, 10s timeout, structured error handling (use devs:typescript-dev agent)
- [ ] T083 [GIT] Commit: add OpenRouter LLM client wrapper
- [ ] T084 [P] [US1] Implement packages/pipeline/src/write/credential-check.ts — regex patterns for API keys, passwords, tokens in content and title (use devs:typescript-dev agent)
- [ ] T085 [P] [US1] Implement packages/pipeline/src/write/quality-gate.ts — LLM evaluation accept/reject with reason, applies only to created_by:ai, includes credential check (use devs:typescript-dev agent)
- [ ] T086 [GIT] Commit: add credential check and quality gate
- [ ] T087 [US1] Implement packages/pipeline/src/write/duplicate-detection.ts — similarity search + LLM 4-outcome decision (discard, replace, update, merge) per D13 (use devs:typescript-dev agent)
- [ ] T088 [GIT] Commit: add duplicate detection with 4 outcomes
- [ ] T089 [US1] Implement packages/pipeline/src/write/index.ts — write path orchestration: human path (store + async embed) vs AI path (quality gate -> dedup -> store/reject -> async embed) (use devs:typescript-dev agent)
- [ ] T090 [GIT] Commit: add write pipeline orchestration
- [ ] T091 [US1] Scaffold packages/activity/ with package.json (@fixonce/activity, depends on @fixonce/shared, @fixonce/storage), tsconfig.json (use devs:typescript-dev agent)
- [ ] T092 [US1] Implement packages/activity/src/index.ts with logActivity() function for cross-cutting activity logging (use devs:typescript-dev agent)
- [ ] T093 [US1] Implement packages/activity/src/stream.ts with SSE event emitter for Web UI realtime stream (use devs:typescript-dev agent)
- [ ] T094 [GIT] Commit: add @fixonce/activity package with logging and SSE
- [ ] T095 [US1] Implement packages/pipeline/src/service.ts with createMemory service function per contracts/service-layer.md (use devs:typescript-dev agent)
- [ ] T096 [GIT] Commit: add createMemory service function
- [ ] T097 [US1] Create packages/pipeline/src/index.ts barrel export with createPipeline() factory (use devs:typescript-dev agent)
- [ ] T098 [GIT] Commit: add pipeline barrel export

### Tests (Story 2)
- [ ] T099 [US1] Write tests/pipeline/write-pipeline.test.ts — quality gate accept/reject (5+ scenarios), 4 dedup outcomes, human bypass, async embedding, credential rejection (use devs:typescript-dev agent)
- [ ] T100 [GIT] Commit: add write pipeline integration tests
- [ ] T101 [US1] Run codebase mapping for Phase 3 changes (/sdd:map incremental)
- [ ] T102 [GIT] Commit: update codebase documents for phase 3
- [ ] T103 [US1] Review retro/P3.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T104 [GIT] Commit: finalize phase 3 retro

**Checkpoint**: All Story 2 acceptance scenarios pass (9 scenarios). AI memories filtered, human memories bypass. Dedup works with all 4 outcomes.

### Phase Completion
- [ ] T105 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T106 [GIT] Create/update PR to main with phase summary
- [ ] T107 [GIT] Verify all CI checks pass
- [ ] T108 [GIT] Report PR ready status

---

## Phase 4: Read Pipeline [US2] (Priority: P1)

**Goal**: Three-stage retrieval pipeline with two-tier result budgeting — Story 3 + Story 4 read-side.

**Independent Test**: Query memories via service layer. Full pipeline (rewrite -> search -> rerank) and simple mode both work. Two-tier results and cache key expansion functional.

### Phase Start
- [ ] T109 [GIT] Verify working tree is clean before starting Phase 4
- [ ] T110 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T111 [US2] Create retro/P4.md for this phase
- [ ] T112 [GIT] Commit: initialize phase 4 retro
- [ ] T113 [US2] Implement packages/pipeline/src/read/query-rewriter.ts — LLM reformulates user context into optimized search queries, skippable via rewrite=false (use devs:typescript-dev agent)
- [ ] T114 [GIT] Commit: add query rewriter
- [ ] T115 [US2] Implement packages/pipeline/src/read/reranker.ts — LLM consolidates, deduplicates, ranks candidates, assigns relevancy scores, de-ranks negative feedback memories with warning per D19 (use devs:typescript-dev agent)
- [ ] T116 [GIT] Commit: add reranker
- [ ] T117 [US2] Implement packages/pipeline/src/read/cache.ts — in-memory cache mapping ck_<hash> to memory ID with TTL-based expiry (use devs:typescript-dev agent)
- [ ] T118 [GIT] Commit: add cache key system
- [ ] T119 [US2] Implement packages/pipeline/src/read/index.ts — read path orchestration: rewrite (optional) -> hybrid search -> rerank (optional) -> two-tier response (top N full + overflow summaries with cache keys) -> update surfaced_count/last_surfaced_at (use devs:typescript-dev agent)
- [ ] T120 [GIT] Commit: add read pipeline orchestration
- [ ] T121 [US2] Implement environment detection in packages/pipeline/src/environment.ts — scan package.json, lock files, compact.toml for Midnight component versions, return DetectedVersions (use devs:typescript-dev agent)
- [ ] T122 [GIT] Commit: add environment detection
- [ ] T123 [US2] Add remaining service functions to packages/pipeline/src/service.ts — queryMemories, expandCacheKey, getMemory, updateMemory, submitFeedback, detectEnvironment per contracts/service-layer.md (use devs:typescript-dev agent)
- [ ] T124 [GIT] Commit: complete service layer with all 7 functions

### Tests (Story 3 + Story 4)
- [ ] T125 [US2] Write tests/pipeline/read-pipeline.test.ts — full pipeline e2e, simple mode, two-tier budgeting, cache key expansion, version filtering in results, graceful degradation (use devs:typescript-dev agent)
- [ ] T126 [GIT] Commit: add read pipeline integration tests
- [ ] T127 [US2] Run codebase mapping for Phase 4 changes (/sdd:map incremental)
- [ ] T128 [GIT] Commit: update codebase documents for phase 4
- [ ] T129 [US2] Review retro/P4.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T130 [GIT] Commit: finalize phase 4 retro

**Checkpoint**: All Story 3 acceptance scenarios pass (scenarios 1-9; scenario 10 deferred). All Story 4 acceptance scenarios pass (7 scenarios).

### Phase Completion
- [ ] T131 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T132 [GIT] Create/update PR to main with phase summary
- [ ] T133 [GIT] Verify all CI checks pass
- [ ] T134 [GIT] Report PR ready status

---

## Phase 5: MCP Server [US3] (Priority: P1)

**Goal**: Expose all 7 service layer functions as MCP tools — Story 5.

**Independent Test**: Each MCP tool accepts valid input and returns expected output shape. Invalid input returns structured errors.

### Phase Start
- [ ] T135 [GIT] Verify working tree is clean before starting Phase 5
- [ ] T136 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T137 [US3] Create retro/P5.md for this phase
- [ ] T138 [GIT] Commit: initialize phase 5 retro
- [ ] T139 [US3] Scaffold apps/mcp-server/ with package.json (@fixonce/mcp-server, depends on @fixonce/pipeline, @modelcontextprotocol/sdk, zod), tsconfig.json (use devs:typescript-dev agent)
- [ ] T140 [GIT] Commit: scaffold MCP server app
- [ ] T141 [P] [US3] Implement apps/mcp-server/src/tools/create-memory.ts — fixonce_create_memory tool delegating to service.createMemory (use devs:typescript-dev agent)
- [ ] T142 [P] [US3] Implement apps/mcp-server/src/tools/query.ts — fixonce_query tool delegating to service.queryMemories (use devs:typescript-dev agent)
- [ ] T143 [P] [US3] Implement apps/mcp-server/src/tools/expand.ts — fixonce_expand tool delegating to service.expandCacheKey (use devs:typescript-dev agent)
- [ ] T144 [P] [US3] Implement apps/mcp-server/src/tools/get-memory.ts — fixonce_get_memory tool delegating to service.getMemory (use devs:typescript-dev agent)
- [ ] T145 [P] [US3] Implement apps/mcp-server/src/tools/update-memory.ts — fixonce_update_memory tool delegating to service.updateMemory (use devs:typescript-dev agent)
- [ ] T146 [P] [US3] Implement apps/mcp-server/src/tools/feedback.ts — fixonce_feedback tool delegating to service.submitFeedback (use devs:typescript-dev agent)
- [ ] T147 [P] [US3] Implement apps/mcp-server/src/tools/detect-environment.ts — fixonce_detect_environment tool delegating to service.detectEnvironment (use devs:typescript-dev agent)
- [ ] T148 [GIT] Commit: add all 7 MCP tool handlers
- [ ] T149 [US3] Implement apps/mcp-server/src/index.ts — McpServer creation, register all 7 tools with Zod v4 input schemas from @fixonce/shared, connect StdioServerTransport (use devs:typescript-dev agent)
- [ ] T150 [GIT] Commit: add MCP server entry point with tool registration

### Tests (Story 5)
- [ ] T151 [US3] Write tests/mcp/tool-contracts.test.ts — each tool accepts valid input and returns expected shape, rejects invalid input with structured error, verbosity levels return correct field sets (use devs:typescript-dev agent)
- [ ] T152 [GIT] Commit: add MCP tool contract tests
- [ ] T153 [US3] Run codebase mapping for Phase 5 changes (/sdd:map incremental)
- [ ] T154 [GIT] Commit: update codebase documents for phase 5
- [ ] T155 [US3] Review retro/P5.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T156 [GIT] Commit: finalize phase 5 retro

**Checkpoint**: All Story 5 acceptance scenarios pass (10 scenarios). All 7 MCP tools functional with proper validation and error handling.

### Phase Completion
- [ ] T157 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T158 [GIT] Create/update PR to main with phase summary
- [ ] T159 [GIT] Verify all CI checks pass
- [ ] T160 [GIT] Report PR ready status

---

## Phase 6: CLI Interface [US4] (Priority: P2)

**Goal**: Command-line interface mirroring MCP tools with 9 commands — Story 6.

**Independent Test**: All commands functional, pipe support works, --json output matches MCP response shape.

### Phase Start
- [ ] T161 [GIT] Verify working tree is clean before starting Phase 6
- [ ] T162 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T163 [US4] Create retro/P6.md for this phase
- [ ] T164 [GIT] Commit: initialize phase 6 retro
- [ ] T165 [US4] Scaffold apps/cli/ with package.json (fixonce, bin: "fixonce", depends on @fixonce/pipeline, commander or yargs), tsconfig.json (use devs:typescript-dev agent)
- [ ] T166 [GIT] Commit: scaffold CLI app
- [ ] T167 [P] [US4] Implement apps/cli/src/commands/create.ts — fixonce create with flags + stdin pipe support per contracts/cli-commands.md (use devs:typescript-dev agent)
- [ ] T168 [P] [US4] Implement apps/cli/src/commands/query.ts — fixonce query with positional arg + filter/pipeline control flags (use devs:typescript-dev agent)
- [ ] T169 [P] [US4] Implement apps/cli/src/commands/expand.ts — fixonce expand with positional cache_key arg (use devs:typescript-dev agent)
- [ ] T170 [P] [US4] Implement apps/cli/src/commands/get.ts — fixonce get with positional UUID arg (use devs:typescript-dev agent)
- [ ] T171 [P] [US4] Implement apps/cli/src/commands/update.ts — fixonce update with UUID arg + partial field flags + stdin (use devs:typescript-dev agent)
- [ ] T172 [P] [US4] Implement apps/cli/src/commands/feedback.ts — fixonce feedback with UUID arg + feedback flags (use devs:typescript-dev agent)
- [ ] T173 [P] [US4] Implement apps/cli/src/commands/detect.ts — fixonce detect with optional path arg (use devs:typescript-dev agent)
- [ ] T174 [P] [US4] Implement apps/cli/src/commands/serve.ts — fixonce serve starting MCP server process (use devs:typescript-dev agent)
- [ ] T175 [P] [US4] Implement apps/cli/src/commands/web.ts — fixonce web starting Web UI server process (use devs:typescript-dev agent)
- [ ] T176 [GIT] Commit: add all 9 CLI commands
- [ ] T177 [US4] Implement apps/cli/src/formatters/index.ts — human-readable table/card format + --json flag for MCP response shape output (use devs:typescript-dev agent)
- [ ] T178 [GIT] Commit: add CLI output formatters
- [ ] T179 [US4] Implement apps/cli/src/index.ts — CLI entrypoint with command registration, global --json flag, stdin detection (use devs:typescript-dev agent)
- [ ] T180 [GIT] Commit: add CLI entry point
- [ ] T181 [US4] Run codebase mapping for Phase 6 changes (/sdd:map incremental)
- [ ] T182 [GIT] Commit: update codebase documents for phase 6
- [ ] T183 [US4] Review retro/P6.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T184 [GIT] Commit: finalize phase 6 retro

**Checkpoint**: All Story 6 acceptance scenarios pass (10 scenarios). All commands functional, pipe support works, JSON output matches MCP.

### Phase Completion
- [ ] T185 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T186 [GIT] Create/update PR to main with phase summary
- [ ] T187 [GIT] Verify all CI checks pass
- [ ] T188 [GIT] Report PR ready status

---

## Phase 7: Web UI [US5] (Priority: P2)

**Goal**: Local web interface for memory management with realtime activity stream — Story 7.

**Independent Test**: Dashboard shows flagged memories. Query matches CLI/MCP results. Realtime stream updates without page refresh.

### Phase Start
- [ ] T189 [GIT] Verify working tree is clean before starting Phase 7
- [ ] T190 [GIT] Pull and rebase on origin/main if needed

### Web UI Backend
- [ ] T191 [US5] Create retro/P7.md for this phase
- [ ] T192 [GIT] Commit: initialize phase 7 retro
- [ ] T193 [US5] Scaffold apps/web/ with package.json (@fixonce/web, depends on @fixonce/pipeline, react v19, vite, express), tsconfig.json, vite.config.ts (use devs:typescript-dev agent)
- [ ] T194 [GIT] Commit: scaffold Web UI app
- [ ] T195 [US5] Implement apps/web/server/routes.ts — 11 HTTP API routes per contracts/service-layer.md including PATCH setting created_by=human_modified, POST preview-duplicates, GET activity/stream SSE, DELETE hard delete (use devs:typescript-dev agent)
- [ ] T196 [GIT] Commit: add Web UI API routes
- [ ] T197 [US5] Implement apps/web/server/index.ts — HTTP server setup + SSE activity stream endpoint per research.md SSE decision (use devs:typescript-dev agent)
- [ ] T198 [GIT] Commit: add Web UI server with SSE support

### Web UI Frontend
- [ ] T199 [US5] Implement apps/web/src/api/client.ts — HTTP client for backend API with typed request/response (use devs:react-dev agent)
- [ ] T200 [GIT] Commit: add API client
- [ ] T201 [US5] Implement apps/web/src/hooks/useActivityStream.ts — SSE EventSource hook with auto-reconnect and Last-Event-ID support (use devs:react-dev agent)
- [ ] T202 [GIT] Commit: add SSE activity stream hook
- [ ] T203 [P] [US5] Implement apps/web/src/views/Dashboard.tsx — overview stats + flagged memories list (prominent) + quick actions (disable, delete) (use devs:react-dev agent)
- [ ] T204 [P] [US5] Implement apps/web/src/views/MemoryQuery.tsx — form mirroring fixonce_query params + results cards with relevancy scores (use devs:react-dev agent)
- [ ] T205 [P] [US5] Implement apps/web/src/views/MemoryDetail.tsx — full editable view + feedback history + version predicates display (use devs:react-dev agent)
- [ ] T206 [P] [US5] Implement apps/web/src/views/CreateMemory.tsx — form for all fields + live duplicate suggestions (debounced) (use devs:react-dev agent)
- [ ] T207 [P] [US5] Implement apps/web/src/views/RecentFeedback.tsx — filterable feedback list by tags, action, date range (use devs:react-dev agent)
- [ ] T208 [P] [US5] Implement apps/web/src/views/RecentActivity.tsx — realtime stream via useActivityStream hook, filterable by operation type (use devs:react-dev agent)
- [ ] T209 [GIT] Commit: add all 6 Web UI views
- [ ] T210 [US5] Implement apps/web/src/App.tsx — router setup for 6 views with navigation (use devs:react-dev agent)
- [ ] T211 [US5] Implement apps/web/src/main.tsx — React 19 entry point (use devs:react-dev agent)
- [ ] T212 [GIT] Commit: add App router and entry point
- [ ] T213 [US5] Run codebase mapping for Phase 7 changes (/sdd:map incremental)
- [ ] T214 [GIT] Commit: update codebase documents for phase 7
- [ ] T215 [US5] Review retro/P7.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T216 [GIT] Commit: finalize phase 7 retro

**Checkpoint**: All Story 7 acceptance scenarios pass (9 scenarios). Dashboard shows flagged memories. Query matches CLI/MCP results. Realtime stream updates.

### Phase Completion
- [ ] T217 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T218 [GIT] Create/update PR to main with phase summary
- [ ] T219 [GIT] Verify all CI checks pass
- [ ] T220 [GIT] Report PR ready status

---

## Phase 8: Hook Integration [US6] (Priority: P1)

**Goal**: Wire FixOnce into Claude Code's hook system for automatic memory surfacing — Story 3 hook scenarios.

**Independent Test**: Memories surface at appropriate hook points. SessionStart returns version-specific memories. UserPromptSubmit does blocking + async deep search. PreToolUse/PostToolUse check anti-patterns.

### Phase Start
- [ ] T221 [GIT] Verify working tree is clean before starting Phase 8
- [ ] T222 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T223 [US6] Create retro/P8.md for this phase
- [ ] T224 [GIT] Commit: initialize phase 8 retro
- [ ] T225 [US6] Implement SessionStart hook — detect environment via detectEnvironment, query project-level critical memories (blocking, 1-2s budget) using @anthropic-ai/claude-agent-sdk (use devs:typescript-dev agent)
- [ ] T226 [GIT] Commit: add SessionStart hook
- [ ] T227 [US6] Implement UserPromptSubmit hook — blocking quick check returning immediate results via additionalContext + async deep search injecting via query.streamInput(AsyncIterable) (use devs:typescript-dev agent)
- [ ] T228 [GIT] Commit: add UserPromptSubmit hook with dual-mode search
- [ ] T229 [US6] Implement PreToolUse hook (matcher: Write|Edit) — check content against anti_pattern memories, deny with reason if matched (use devs:typescript-dev agent)
- [ ] T230 [GIT] Commit: add PreToolUse anti-pattern hook
- [ ] T231 [US6] Implement PostToolUse hook (matcher: Write|Edit) — check written content, add context warning if anti-patterns detected (use devs:typescript-dev agent)
- [ ] T232 [GIT] Commit: add PostToolUse anti-pattern hook
- [ ] T233 [US6] Implement Stop hook — final critical error check against session changes (use devs:typescript-dev agent)
- [ ] T234 [GIT] Commit: add Stop hook
- [ ] T235 [US6] Create .claude/settings.json with hook registration configuration for end users (use devs:typescript-dev agent)
- [ ] T236 [GIT] Commit: add Claude Code hook configuration
- [ ] T237 [US6] Run codebase mapping for Phase 8 changes (/sdd:map incremental)
- [ ] T238 [GIT] Commit: update codebase documents for phase 8
- [ ] T239 [US6] Review retro/P8.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T240 [GIT] Commit: finalize phase 8 retro

**Checkpoint**: Story 3 scenarios 1-9 pass. Memories surface at appropriate hook points with correct context. Scenario 10 (Agent Teams monitor) deferred.

### Phase Completion
- [ ] T241 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T242 [GIT] Create/update PR to main with phase summary
- [ ] T243 [GIT] Verify all CI checks pass
- [ ] T244 [GIT] Report PR ready status

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: End-to-end integration testing, cross-interface consistency, and documentation.

### Phase Start
- [ ] T245 [GIT] Verify working tree is clean before starting Phase 9
- [ ] T246 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T247 Create retro/P9.md for this phase
- [ ] T248 [GIT] Commit: initialize phase 9 retro
- [ ] T249 [P] Write end-to-end integration test: create memory via CLI -> query via MCP -> feedback via Web UI -> verify in dashboard (use devs:typescript-dev agent)
- [ ] T250 [P] Write cross-interface consistency test: same query params produce same results in MCP, CLI, and Web UI (SC-017) (use devs:typescript-dev agent)
- [ ] T251 [P] Write activity stream integration test: operations from all three interfaces appear in activity log (use devs:typescript-dev agent)
- [ ] T252 [GIT] Commit: add end-to-end integration tests
- [ ] T253 Create .env.example with all required environment variables and usage comments
- [ ] T254 [GIT] Commit: finalize environment documentation
- [ ] T255 Verify all 19 success criteria from spec pass — run full test suite (use devs:typescript-dev agent)
- [ ] T256 [GIT] Commit: success criteria verification
- [ ] T257 Run final codebase mapping (/sdd:map incremental)
- [ ] T258 [GIT] Commit: final codebase document update
- [ ] T259 Review retro/P9.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T260 [GIT] Commit: finalize phase 9 retro

### Phase Completion
- [ ] T261 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T262 [GIT] Create/update PR to main with phase summary
- [ ] T263 [GIT] Verify all CI checks pass
- [ ] T264 [GIT] Report PR ready status

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories
- **Write Pipeline [US1] (Phase 3)**: Depends on Foundational
- **Read Pipeline [US2] (Phase 4)**: Depends on Foundational + US1 (pipeline package structure, LLM client)
- **MCP Server [US3] (Phase 5)**: Depends on US1 + US2 (complete service layer)
- **CLI [US4] (Phase 6)**: Depends on US3 (service layer proven via MCP tests)
- **Web UI [US5] (Phase 7)**: Depends on US1 + US2 (service layer)
- **Hook Integration [US6] (Phase 8)**: Depends on US3 (MCP server) + US2 (retrieval pipeline)
- **Polish (Phase 9)**: Depends on all user stories complete

### User Story Dependencies

```
Phase 1: Setup
    |
Phase 2: Foundational (Story 1 + Story 4 schema)
    |
    +---> Phase 3: US1 Write Pipeline (Story 2)
    |         |
    |         +---> Phase 4: US2 Read Pipeline (Story 3)
    |                   |
    |                   +---> Phase 5: US3 MCP Server (Story 5)
    |                   |         |
    |                   |         +---> Phase 6: US4 CLI (Story 6)
    |                   |         |
    |                   |         +---> Phase 8: US6 Hooks (Story 3 hooks)
    |                   |
    |                   +---> Phase 7: US5 Web UI (Story 7)
    |
    +---> Phase 9: Polish (after all stories)
```

### Within Each User Story

- Models/schemas before services
- Services before endpoint handlers
- Core implementation before integration
- Tests alongside implementation
- Story complete before next priority

### Parallel Opportunities

**Phase 2 (Foundational)**:
- T024/T025/T026 shared type files can be written in parallel
- T038-T042 migration files can be written in parallel
- T044-T046 index/FTS/trigger migrations in parallel
- T064-T066 storage tests in parallel

**Phase 3 (US1 Write Pipeline)**:
- T084/T085 credential check and quality gate in parallel

**Phase 5 (US3 MCP Server)**:
- T141-T147 all 7 tool handlers can be written in parallel

**Phase 6 (US4 CLI)**:
- T167-T175 all 9 CLI commands can be written in parallel

**Phase 7 (US5 Web UI)**:
- T203-T208 all 6 view components can be written in parallel

**Phase 9 (Polish)**:
- T249-T251 integration tests in parallel

---

## Parallel Example: Phase 5 MCP Tool Handlers

```bash
# All 7 tool handlers can run in parallel (different files, same interface):
Task: "Implement create-memory.ts tool handler"
Task: "Implement query.ts tool handler"
Task: "Implement expand.ts tool handler"
Task: "Implement get-memory.ts tool handler"
Task: "Implement update-memory.ts tool handler"
Task: "Implement feedback.ts tool handler"
Task: "Implement detect-environment.ts tool handler"
```

---

## Implementation Strategy

### MVP First (Phase 1-5: Setup + Foundation + US1 + US2 + US3)

1. Complete Phase 1: Setup — monorepo structure
2. Complete Phase 2: Foundational — storage layer + shared types
3. Complete Phase 3: US1 Write Pipeline — memory creation works
4. Complete Phase 4: US2 Read Pipeline — memory retrieval works
5. Complete Phase 5: US3 MCP Server — agents can interact
6. **STOP and VALIDATE**: MCP server fully functional with all 7 tools
7. Deploy MCP server as minimum viable product

### Incremental Delivery

1. Setup + Foundational → Data foundation ready
2. Add Write Pipeline → Memories can be created (MVP increment 1)
3. Add Read Pipeline → Memories can be queried (MVP increment 2)
4. Add MCP Server → Agents can use FixOnce (MVP!)
5. Add CLI → Humans can use FixOnce from terminal
6. Add Web UI → Full management interface
7. Add Hook Integration → Automatic memory surfacing
8. Each phase adds value without breaking previous phases

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story is independently completable and testable
- Commit after each task or logical group per git workflow
- Stop at any checkpoint to validate story independently
- Story 4 (Version-Scoped Metadata) is woven into Phases 2-4 rather than a separate phase
- Story 3 Scenario 10 (Agent Teams monitor) deferred as experimental per D17
