# Implementation Plan: FixOnce v2 — Clean Slate Rewrite

**Branch**: `cleanslate` | **Date**: 2026-03-23 | **Spec**: `discovery/SPEC.md`
**Input**: Vision document `docs/ideas/2026-03-23-fixonce-v2-vision.md` + 16 graduated user stories

## Summary

FixOnce v2 is a clean-slate rewrite of a shared memory layer for LLM coding agents. The system stores corrections, gotchas, best practices, and discoveries with rich Midnight-specific metadata, then surfaces them contextually using hybrid search and LLM-powered RAG pipelines. Memories are alive — they decay, reinforce, compete, and self-correct over time.

The implementation is a two-codebase architecture: a **Rust CLI/TUI** (primary interface for both humans and agents) and **Supabase Deno edge functions** (sole API surface to the database). All inference uses Claude via `claude -p`. Embeddings use VoyageAI voyage-code-3.

## Technical Context

**Languages/Runtimes**:
- Rust (stable, latest) — CLI binary, TUI, pipelines, environment detection
- TypeScript/Deno — Supabase edge functions only
- SQL/PL/pgSQL — Postgres migrations, RPC functions, triggers

**Primary Dependencies (Rust)**:
- `clap` — CLI argument parsing
- `ratatui` + `crossterm` — TUI rendering
- `reqwest` — HTTP client for Supabase edge functions and VoyageAI API
- `serde` + `serde_json` — JSON serialization
- `toon` — TOON output format
- `thiserror` — structured error types
- `tokio` — async runtime
- `ed25519-dalek` or `ring` — public-key cryptography for challenge-response auth
- `keyring` or custom — secure local private key storage (not secrets — just the CLI's own keypair)
- `indicatif` — progress bars for TTY mode
- `open` — browser launch for OAuth flow

**Primary Dependencies (Deno/Edge Functions)**:
- `zod` — input/output schema validation
- `@supabase/supabase-js` — Supabase client (used within edge functions for auth helpers)
- Web Crypto API (built-in) — AES-256-GCM encryption/decryption

**Storage**: Supabase Postgres with pgvector extension + pg_trgm + uuid-ossp
**Testing**: `cargo test` (Rust), `deno test` (edge functions)
**Target Platforms**: macOS ARM64, macOS x86_64, Linux x86_64 (statically linked binaries)
**Project Type**: Multi-codebase monorepo (Cargo workspace + Supabase project)

**Performance Goals**:
- Hybrid search: <500ms at 10,000 memories (SC-006)
- Hot cache queries: <50ms (FR-058)
- Secret retrieval: <300ms including decryption (SC-010)
- CLI challenge-response auth: <2 seconds (SC-008)
- Hook execution: <3 seconds, 95th percentile (SC-023)
- TUI render/input response: <100ms (SC-020)
- Pre-commit hooks: <30 seconds for 5-10 files (SC-003)

**Constraints**:
- Claude is a hard dependency — no offline/local LLM fallback
- VoyageAI voyage-code-3 for embeddings — not swappable
- Supabase is the sole backend — no self-hosted option
- Midnight-specific metadata schema — no generic multi-ecosystem
- Ship complete — no incremental/degraded releases
- Warn-only intervention — hooks never block agent actions

**Scale/Scope**:
- Initial: ~100-1,000 memories, 1 team (Midnight DevRel)
- Target: ~10,000+ memories, multiple teams (Midnight ecosystem)
- 15 CLI commands + TUI mode
- 7 database tables
- ~15 edge functions
- 8 query techniques, 7 result refinement techniques, 6 search modes
- 5 Claude Code hooks

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Edge Functions Are the API | ✅ PASS | All DB ops via edge functions. CLI never touches DB directly. RLS deny-by-default. |
| II. Secrets Never Touch Disk | ✅ PASS | AES-256-GCM in edge functions, master key as env secret. CLI ephemeral retrieval. |
| III. Unix CLI Philosophy | ✅ PASS | text/json/toon formats, stdin/stdout/stderr, exit codes, non-TTY detection. |
| IV. Strict Rust Discipline | ✅ PASS | clippy pedantic, thiserror, no unwrap in library code. |
| V. Deno Strict Mode | ✅ PASS | deno lint/fmt/check, Zod schemas, deno test. |
| VI. Fail Fast with Actionable Errors | ✅ PASS | Structured errors with what/why/action for both human and agent formats. |
| VII. Comprehensive Testing | ✅ PASS | cargo test + deno test. Integration tests for auth, RLS, embeddings. |
| VIII. Conventional Commits | ✅ PASS | type(scope): subject format. Scopes defined. |
| IX. API-First Design | ✅ PASS | Edge function Zod schemas designed before implementation. |
| X. Simplicity Until Proven Otherwise | ⚠️ WATCH | Full RAG pipeline menu is ambitious — monitor for over-engineering. |

## Project Structure

### Documentation (this feature)

```text
specs/001-fixonce-v2/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output (edge function schemas)
└── tasks.md             # Phase 2 output (/sdd:tasks)
```

### Source Code (repository root)

```text
fixonce/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── Makefile                      # Top-level commands (make check, make build, etc.)
├── lefthook.yml                  # Git hook configuration
├── .github/
│   └── workflows/
│       ├── ci.yml                # PR checks (Rust + Deno)
│       └── release.yml           # Binary release builds
├── fixonce-mascot.png
│
├── crates/                       # Rust workspace members
│   ├── fixonce-cli/              # Binary crate — CLI entry point + TUI
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs           # CLI argument parsing, command dispatch
│   │       ├── commands/         # One module per CLI command
│   │       │   ├── mod.rs
│   │       │   ├── login.rs      # GitHub OAuth browser flow
│   │       │   ├── auth.rs       # Challenge-response auth
│   │       │   ├── keys.rs       # Key management (add/list/revoke)
│   │       │   ├── create.rs     # Memory creation (invokes write pipeline)
│   │       │   ├── query.rs      # Memory query (invokes read pipeline)
│   │       │   ├── get.rs        # Get memory by ID
│   │       │   ├── update.rs     # Update memory
│   │       │   ├── delete.rs     # Soft-delete memory
│   │       │   ├── feedback.rs   # Submit feedback
│   │       │   ├── lineage.rs    # View provenance chain
│   │       │   ├── detect.rs     # Environment detection
│   │       │   ├── context.rs    # Context gathering
│   │       │   ├── analyze.rs    # Session transcript analysis
│   │       │   └── config.rs     # CLI configuration
│   │       └── tui/              # TUI module (ratatui)
│   │           ├── mod.rs
│   │           ├── app.rs        # App state and event loop
│   │           ├── views/        # TUI screens
│   │           │   ├── dashboard.rs
│   │           │   ├── memory_list.rs
│   │           │   ├── memory_detail.rs
│   │           │   ├── create_form.rs
│   │           │   ├── activity.rs
│   │           │   ├── keys.rs
│   │           │   ├── secrets.rs
│   │           │   └── health.rs
│   │           └── widgets/      # Reusable TUI components
│   │
│   ├── fixonce-core/             # Library crate — shared logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── api/              # Supabase edge function client
│   │       │   ├── mod.rs
│   │       │   ├── client.rs     # HTTP client with auth headers
│   │       │   ├── memories.rs   # Memory CRUD operations
│   │       │   ├── search.rs     # Search endpoint wrapper
│   │       │   ├── feedback.rs   # Feedback operations
│   │       │   ├── secrets.rs    # Secret retrieval (ephemeral)
│   │       │   ├── auth.rs       # Login, challenge-response, key mgmt
│   │       │   └── activity.rs   # Activity log queries
│   │       ├── auth/             # Auth logic
│   │       │   ├── mod.rs
│   │       │   ├── oauth.rs      # GitHub OAuth browser flow
│   │       │   ├── keypair.rs    # Ed25519 key generation/storage
│   │       │   ├── challenge.rs  # Challenge-response protocol
│   │       │   └── token.rs      # JWT management
│   │       ├── pipeline/         # Read/write pipelines
│   │       │   ├── mod.rs
│   │       │   ├── write/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── credential_check.rs
│   │       │   │   ├── quality_gate.rs
│   │       │   │   ├── dedup.rs
│   │       │   │   └── enrichment.rs
│   │       │   ├── read/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── query_techniques.rs   # All 8 query techniques
│   │       │   │   ├── result_refinement.rs  # All 7 refinement techniques
│   │       │   │   ├── search_modes.rs       # All 6 search modes
│   │       │   │   └── pipeline_runner.rs    # Composable pipeline executor
│   │       │   └── claude.rs     # Claude CLI wrapper (claude -p)
│   │       ├── memory/           # Memory model types
│   │       │   ├── mod.rs
│   │       │   ├── types.rs      # Memory, AntiMemory, Feedback structs
│   │       │   ├── metadata.rs   # Midnight version metadata
│   │       │   ├── provenance.rs # Source tracking
│   │       │   ├── dynamics.rs   # Decay, reinforcement, scoring
│   │       │   ├── lineage.rs    # Lineage tracking
│   │       │   ├── signatures.rs # Memory signatures and hot cache
│   │       │   └── contradictions.rs # Contradiction detection/resolution
│   │       ├── detect/           # Environment detection
│   │       │   ├── mod.rs
│   │       │   ├── midnight.rs   # Midnight-specific version detection
│   │       │   └── context.rs    # Project context gathering
│   │       ├── embeddings/       # VoyageAI integration
│   │       │   ├── mod.rs
│   │       │   └── voyage.rs     # voyage-code-3 embedding generation
│   │       ├── output/           # Output formatting
│   │       │   ├── mod.rs
│   │       │   ├── text.rs       # Human-readable text
│   │       │   ├── json.rs       # JSON output
│   │       │   └── toon.rs       # TOON output
│   │       └── error.rs          # Structured error types (thiserror)
│   │
│   └── fixonce-hooks/            # Library crate — Claude Code hook scripts
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── session_start.rs
│           ├── user_prompt.rs
│           ├── pre_tool_use.rs
│           ├── post_tool_use.rs
│           └── stop.rs
│
├── supabase/                     # Supabase project
│   ├── config.toml               # Supabase project config
│   ├── migrations/               # SQL migrations (Supabase CLI managed)
│   │   ├── 001_extensions.sql
│   │   ├── 002_tables.sql
│   │   ├── 003_rls_policies.sql
│   │   ├── 004_indexes.sql
│   │   ├── 005_fts_setup.sql
│   │   ├── 006_hybrid_search_rpc.sql
│   │   ├── 007_triggers.sql
│   │   └── 008_cron_jobs.sql
│   └── functions/                # Deno edge functions
│       ├── _shared/              # Shared utilities
│       │   ├── auth.ts           # Auth verification helper
│       │   ├── validate.ts       # Zod schema validation helper
│       │   ├── errors.ts         # Structured error responses
│       │   ├── activity.ts       # Activity logging helper
│       │   └── cors.ts           # CORS headers
│       ├── memory-create/
│       │   └── index.ts
│       ├── memory-get/
│       │   └── index.ts
│       ├── memory-update/
│       │   └── index.ts
│       ├── memory-delete/
│       │   └── index.ts
│       ├── memory-search/
│       │   └── index.ts
│       ├── feedback-submit/
│       │   └── index.ts
│       ├── secret-get/
│       │   └── index.ts
│       ├── secret-create/
│       │   └── index.ts
│       ├── secret-rotate-master/
│       │   └── index.ts
│       ├── auth-nonce/
│       │   └── index.ts
│       ├── auth-verify/
│       │   └── index.ts
│       ├── auth-org-check/
│       │   └── index.ts
│       ├── keys-register/
│       │   └── index.ts
│       ├── keys-list/
│       │   └── index.ts
│       ├── keys-revoke/
│       │   └── index.ts
│       └── activity-stream/
│           └── index.ts
│
├── hooks/                        # Claude Code hook shell scripts
│   ├── session-start.sh
│   ├── user-prompt-submit.sh
│   ├── pre-tool-use.sh
│   ├── post-tool-use.sh
│   └── stop.sh
│
├── .sdd/                         # SDD artifacts
├── .claude/                      # Claude Code settings
├── discovery/                    # Specification discovery
├── docs/                         # Vision, plans
└── specs/                        # Implementation specs
```

**Structure Decision**: Cargo workspace with two library crates (`fixonce-core`, `fixonce-hooks`) and one binary crate (`fixonce-cli`). This separates the CLI/TUI entry point from reusable logic, enabling the hooks crate to share core functionality without pulling in CLI dependencies. Supabase lives in its own `supabase/` directory following Supabase CLI conventions.

## Implementation Phases

### Phase 0: Foundation & Dev Tooling (Story 1)

**Goal**: Establish the monorepo structure, quality gates, and CI before any feature code.

**Tasks**:
1. Initialize Cargo workspace with three crates (fixonce-cli, fixonce-core, fixonce-hooks)
2. Initialize Supabase project (`supabase init`)
3. Configure Lefthook (`lefthook.yml`) with two tracks:
   - Rust: `cargo fmt --check` + `cargo clippy -- -D warnings` on `*.rs` files
   - Deno: `deno fmt --check` + `deno lint` on `supabase/functions/**/*.ts`
4. Create Makefile with targets: `check`, `build`, `test`, `fmt`, `lint`
5. Create GitHub Actions CI workflow (`ci.yml`):
   - Job 1 (Rust): cargo fmt, clippy, test, audit (advisory)
   - Job 2 (Deno): deno fmt --check, deno lint, deno check
   - Cache: Cargo registry + target (keyed on Cargo.lock hash), Deno cache
6. Create GitHub Actions release workflow (`release.yml`):
   - Matrix build: macOS ARM64, macOS x86_64, Linux x86_64
   - Upload binaries as release artifacts
7. Configure branch protection rules on `main`
8. Create initial `.gitignore` (Rust + Deno + Supabase patterns)
9. Set clippy pedantic config in root `Cargo.toml`:
   ```toml
   [workspace.lints.clippy]
   all = { level = "deny" }
   pedantic = { level = "deny" }
   ```

**Exit Criteria**: `make check` passes on empty workspace. CI runs on PR. Lefthook hooks fire on commit.

### Phase 1: Database & Edge Function Foundation (Story 2)

**Goal**: Set up Supabase schema, RLS, and core edge functions.

**Tasks**:
1. Write SQL migrations (001-008):
   - 001: Enable extensions (pgvector, uuid-ossp, pg_trgm, pg_cron)
   - 002: Create all 7 tables with columns, constraints, defaults
   - 003: RLS policies — deny-by-default on all tables, allow via auth.uid()
   - 004: Indexes — HNSW on embedding column, GIN on tsvector, btree on foreign keys
   - 005: FTS setup — weighted tsvector column with trigger (title=A, summary=B, content=C)
   - 006: Hybrid search RPC function (Reciprocal Rank Fusion)
   - 007: Triggers — updated_at timestamps, soft-delete cascades
   - 008: Cron jobs — activity_log retention (90-day cleanup), org membership sweep (twice daily)
2. Create shared edge function utilities (`_shared/`):
   - `auth.ts` — verify JWT via `supabase.auth.getUser()`, extract user_id
   - `validate.ts` — Zod schema validation wrapper returning structured errors
   - `errors.ts` — structured error response builder (status, type, message, action)
   - `activity.ts` — activity log insertion helper
3. Create initial edge functions:
   - `memory-create/` — validate schema, insert memory, return ID
   - `memory-get/` — retrieve by ID (exclude embedding by default)
   - `memory-search/` — accept search_type parameter, dispatch to hybrid/fts/vector
4. Run `supabase db push` to verify migrations
5. Write `deno test` integration tests for RLS enforcement

**Exit Criteria**: All tables exist with RLS. Hybrid search RPC returns results. Edge functions validate auth and input. Tests pass.

### Phase 2: Authentication & Secrets (Stories 3-4)

**Goal**: Implement GitHub OAuth, CLI public-key auth, org membership verification, and encrypted secrets.

**Tasks**:
1. Configure GitHub OAuth in Supabase dashboard
2. Implement auth edge functions:
   - `auth-nonce/` — generate and store nonce for challenge-response
   - `auth-verify/` — verify Ed25519 signature, issue custom JWT (8hr expiry)
   - `auth-org-check/` — check GitHub org membership via GitHub API, cache result (1hr TTL)
3. Implement key management edge functions:
   - `keys-register/` — store public key linked to user
   - `keys-list/` — return user's registered keys
   - `keys-revoke/` — delete key, invalidate associated JWTs
4. Implement Rust auth module:
   - OAuth browser flow (open browser → callback server → receive JWT)
   - Ed25519 keypair generation and local storage (keyring or file)
   - Challenge-response protocol implementation
   - JWT storage and refresh logic
5. Implement secrets edge functions:
   - `secret-create/` — encrypt with AES-256-GCM (master key from env), store ciphertext
   - `secret-get/` — decrypt on request, log access, return plaintext
   - `secret-rotate-master/` — re-encrypt all secrets in transaction
6. Implement Rust secrets client — ephemeral retrieval, use, discard
7. Implement org membership cron job (twice-daily sweep)
8. Write integration tests: auth flow, key registration, secret encryption/decryption, org check

**Exit Criteria**: Full auth flow works (login → JWT → authenticated requests). Secrets encrypted at rest, decryptable only via edge function. Org check blocks non-members.

### Phase 3: Memory CRUD & Embeddings (Story 5)

**Goal**: Core memory operations with VoyageAI embedding generation.

**Tasks**:
1. Implement remaining memory edge functions:
   - `memory-update/` — update memory, handle embedding regeneration flag
   - `memory-delete/` — soft-delete, preserve lineage
   - `feedback-submit/` — store feedback, update memory scores
2. Implement Rust VoyageAI client:
   - Request API key from secrets endpoint
   - Generate voyage-code-3 embeddings (1024 dims)
   - Discard API key after use
   - Retry with exponential backoff on failure
   - Handle "pending_embedding" fallback (EC-25)
3. Implement Rust memory types (`fixonce-core/src/memory/`):
   - Memory struct with all fields
   - Midnight version metadata types
   - Provenance types
   - Memory type and source type enums
4. Implement Rust API client for memory CRUD
5. Implement `fixonce create`, `fixonce get`, `fixonce update`, `fixonce delete`, `fixonce feedback` commands
6. Implement output formatting (text/json/toon) for memory responses
7. Write tests: CRUD operations, embedding generation, output formatting
8. Implement environment detection stub in `crates/fixonce-core/src/detect/mod.rs` — minimal module that returns `None` for all version fields. This allows the read pipeline (Phase 5) to call detection without blocking on the full implementation in Phase 7. The stub is replaced by the real implementation in Phase 7.
9. Implement memory signature computation at creation time in `crates/fixonce-core/src/memory/signatures.rs` — compute file patterns, error patterns, SDK method calls from memory content when a memory is created or updated. Store as a JSONB column on the memory table. This is the "write path" for signatures; the "read path" (session hot cache) is built in Phase 7.

**Exit Criteria**: Can create memory with embedding, query it, update it, soft-delete it. All output formats work. Feedback recorded.

### Phase 4: Write Pipeline (Story 6)

**Goal**: Quality gating, deduplication, and metadata enrichment.

**Tasks**:
1. Implement Claude CLI wrapper (`pipeline/claude.rs`):
   - Shell out to `claude -p --output-format json`
   - Parse JSON responses
   - Handle timeouts and retries
   - Detect missing Claude CLI (EC-37)
2. Implement credential/PII detection (`pipeline/write/credential_check.rs`):
   - Regex patterns for API keys, private keys, passwords, emails, etc.
   - No LLM needed — pure pattern matching for speed
3. Implement quality gate (`pipeline/write/quality_gate.rs`):
   - Claude prompt: assess actionability, specificity, signal-to-noise
   - Return accept/reject with rationale
4. Implement dedup (`pipeline/write/dedup.rs`):
   - Fetch top-N similar memories by embedding cosine similarity
   - Claude prompt: compare candidate vs existing, return outcome (new/discard/replace/update/merge)
   - Handle each outcome: create lineage records, soft-delete, merge content
5. Implement metadata enrichment (`pipeline/write/enrichment.rs`):
   - Auto-detect language if not specified
   - Suggest memory_type if ambiguous
   - Flag missing version metadata
6. Wire pipeline into `fixonce create` command
7. Write tests: credential detection, quality gate (mock Claude), dedup outcomes, lineage creation

**Exit Criteria**: `fixonce create` runs full write pipeline. Credentials blocked. Duplicates detected. Lineage created for replace/update/merge.

### Phase 5: Read Pipeline (Story 7)

**Goal**: Full RAG query pipeline with composable techniques.

**Tasks**:
1. Implement pipeline runner (`pipeline/read/pipeline_runner.rs`):
   - Composable stage architecture — each technique is a function
   - Default pipeline: query rewriting → hybrid search → relevance reranking
   - Deep pipeline (`--deep`): multi-query → HyDE → hybrid → retrieve-read-retrieve → confidence → reranking → coverage
   - Custom pipeline via flags
2. Implement all 8 query techniques (`pipeline/read/query_techniques.rs`):
   - Each technique: Claude prompt → structured output → next stage input
3. Implement all 7 result refinement techniques (`pipeline/read/result_refinement.rs`)
4. Implement all 6 search modes (`pipeline/read/search_modes.rs`):
   - Hybrid/FTS/vector via edge function search endpoint
   - Metadata filtering via query parameters
   - Graph-assisted via lineage/contradiction queries
   - Parent-child via lineage chain traversal
   - Field-aware via weighted search parameters
   - Passage compression via Claude
5. Wire into `fixonce query` command with flags (`--deep`, `--version`, `--format`)
6. Handle degraded mode: Claude outage → return raw search results marked "unranked" (EC-29)
7. Write tests: pipeline composition, each technique in isolation, degraded mode

**Exit Criteria**: `fixonce query` returns relevant results. Default and deep pipelines work. Version filtering works. All output formats work.

### Phase 6: Memory Dynamics (Stories 8-12)

**Goal**: Decay, reinforcement, anti-memories, contradictions, lineage, and signatures.

**Tasks**:
1. Implement decay system (`memory/dynamics.rs`):
   - Temporal decay function (exponential with configurable half-life)
   - Event-driven decay triggers (via CLI command or cron)
   - Decay threshold soft-deletion
   - Reinforcement score updates on access and positive feedback
2. Implement anti-memory support (`memory/types.rs`):
   - Anti-memory creation with description, reason, alternative, version constraints
   - Priority boosting in search results for matching version constraints
   - Auto-proposal from negative feedback patterns (write pipeline integration)
3. Implement contradiction detection (`memory/contradictions.rs`):
   - Detection during read pipeline (Claude identifies contradictions)
   - Storage in contradiction_pairs table via edge function
   - Tiebreaker vote recording
   - Resolution logic (3+ votes → apply decay/reinforcement)
4. Implement lineage tracking (`memory/lineage.rs`):
   - Automatic lineage creation on replace/update/merge/feedback
   - `fixonce lineage <id>` command for on-demand retrieval
   - Lineage display in TUI detail view
5. Implement session hot cache in `crates/fixonce-core/src/memory/signatures.rs` — session relevance profile matching using cosine similarity of session profile vector to pre-computed memory signatures. In-memory HashMap cache, capped at 50 memories (LRU eviction when cap exceeded). Cache is populated on session start (Phase 9 hooks), refreshed on cache miss only. This builds on the signature computation from Phase 3.
6. Implement decay cron edge function (or CLI scheduled task)
7. Write tests: decay curves, reinforcement, anti-memory priority, contradiction resolution, lineage chain, signature matching, hot cache performance

**Exit Criteria**: Memories decay over time. Feedback affects scores. Anti-memories surface with priority. Contradictions detected and resolvable. Lineage queryable. Hot cache <50ms.

### Phase 7: Environment Detection & Session Analysis (Stories 14-15)

**Goal**: Project scanning and passive memory harvesting.

**Tasks**:
1. Implement environment detection (`detect/midnight.rs`):
   - Scan package.json for midnight-js versions
   - Scan .compact files for pragma versions
   - Scan compiler config for version
   - Scan for other ecosystem markers
   - Output structured JSON for use by other commands
2. Implement context gathering (`detect/context.rs`):
   - Detected versions + git remote + branch + recent commits + file structure
3. Implement session transcript analysis (`commands/analyze.rs`):
   - Parse Claude Code session log format
   - Claude prompt: identify corrections, discoveries, gotchas, best practices
   - Present candidates with confidence scores
   - Interactive accept/edit/skip/reject flow (TTY only)
   - Accepted candidates enter full write pipeline
4. Wire auto-detection into `fixonce query` (use detected versions for filtering unless overridden)
5. Write tests: detection accuracy for various project layouts, analysis of sample sessions

**Exit Criteria**: `fixonce detect` correctly identifies Midnight components. `fixonce context` gathers project metadata. `fixonce analyze` proposes relevant candidate memories from session logs.

### Phase 8: TUI (Story 13 — TUI portion)

**Goal**: Rich terminal UI for admin operations.

**Tasks**:
1. Implement TUI app state and event loop (`tui/app.rs`)
2. Implement TUI views:
   - Dashboard: search bar, memory list, activity sidebar
   - Memory list: filterable, sortable, shows key metadata
   - Memory detail: full content, metadata, scores, provenance, feedback history
   - Create form: all memory fields with validation
   - Activity stream: polling for recent activity_log entries
   - Key management: list/revoke CLI keys
   - Secret management: create/view/rotate (admin only)
   - System health: memory count, avg scores, decay stats
3. Implement keyboard navigation and shortcuts
4. Handle terminal resize, minimum size (EC-35)
5. Handle non-TTY detection (EC-36)
6. Write tests: TUI state transitions, keyboard navigation

**Exit Criteria**: `fixonce tui` launches a functional admin interface. All views render correctly. Keyboard navigation works. Minimum terminal size enforced.

### Phase 9: Claude Code Hooks (Story 16)

**Goal**: Automatic memory surfacing during coding sessions.

**Tasks**:
1. Implement hook shell scripts (hooks/):
   - session-start.sh: run `fixonce detect`, populate hot cache, surface top 3 memories
   - user-prompt-submit.sh: lightweight query on prompt text
   - pre-tool-use.sh: check proposed content against anti-memories (score > 0.7)
   - post-tool-use.sh: check written content against anti-memories (score > 0.5)
   - stop.sh: surface critical reminders for project context
2. Implement hook timeout handling (3 second max, EC-41)
3. Implement missing CLI detection (EC-42)
4. Implement unauthenticated fallback (EC-43 — skip silently)
5. Create hooks settings template for Claude Code configuration
6. Write tests: hook timeout, missing CLI graceful degradation

**Exit Criteria**: All 5 hooks fire at correct lifecycle points. Anti-memory warnings surface. Hooks timeout gracefully. Missing CLI doesn't block agent.

### Phase 10: Integration Testing & Polish

**Goal**: End-to-end testing across all components.

**Tasks**:
1. End-to-end test: create memory → query it → get feedback → watch decay
2. End-to-end test: auth flow → create memory → query with version filter
3. End-to-end test: write pipeline dedup → lineage creation → lineage query
4. End-to-end test: contradiction detection → tiebreaker vote → resolution
5. End-to-end test: session analysis → candidate proposal → write pipeline → memory stored
6. Performance benchmarks: hybrid search at 1k/10k memories, hot cache timing
7. Binary release builds: verify cross-platform compilation
8. Documentation: README, installation instructions, configuration guide

**Exit Criteria**: All end-to-end tests pass. Performance within spec. Binaries build for all targets. Documentation complete.

## Complexity Tracking

| Concern | Why Needed | Simplicity Check |
|---------|------------|-----------------|
| Full RAG pipeline (8 query techniques) | Core product value — relevance quality determines trust | Each technique is a composable function, not a monolith. Remove any that don't measurably improve results (Constitution §X). |
| Memory dynamics (decay + reinforcement) | Self-correcting knowledge is the core differentiator vs static KBs | Simple exponential decay + additive reinforcement. No ML models, no complex scheduling. |
| Contradiction courts | Conflicting memories confuse agents without resolution mechanism | Minimal: flag pair, record votes, apply threshold. No sophisticated consensus algorithms. |
| Three Rust crates | Separation of CLI, core logic, and hooks | Core must be reusable by hooks without CLI dependencies. Two crates would force hooks to depend on CLI binary. Three is the minimum. |

## Open Questions (from Vision)

These are tracked but NOT blocking implementation. Reasonable defaults are used; calibration happens post-launch with real usage data.

| Question | Default | Revisit When |
|----------|---------|-------------|
| Decay half-life per memory type | 30 days for all types | After 90 days of usage data |
| Contradiction court quorum | 3 tiebreaker votes | After 50+ contradiction resolutions |
| Signature cache refresh strategy | In-memory HashMap, LRU eviction at 50-cap, refresh on cache miss. Cosine similarity of session profile vector to memory signature vectors. | After measuring session cache hit rates |
| Harvesting signal-to-noise threshold | Confidence > 0.7 to propose | After 100+ analyzed sessions |
| Multi-tenant isolation model | Single shared store (v1 scope) | When ecosystem developers onboard |
| Secret rotation model | Manual rotation via admin | When automated rotation is needed |
