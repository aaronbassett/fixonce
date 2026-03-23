# Tasks: FixOnce v2

**Traceability**: Tasks map to user stories (US1-US16) via [Story] labels. Functional requirements (FR-001 through FR-073) are implemented by the tasks in the corresponding story's phase. Edge cases (EC-01 through EC-43) are handled by dedicated edge case tasks (T028a, T059a, T098a, T136a, T157a, T179a, T207a, T230a) within each phase. Success criteria (SC-001 through SC-024) are verified by test tasks and Phase 11 benchmarks; SC-015, SC-016, SC-017, SC-024 require post-launch monitoring data.

**Input**: Design documents from `/specs/001-fixonce-v2/`
**Prerequisites**: plan.md, discovery/SPEC.md, research.md, data-model.md, contracts/edge-functions.md
**Constitution**: `.sdd/memory/constitution.md` v1.1.0

**Tests**: Comprehensive testing is required per Constitution §VII. Test tasks are included for all stories.

**Organization**: Tasks grouped by implementation phase (matching plan.md). Each phase maps to one or more user stories and is independently testable.

## Format: `[ID] [P?] [Story?] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story (US1-US16 from discovery/SPEC.md)
- **[GIT]**: Git workflow task (commit, push, PR)
- Include exact file paths in descriptions

## Path Conventions

```text
crates/fixonce-cli/src/       # Binary crate (CLI entry + TUI)
crates/fixonce-core/src/      # Library crate (shared logic)
crates/fixonce-hooks/src/     # Library crate (hook implementations)
supabase/migrations/          # SQL migrations
supabase/functions/           # Deno edge functions
hooks/                        # Claude Code hook shell scripts
```

## Dependencies

```text
Phase 1 (Setup) ──► Phase 2 (DB Foundation) ──► Phase 3 (Auth & Secrets)
                                                       │
                                    ┌──────────────────┤
                                    ▼                  ▼
                             Phase 4 (CRUD)     Phase 9 (TUI)
                                    │
                        ┌───────────┼───────────┐
                        ▼           ▼           ▼
                 Phase 5 (Write) Phase 6 (Read) Phase 7 (Dynamics)
                        │           │           │
                        └───────────┼───────────┘
                                    ▼
                             Phase 8 (Detection & Analysis)
                                    ▼
                             Phase 10 (Hooks)
                                    ▼
                             Phase 11 (Polish)
```

---

## Phase 1: Setup & Dev Tooling [US1]

**Goal**: Monorepo structure, quality gates, CI — before any feature code.
**Independent Test**: `make check` passes on empty workspace. CI runs on PR.

### Phase Start
- [ ] T001 [GIT] Verify on main branch and working tree is clean
- [ ] T002 [GIT] Pull latest changes from origin/main
- [ ] T003 [GIT] Create feature branch: 001-fixonce-v2

### Implementation
- [ ] T004 [US1] Create Cargo workspace root with `Cargo.toml` (members: crates/fixonce-cli, crates/fixonce-core, crates/fixonce-hooks) (use devs:rust-dev agent)
- [ ] T005 [GIT] Commit: initialize Cargo workspace
- [ ] T006 [P] [US1] Create `crates/fixonce-cli/Cargo.toml` with clap, ratatui, crossterm dependencies (use devs:rust-dev agent)
- [ ] T007 [P] [US1] Create `crates/fixonce-core/Cargo.toml` with reqwest, serde, thiserror, tokio, ed25519-dalek dependencies (use devs:rust-dev agent)
- [ ] T008 [P] [US1] Create `crates/fixonce-hooks/Cargo.toml` with fixonce-core dependency (use devs:rust-dev agent)
- [ ] T009 [GIT] Commit: add crate manifests with dependencies
- [ ] T010 [P] [US1] Create minimal `crates/fixonce-cli/src/main.rs` with clap setup and `--version` flag (use devs:rust-dev agent)
- [ ] T011 [P] [US1] Create minimal `crates/fixonce-core/src/lib.rs` with module stubs (use devs:rust-dev agent)
- [ ] T012 [P] [US1] Create minimal `crates/fixonce-hooks/src/lib.rs` with module stubs (use devs:rust-dev agent)
- [ ] T013 [GIT] Commit: add crate entry points
- [ ] T014 [US1] Initialize Supabase project: `supabase init` in `supabase/` directory
- [ ] T015 [GIT] Commit: initialize Supabase project
- [ ] T016 [US1] Create `.gitignore` with Rust, Deno, Supabase, and editor patterns
- [ ] T017 [GIT] Commit: add .gitignore
- [ ] T018 [US1] Configure clippy pedantic in workspace `Cargo.toml` lints section: `[workspace.lints.clippy] all = "deny"`, `pedantic = "deny"` (use devs:rust-dev agent)
- [ ] T019 [GIT] Commit: configure clippy pedantic lints
- [ ] T020 [US1] Create `lefthook.yml` with two hooks: pre-commit (Rust: `cargo fmt --check` + `cargo clippy` on `*.rs`; Deno: `deno fmt --check` + `deno lint` on `supabase/functions/**/*.ts`)
- [ ] T021 [GIT] Commit: add Lefthook configuration
- [ ] T022 [US1] Create `Makefile` with targets: `check` (runs all quality checks), `build`, `test`, `fmt`, `lint`, `clippy`
- [ ] T023 [GIT] Commit: add Makefile with quality targets
- [ ] T024 [US1] Create `.github/workflows/ci.yml` with two parallel jobs: Rust (fmt, clippy, test, audit advisory) and Deno (fmt, lint, check). Cache Cargo registry+target keyed on Cargo.lock hash.
- [ ] T025 [GIT] Commit: add GitHub Actions CI workflow
- [ ] T026 [US1] Create `.github/workflows/release.yml` with matrix build: macOS ARM64, macOS x86_64, Linux x86_64. Upload binaries as release artifacts.
- [ ] T027 [GIT] Commit: add GitHub Actions release workflow
- [ ] T028 [US1] Verify `make check` passes on the workspace (cargo fmt, clippy, test all succeed)
- [ ] T028a [US1] Handle edge cases: EC-01 (agent --no-verify bypass — verify CI catches what hooks miss), EC-02 (Rust-only commits skip Deno hooks via Lefthook globs), EC-03 (docs-only commits skip code checks), EC-04 (cargo audit advisory-only in CI), EC-05 (cache keys include lockfile hashes)
- [ ] T029 [GIT] Commit: verify quality gates pass

### Phase Completion
- [ ] T030 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T031 [GIT] Create/update PR to main with Phase 1 summary
- [ ] T032 [GIT] Verify all CI checks pass
- [ ] T033 [GIT] Report PR ready status

---

## Phase 2: Database Foundation & Edge Functions [US2]

**Goal**: Supabase schema, RLS, indexes, core edge functions, hybrid search RPC.
**Independent Test**: Migrations apply cleanly. RLS blocks unauthenticated access. Hybrid search returns results.

### Phase Start
- [ ] T034 [GIT] Verify working tree is clean before starting Phase 2
- [ ] T035 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T036 [US2] Create retro/P2.md for this phase
- [ ] T037 [GIT] Commit: initialize phase 2 retro
- [ ] T038 [US2] Create `supabase/migrations/001_extensions.sql` — enable pgvector, uuid-ossp, pg_trgm, pg_cron extensions
- [ ] T039 [US2] Create `supabase/migrations/002_tables.sql` — all 7 tables (memory, feedback, activity_log, secrets, cli_keys, memory_lineage, contradiction_pairs) per data-model.md
- [ ] T040 [US2] Create `supabase/migrations/003_rls_policies.sql` — deny-by-default RLS on all tables per data-model.md RLS policy summary
- [ ] T041 [US2] Create `supabase/migrations/004_indexes.sql` — HNSW on embedding, GIN on fts_vector, btree indexes per data-model.md
- [ ] T042 [US2] Create `supabase/migrations/005_fts_setup.sql` — weighted tsvector column with trigger (title=A, summary=B, content=C)
- [ ] T043 [US2] Create `supabase/migrations/006_hybrid_search_rpc.sql` — Reciprocal Rank Fusion function per data-model.md RPC spec
- [ ] T044 [US2] Create `supabase/migrations/007_triggers.sql` — updated_at timestamps, soft-delete cascade triggers
- [ ] T045 [US2] Create `supabase/migrations/008_cron_jobs.sql` — activity_log 90-day retention cleanup via pg_cron
- [ ] T046 [GIT] Commit: add all SQL migrations
- [ ] T047 [US2] Create `supabase/functions/_shared/auth.ts` — verify JWT via supabase.auth.getUser(), extract user_id
- [ ] T048 [US2] Create `supabase/functions/_shared/validate.ts` — Zod schema validation wrapper returning structured errors
- [ ] T049 [US2] Create `supabase/functions/_shared/errors.ts` — structured error response builder (status, type, message, action) per Constitution §VI
- [ ] T050 [US2] Create `supabase/functions/_shared/activity.ts` — activity log insertion helper
- [ ] T051 [US2] Create `supabase/functions/_shared/cors.ts` — CORS headers for edge functions
- [ ] T052 [GIT] Commit: add shared edge function utilities
- [ ] T053 [US2] Create `supabase/functions/memory-create/index.ts` — validate schema (Zod), verify auth, insert memory, log activity, return ID. Per contracts/edge-functions.md
- [ ] T054 [US2] Create `supabase/functions/memory-get/index.ts` — verify auth, retrieve by ID, exclude embedding by default. Per contracts/edge-functions.md
- [ ] T055 [US2] Create `supabase/functions/memory-search/index.ts` — accept search_type param (hybrid|fts|vector), dispatch to appropriate search, apply version filters. Per contracts/edge-functions.md
- [ ] T056 [GIT] Commit: add core memory edge functions
- [ ] T057 [US2] Write `deno test` integration tests for RLS enforcement — verify unauthenticated queries return zero rows
- [ ] T058 [US2] Write `deno test` for memory-create validation — verify Zod schema rejects invalid input
- [ ] T059 [US2] Write `deno test` for memory-search — verify hybrid/fts/vector modes return expected results
- [ ] T059a [US2] Handle edge cases: EC-06 (idempotent migrations via Supabase CLI tracking), EC-07 (vector(1024) dimension constraint in migration), EC-08 (tsvector query input sanitization for special characters), EC-09 (empty vector results return empty array not error), EC-10 (malformed JSON returns 400 with structured error), EC-11 (activity_log 90-day retention cron cleanup)
- [ ] T060 [GIT] Commit: add edge function tests
- [ ] T061 [US2] Review retro/P2.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T062 [GIT] Commit: finalize phase 2 retro

### Phase Completion
- [ ] T063 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T064 [GIT] Create/update PR to main with Phase 2 summary
- [ ] T065 [GIT] Verify all CI checks pass
- [ ] T066 [GIT] Report PR ready status

---

## Phase 3: Authentication & Secrets [US3, US4]

**Goal**: GitHub OAuth, CLI public-key auth, org membership verification, encrypted secrets.
**Independent Test**: Login flow works. Challenge-response issues JWT. Secrets encrypted at rest, decryptable via edge function. Non-org members blocked.

### Phase Start
- [ ] T067 [GIT] Verify working tree is clean before starting Phase 3
- [ ] T068 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T069 [US3] Create retro/P3.md for this phase
- [ ] T070 [GIT] Commit: initialize phase 3 retro
- [ ] T071 [US3] Create `supabase/functions/auth-nonce/index.ts` — generate nonce for challenge-response, store with 5min TTL. Per contracts/edge-functions.md
- [ ] T072 [US3] Create `supabase/functions/auth-verify/index.ts` — verify Ed25519 signature against registered public key, issue 8hr JWT. Per contracts/edge-functions.md
- [ ] T073 [US3] Create `supabase/functions/auth-org-check/index.ts` — check GitHub org membership via GitHub API, cache result (1hr TTL). Per contracts/edge-functions.md
- [ ] T074 [GIT] Commit: add auth edge functions
- [ ] T075 [P] [US3] Create `supabase/functions/keys-register/index.ts` — store public key linked to user. Per contracts/edge-functions.md
- [ ] T076 [P] [US3] Create `supabase/functions/keys-list/index.ts` — return user's registered keys. Per contracts/edge-functions.md
- [ ] T077 [P] [US3] Create `supabase/functions/keys-revoke/index.ts` — delete key, invalidate JWTs. Per contracts/edge-functions.md
- [ ] T078 [GIT] Commit: add key management edge functions
- [ ] T079 [US3] Add org membership sweep to `supabase/migrations/008_cron_jobs.sql` — twice-daily cron to deactivate non-members
- [ ] T080 [GIT] Commit: add org membership cron sweep
- [ ] T081 [US4] Create `supabase/functions/secret-create/index.ts` — encrypt with AES-256-GCM (master key from Deno.env), store ciphertext + IV. Per contracts/edge-functions.md
- [ ] T082 [US4] Create `supabase/functions/secret-get/index.ts` — decrypt, log access (secret name only, never value), return plaintext. Per contracts/edge-functions.md
- [ ] T083 [US4] Create `supabase/functions/secret-rotate-master/index.ts` — re-encrypt all secrets in transaction. Per contracts/edge-functions.md
- [ ] T084 [GIT] Commit: add secrets edge functions
- [ ] T085 [US3] Implement `crates/fixonce-core/src/auth/oauth.rs` — GitHub OAuth browser flow (open browser, local callback server, receive JWT) (use devs:rust-dev agent)
- [ ] T086 [US3] Implement `crates/fixonce-core/src/auth/keypair.rs` — Ed25519 key generation, local storage via keyring or encrypted file (use devs:rust-dev agent)
- [ ] T087 [US3] Implement `crates/fixonce-core/src/auth/challenge.rs` — challenge-response protocol (request nonce, sign, verify) (use devs:rust-dev agent)
- [ ] T088 [US3] Implement `crates/fixonce-core/src/auth/token.rs` — JWT storage, expiry checking, refresh logic (use devs:rust-dev agent)
- [ ] T089 [GIT] Commit: add Rust auth module
- [ ] T090 [US3] Implement `crates/fixonce-cli/src/commands/login.rs` — `fixonce login` command (use devs:rust-dev agent)
- [ ] T091 [US3] Implement `crates/fixonce-cli/src/commands/auth.rs` — `fixonce auth` command (challenge-response) (use devs:rust-dev agent)
- [ ] T092 [US3] Implement `crates/fixonce-cli/src/commands/keys.rs` — `fixonce keys add|list|revoke` commands (use devs:rust-dev agent)
- [ ] T093 [GIT] Commit: add CLI auth commands
- [ ] T094 [US4] Implement `crates/fixonce-core/src/api/secrets.rs` — ephemeral secret retrieval client (request, use, discard from memory) (use devs:rust-dev agent)
- [ ] T095 [GIT] Commit: add Rust secrets client
- [ ] T096 [US3] Write `cargo test` for auth module — keypair generation, challenge-response signing/verification (use devs:rust-dev agent)
- [ ] T097 [US4] Write `deno test` for secret encryption/decryption — verify ciphertext differs from plaintext, decryption recovers original
- [ ] T098 [US3] Write `deno test` for org membership check — verify non-member rejection, cache behavior
- [ ] T098a [US3] Handle edge cases: EC-12 (GitHub API rate limit — cache last known status, retry on next request), EC-13 (deleted GitHub account treated as revocation), EC-14 (unknown public key returns 401 with registration guidance), EC-15 (validate key format and minimum strength before storing), EC-16 (enforce unique constraint on public keys across users), EC-17 (mid-request org revocation — current request completes, next fails)
- [ ] T099 [GIT] Commit: add auth and secrets tests
- [ ] T100 [US3] Review retro/P3.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T101 [GIT] Commit: finalize phase 3 retro

### Phase Completion
- [ ] T102 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T103 [GIT] Create/update PR to main with Phase 3 summary
- [ ] T104 [GIT] Verify all CI checks pass
- [ ] T105 [GIT] Report PR ready status

---

## Phase 4: Memory CRUD & Embeddings [US5]

**Goal**: Memory create/read/update/delete with VoyageAI embeddings and all output formats.
**Independent Test**: Create memory with embedding, query by ID, update, soft-delete. text/json/toon output.

### Phase Start
- [ ] T106 [GIT] Verify working tree is clean before starting Phase 4
- [ ] T107 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T108 [US5] Create retro/P4.md for this phase
- [ ] T109 [GIT] Commit: initialize phase 4 retro
- [ ] T110 [US5] Implement `crates/fixonce-core/src/memory/types.rs` — Memory, Feedback, MemoryType, SourceType structs with serde (use devs:rust-dev agent)
- [ ] T111 [US5] Implement `crates/fixonce-core/src/memory/metadata.rs` — Midnight version metadata types (compact_pragma, compact_compiler, midnight_js, etc.) (use devs:rust-dev agent)
- [ ] T112 [US5] Implement `crates/fixonce-core/src/memory/provenance.rs` — source_url, repo_url, task_summary, session_id tracking (use devs:rust-dev agent)
- [ ] T113 [GIT] Commit: add memory model types
- [ ] T114 [US5] Implement `crates/fixonce-core/src/embeddings/voyage.rs` — VoyageAI voyage-code-3 client: fetch API key from secrets, generate 1024-dim embedding, discard key. Retry with exponential backoff (3 attempts). Handle pending_embedding fallback (EC-25). (use devs:rust-dev agent)
- [ ] T115 [GIT] Commit: add VoyageAI embedding client
- [ ] T116 [US5] Implement `crates/fixonce-core/src/api/client.rs` — HTTP client with auth headers (JWT from token module) (use devs:rust-dev agent)
- [ ] T117 [US5] Implement `crates/fixonce-core/src/api/memories.rs` — create, get, update, delete memory via edge functions (use devs:rust-dev agent)
- [ ] T118 [US5] Implement `crates/fixonce-core/src/api/feedback.rs` — submit feedback via edge function (use devs:rust-dev agent)
- [ ] T119 [GIT] Commit: add API client for memory operations
- [ ] T120 [US5] Implement `crates/fixonce-core/src/output/text.rs` — human-readable text formatting for memories (use devs:rust-dev agent)
- [ ] T121 [US5] Implement `crates/fixonce-core/src/output/json.rs` — JSON output formatting (use devs:rust-dev agent)
- [ ] T122 [US5] Implement `crates/fixonce-core/src/output/toon.rs` — TOON output formatting using toon crate (use devs:rust-dev agent)
- [ ] T123 [GIT] Commit: add output formatters
- [ ] T124 [P] [US5] Create `supabase/functions/memory-update/index.ts` — validate, update, handle embedding regeneration flag. Per contracts/edge-functions.md
- [ ] T125 [P] [US5] Create `supabase/functions/memory-delete/index.ts` — soft-delete, preserve lineage. Per contracts/edge-functions.md
- [ ] T126 [P] [US5] Create `supabase/functions/feedback-submit/index.ts` — store feedback, update memory scores. Per contracts/edge-functions.md
- [ ] T127 [GIT] Commit: add remaining memory edge functions
- [ ] T128 [US5] Implement `crates/fixonce-cli/src/commands/create.rs` — `fixonce create` (generate embedding, call edge function) (use devs:rust-dev agent)
- [ ] T129 [US5] Implement `crates/fixonce-cli/src/commands/get.rs` — `fixonce get <id>` (use devs:rust-dev agent)
- [ ] T130 [US5] Implement `crates/fixonce-cli/src/commands/update.rs` — `fixonce update <id>` with re-embedding on content change (use devs:rust-dev agent)
- [ ] T131 [US5] Implement `crates/fixonce-cli/src/commands/delete.rs` — `fixonce delete <id>` (use devs:rust-dev agent)
- [ ] T132 [US5] Implement `crates/fixonce-cli/src/commands/feedback.rs` — `fixonce feedback <id> <rating>` (use devs:rust-dev agent)
- [ ] T133 [GIT] Commit: add memory CRUD CLI commands
- [ ] T134 [US5] Implement `crates/fixonce-core/src/error.rs` — structured error types with thiserror: what happened, why, what to do. Separate human and structured (JSON/TOON) error formats per Constitution §VI (use devs:rust-dev agent)
- [ ] T135 [GIT] Commit: add structured error types
- [ ] T136 [US5] Write `cargo test` for memory types serialization, output formatting, embedding client (mock VoyageAI) (use devs:rust-dev agent)
- [ ] T136a [US5] Handle edge cases: EC-18 (master key backup/recovery procedure documented), EC-19 (missing secret returns 404 with guidance), EC-20 (concurrent secret update — optimistic locking with 409 on conflict), EC-21 (single secret per request — no bulk decryption), EC-22 (CLI crash frees memory — acceptable risk), EC-23 (content chunking for embeddings exceeding token limit), EC-24 (conflicting version metadata accepted — quality gate flags it), EC-25 (VoyageAI retry with exponential backoff: 1s, 2s, 4s — pending_embedding fallback on final failure)
- [ ] T137 [GIT] Commit: add memory CRUD tests
- [ ] T138 [US5] Review retro/P4.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T139 [GIT] Commit: finalize phase 4 retro

### Phase Completion
- [ ] T140 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T141 [GIT] Create/update PR to main with Phase 4 summary
- [ ] T142 [GIT] Verify all CI checks pass
- [ ] T143 [GIT] Report PR ready status

---

## Phase 5: Write Pipeline [US6]

**Goal**: Credential detection, quality gating, LLM dedup, metadata enrichment.
**Independent Test**: Credentials blocked. Low-quality rejected. Duplicates detected with correct outcomes (new/discard/replace/update/merge). Lineage created.

### Phase Start
- [ ] T144 [GIT] Verify working tree is clean before starting Phase 5
- [ ] T145 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T146 [US6] Create retro/P5.md for this phase
- [ ] T147 [GIT] Commit: initialize phase 5 retro
- [ ] T148 [US6] Implement `crates/fixonce-core/src/pipeline/claude.rs` — Claude CLI wrapper: shell out to `claude -p --output-format json`, parse response, handle timeouts, detect missing Claude CLI (EC-37) (use devs:rust-dev agent)
- [ ] T149 [GIT] Commit: add Claude CLI wrapper
- [ ] T150 [US6] Implement `crates/fixonce-core/src/pipeline/write/credential_check.rs` — regex-based detection for API keys, private keys, passwords, PII patterns. No LLM needed. (use devs:rust-dev agent)
- [ ] T151 [US6] Implement `crates/fixonce-core/src/pipeline/write/quality_gate.rs` — Claude prompt: assess actionability, specificity, signal-to-noise. Accept/reject with rationale. (use devs:rust-dev agent)
- [ ] T152 [US6] Implement `crates/fixonce-core/src/pipeline/write/dedup.rs` — fetch top-N similar by cosine similarity, Claude prompt for 5-outcome dedup. Handle each outcome with lineage records. (use devs:rust-dev agent)
- [ ] T153 [US6] Implement `crates/fixonce-core/src/pipeline/write/enrichment.rs` — auto-detect language, suggest memory_type, flag missing version metadata. (use devs:rust-dev agent)
- [ ] T154 [GIT] Commit: add write pipeline stages
- [ ] T155 [US6] Wire write pipeline into `crates/fixonce-cli/src/commands/create.rs` — run full pipeline before storing memory (use devs:rust-dev agent)
- [ ] T156 [GIT] Commit: integrate write pipeline into create command
- [ ] T157 [US6] Write `cargo test` for credential detection (known patterns), quality gate (mock Claude), dedup outcomes (mock Claude), enrichment logic (use devs:rust-dev agent)
- [ ] T157a [US6] Handle edge cases: EC-26 (Claude timeout — retry once, then store with pipeline_incomplete flag), EC-27 (empty dedup comparison set — skip dedup, outcome always "new")
- [ ] T158 [GIT] Commit: add write pipeline tests
- [ ] T159 [US6] Review retro/P5.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T160 [GIT] Commit: finalize phase 5 retro

### Phase Completion
- [ ] T161 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T162 [GIT] Create/update PR to main with Phase 5 summary
- [ ] T163 [GIT] Verify all CI checks pass
- [ ] T164 [GIT] Report PR ready status

---

## Phase 6: Read Pipeline [US7]

**Goal**: Full RAG query pipeline with composable techniques.
**Independent Test**: Default query returns ranked results. Deep pipeline applies all stages. Version filtering works. Degraded mode returns unranked results on Claude outage.

### Phase Start
- [ ] T165 [GIT] Verify working tree is clean before starting Phase 6
- [ ] T166 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T167 [US7] Create retro/P6.md for this phase
- [ ] T168 [GIT] Commit: initialize phase 6 retro
- [ ] T169 [US7] Implement `crates/fixonce-core/src/pipeline/read/pipeline_runner.rs` — composable stage architecture. Default pipeline: query rewriting → hybrid search → relevance reranking. Deep pipeline (`--deep`): multi-query → HyDE → hybrid → retrieve-read-retrieve → confidence → reranking → coverage. (use devs:rust-dev agent)
- [ ] T170 [US7] Implement `crates/fixonce-core/src/pipeline/read/query_techniques.rs` — all 8 query techniques (query rewriting, multi-query, step-back, HyDE, decomposition, retrieve-read-retrieve, query refinement, contradiction detection) each as a composable function (use devs:rust-dev agent)
- [ ] T171 [US7] Implement `crates/fixonce-core/src/pipeline/read/result_refinement.rs` — all 7 result refinement techniques (confidence, relevance reranking, trust-aware, freshness, dedup, coverage, answerability) (use devs:rust-dev agent)
- [ ] T172 [US7] Implement `crates/fixonce-core/src/pipeline/read/search_modes.rs` — all 6 search modes: hybrid/fts/vector via edge function, metadata filtering, graph-assisted, parent-child, field-aware, passage compression (use devs:rust-dev agent)
- [ ] T173 [GIT] Commit: add read pipeline stages
- [ ] T174 [US7] Implement `crates/fixonce-core/src/api/search.rs` — search endpoint wrapper with search_type and version_filters (use devs:rust-dev agent)
- [ ] T175 [US7] Implement `crates/fixonce-cli/src/commands/query.rs` — `fixonce query <text>` with flags: `--deep`, `--version`, `--format`, `--limit` (use devs:rust-dev agent)
- [ ] T176 [GIT] Commit: add query command with pipeline integration
- [ ] T177 [US7] Handle degraded mode: Claude outage → return raw search results marked "unranked" (EC-29) (use devs:rust-dev agent)
- [ ] T178 [GIT] Commit: add degraded mode fallback
- [ ] T179 [US7] Write `cargo test` for pipeline composition, individual techniques (mock Claude), degraded mode (use devs:rust-dev agent)
- [ ] T179a [US7] Handle edge cases: EC-28 (near-threshold memories shown with "aging/may be outdated" indicator), EC-29 (Claude outage — return raw search results marked "unranked" with fallback ranking by decay_score)
- [ ] T180 [GIT] Commit: add read pipeline tests
- [ ] T181 [US7] Review retro/P6.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T182 [GIT] Commit: finalize phase 6 retro

### Phase Completion
- [ ] T183 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T184 [GIT] Create/update PR to main with Phase 6 summary
- [ ] T185 [GIT] Verify all CI checks pass
- [ ] T186 [GIT] Report PR ready status

---

## Phase 7: Memory Dynamics [US8, US9, US10, US11, US12]

**Goal**: Decay, reinforcement, anti-memories, contradictions, lineage, signatures.
**Independent Test**: Memories decay over time. Feedback affects scores. Anti-memories surface with priority. Contradictions resolved via tiebreaker. Lineage queryable. Hot cache <50ms.

### Phase Start
- [ ] T187 [GIT] Verify working tree is clean before starting Phase 7
- [ ] T188 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T189 [US8] Create retro/P7.md for this phase
- [ ] T190 [GIT] Commit: initialize phase 7 retro
- [ ] T191 [US8] Implement `crates/fixonce-core/src/memory/dynamics.rs` — exponential decay function (configurable half-life, default 30 days), event-driven acceleration, reinforcement score updates, decay threshold soft-deletion (use devs:rust-dev agent)
- [ ] T192 [GIT] Commit: add memory dynamics
- [ ] T193 [US9] Extend `crates/fixonce-core/src/memory/types.rs` — anti-memory support: description, reason, alternative, version constraints. Priority boosting in search. (use devs:rust-dev agent)
- [ ] T194 [US9] Wire anti-memory auto-proposal from negative feedback patterns into write pipeline (use devs:rust-dev agent)
- [ ] T195 [GIT] Commit: add anti-memory support
- [ ] T196 [US10] Implement `crates/fixonce-core/src/memory/contradictions.rs` — detect contradictions in read pipeline, store in contradiction_pairs, record tiebreaker votes, resolve at 3+ votes (use devs:rust-dev agent)
- [ ] T197 [GIT] Commit: add contradiction detection and resolution
- [ ] T198 [US11] Implement `crates/fixonce-core/src/memory/lineage.rs` — automatic lineage creation on replace/update/merge/feedback, lineage query for on-demand retrieval (use devs:rust-dev agent)
- [ ] T199 [US11] Implement `crates/fixonce-cli/src/commands/lineage.rs` — `fixonce lineage <id>` command (use devs:rust-dev agent)
- [ ] T200 [GIT] Commit: add lineage tracking and CLI command
- [ ] T201 [US12] Implement `crates/fixonce-core/src/memory/signatures.rs` — signature computation on memory creation, session relevance profile matching, hot cache (50-memory cap, <50ms query) (use devs:rust-dev agent)
- [ ] T202 [GIT] Commit: add memory signatures and hot cache
- [ ] T203 [US8] Write `cargo test` for decay curves, reinforcement, threshold soft-deletion (use devs:rust-dev agent)
- [ ] T204 [US9] Write `cargo test` for anti-memory priority boosting, auto-proposal (use devs:rust-dev agent)
- [ ] T205 [US10] Write `cargo test` for contradiction detection, tiebreaker resolution (use devs:rust-dev agent)
- [ ] T206 [US11] Write `cargo test` for lineage chain traversal (use devs:rust-dev agent)
- [ ] T207 [US12] Write `cargo test` for signature matching, hot cache performance (<50ms) (use devs:rust-dev agent)
- [ ] T207a [US8] Handle edge cases: EC-30 (decay cron uses row-level locking — query sees pre-decay score), EC-31 (high reinforcement does not prevent decay below threshold — continuous use prevents reaching threshold), EC-32 (contradiction involving soft-deleted memory — dismiss), EC-33 (deduplicate contradiction pairs — unique constraint on memory_a_id + memory_b_id), EC-34 (hot cache capped at 50 memories ranked by signature overlap)
- [ ] T208 [GIT] Commit: add dynamics tests
- [ ] T209 [US8] Review retro/P7.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T210 [GIT] Commit: finalize phase 7 retro

### Phase Completion
- [ ] T211 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T212 [GIT] Create/update PR to main with Phase 7 summary
- [ ] T213 [GIT] Verify all CI checks pass
- [ ] T214 [GIT] Report PR ready status

---

## Phase 8: Environment Detection & Session Analysis [US14, US15]

**Goal**: Project scanning and passive memory harvesting.
**Independent Test**: `fixonce detect` identifies Midnight components. `fixonce context` gathers metadata. `fixonce analyze` proposes relevant candidate memories.

### Phase Start
- [ ] T215 [GIT] Verify working tree is clean before starting Phase 8
- [ ] T216 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T217 [US14] Create retro/P8.md for this phase
- [ ] T218 [GIT] Commit: initialize phase 8 retro
- [ ] T219 [US14] Implement `crates/fixonce-core/src/detect/midnight.rs` — scan package.json for midnight-js, .compact files for pragma, compiler config for version, other ecosystem markers (use devs:rust-dev agent)
- [ ] T220 [US14] Implement `crates/fixonce-core/src/detect/context.rs` — gather detected versions, git remote, branch, recent commits, file structure (use devs:rust-dev agent)
- [ ] T221 [GIT] Commit: add environment detection logic
- [ ] T222 [US14] Implement `crates/fixonce-cli/src/commands/detect.rs` — `fixonce detect` with `--format` support (use devs:rust-dev agent)
- [ ] T223 [US14] Implement `crates/fixonce-cli/src/commands/context.rs` — `fixonce context` (use devs:rust-dev agent)
- [ ] T224 [GIT] Commit: add detect and context CLI commands
- [ ] T225 [US14] Wire auto-detection into query command — use detected versions for metadata filtering unless overridden by `--version` flags (use devs:rust-dev agent)
- [ ] T226 [GIT] Commit: integrate auto-detection into query pipeline
- [ ] T227 [US15] Implement `crates/fixonce-cli/src/commands/analyze.rs` — parse Claude Code session log, Claude prompt to identify learnings, present candidates with confidence, interactive accept/edit/skip/reject, feed accepted into write pipeline (use devs:rust-dev agent)
- [ ] T228 [GIT] Commit: add session transcript analysis
- [ ] T229 [US14] Write `cargo test` for environment detection — various project layouts (use devs:rust-dev agent)
- [ ] T230 [US15] Write `cargo test` for session analysis — sample session logs (use devs:rust-dev agent)
- [ ] T230a [US14] Handle edge cases: EC-38 (local-only repo — report git info as "local only", no repo_url), EC-39 (session log >100MB — warn user, process in chunks or offer to analyze recent N exchanges), EC-40 (unrecognized session format — report error with list of supported formats)
- [ ] T231 [GIT] Commit: add detection and analysis tests
- [ ] T232 [US14] Review retro/P8.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T233 [GIT] Commit: finalize phase 8 retro

### Phase Completion
- [ ] T234 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T235 [GIT] Create/update PR to main with Phase 8 summary
- [ ] T236 [GIT] Verify all CI checks pass
- [ ] T237 [GIT] Report PR ready status

---

## Phase 9: TUI [US13 — TUI portion]

**Goal**: Rich terminal UI for admin operations.
**Independent Test**: `fixonce tui` launches. All views render. Keyboard navigation works. Non-TTY rejected.

### Phase Start
- [ ] T238 [GIT] Verify working tree is clean before starting Phase 9
- [ ] T239 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T240 [US13] Create retro/P9.md for this phase
- [ ] T241 [GIT] Commit: initialize phase 9 retro
- [ ] T242 [US13] Implement `crates/fixonce-cli/src/tui/app.rs` — app state, event loop, terminal setup/teardown (use devs:rust-dev agent)
- [ ] T243 [US13] Implement `crates/fixonce-cli/src/tui/views/dashboard.rs` — main screen with search bar, memory list, activity sidebar (use devs:rust-dev agent)
- [ ] T244 [US13] Implement `crates/fixonce-cli/src/tui/views/memory_list.rs` — filterable, sortable memory list with key metadata (use devs:rust-dev agent)
- [ ] T245 [US13] Implement `crates/fixonce-cli/src/tui/views/memory_detail.rs` — full content, metadata, scores, provenance, feedback (use devs:rust-dev agent)
- [ ] T246 [GIT] Commit: add TUI core views
- [ ] T247 [P] [US13] Implement `crates/fixonce-cli/src/tui/views/create_form.rs` — memory creation form with validation (use devs:rust-dev agent)
- [ ] T248 [P] [US13] Implement `crates/fixonce-cli/src/tui/views/activity.rs` — activity stream polling edge function (use devs:rust-dev agent)
- [ ] T249 [P] [US13] Implement `crates/fixonce-cli/src/tui/views/keys.rs` — key management view (use devs:rust-dev agent)
- [ ] T250 [P] [US13] Implement `crates/fixonce-cli/src/tui/views/secrets.rs` — secret management (admin only) (use devs:rust-dev agent)
- [ ] T251 [P] [US13] Implement `crates/fixonce-cli/src/tui/views/health.rs` — system health overview (memory count, avg scores, decay stats) (use devs:rust-dev agent)
- [ ] T252 [GIT] Commit: add remaining TUI views
- [ ] T253 [US13] Create `supabase/functions/activity-stream/index.ts` — return recent activity_log entries with since/limit params. Per contracts/edge-functions.md
- [ ] T254 [GIT] Commit: add activity stream edge function
- [ ] T255 [US13] Implement `crates/fixonce-core/src/api/activity.rs` — activity log query client (use devs:rust-dev agent)
- [ ] T256 [US13] Handle terminal minimum size (EC-35) and non-TTY detection (EC-36) (use devs:rust-dev agent)
- [ ] T257 [GIT] Commit: add TUI edge cases
- [ ] T258 [US13] Implement `crates/fixonce-cli/src/commands/config.rs` — `fixonce config` command for CLI settings (use devs:rust-dev agent)
- [ ] T259 [GIT] Commit: add config command
- [ ] T260 [US13] Write `cargo test` for TUI state transitions, non-TTY detection (use devs:rust-dev agent)
- [ ] T261 [GIT] Commit: add TUI tests
- [ ] T262 [US13] Review retro/P9.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T263 [GIT] Commit: finalize phase 9 retro

### Phase Completion
- [ ] T264 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T265 [GIT] Create/update PR to main with Phase 9 summary
- [ ] T266 [GIT] Verify all CI checks pass
- [ ] T267 [GIT] Report PR ready status

---

## Phase 10: Claude Code Hooks [US16]

**Goal**: Automatic memory surfacing during coding sessions.
**Independent Test**: All 5 hooks fire. Anti-memory warnings surface. Timeout gracefully. Missing CLI doesn't block.

### Phase Start
- [ ] T268 [GIT] Verify working tree is clean before starting Phase 10
- [ ] T269 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T270 [US16] Create retro/P10.md for this phase
- [ ] T271 [GIT] Commit: initialize phase 10 retro
- [ ] T272 [US16] Implement `crates/fixonce-hooks/src/session_start.rs` — run detect, populate hot cache, surface top 3 critical memories (use devs:rust-dev agent)
- [ ] T273 [US16] Implement `crates/fixonce-hooks/src/user_prompt.rs` — lightweight query on prompt text (basic rewriting + hybrid search) (use devs:rust-dev agent)
- [ ] T274 [US16] Implement `crates/fixonce-hooks/src/pre_tool_use.rs` — check proposed content against anti-memory patterns, warn at score > 0.7 (use devs:rust-dev agent)
- [ ] T275 [US16] Implement `crates/fixonce-hooks/src/post_tool_use.rs` — check written content against anti-memory patterns, advise at score > 0.5 (use devs:rust-dev agent)
- [ ] T276 [US16] Implement `crates/fixonce-hooks/src/stop.rs` — surface critical reminders for session context (use devs:rust-dev agent)
- [ ] T277 [GIT] Commit: add hook implementations
- [ ] T278 [US16] Create `hooks/session-start.sh` — shell wrapper calling fixonce hook binary with 3s timeout (EC-41)
- [ ] T279 [US16] Create `hooks/user-prompt-submit.sh` — shell wrapper with timeout
- [ ] T280 [US16] Create `hooks/pre-tool-use.sh` — shell wrapper with timeout, always exit 0 (warn-only)
- [ ] T281 [US16] Create `hooks/post-tool-use.sh` — shell wrapper with timeout
- [ ] T282 [US16] Create `hooks/stop.sh` — shell wrapper with timeout
- [ ] T283 [GIT] Commit: add hook shell scripts
- [ ] T284 [US16] Handle missing CLI detection (EC-42) — hook scripts check for fixonce binary, exit 0 with warning if not found
- [ ] T285 [US16] Handle unauthenticated sessions (EC-43) — skip memory surfacing silently
- [ ] T286 [GIT] Commit: add hook edge case handling
- [ ] T287 [US16] Create Claude Code hooks settings template (`.claude/settings.hooks.json`) for easy installation
- [ ] T288 [GIT] Commit: add hooks settings template
- [ ] T289 [US16] Write `cargo test` for hook timeout behavior, missing CLI graceful degradation (use devs:rust-dev agent)
- [ ] T290 [GIT] Commit: add hook tests
- [ ] T291 [US16] Review retro/P10.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T292 [GIT] Commit: finalize phase 10 retro

### Phase Completion
- [ ] T293 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T294 [GIT] Create/update PR to main with Phase 10 summary
- [ ] T295 [GIT] Verify all CI checks pass
- [ ] T296 [GIT] Report PR ready status

---

## Phase 11: Integration Testing & Polish

**Goal**: End-to-end testing, performance validation, documentation.
**Independent Test**: All E2E tests pass. Performance within spec. Binaries build for all targets.

### Phase Start
- [ ] T297 [GIT] Verify working tree is clean before starting Phase 11
- [ ] T298 [GIT] Pull and rebase on origin/main if needed

### Implementation
- [ ] T299 Create retro/P11.md for this phase
- [ ] T300 [GIT] Commit: initialize phase 11 retro
- [ ] T301 Write E2E test: create memory → query → feedback → verify decay score changes (use devs:rust-dev agent)
- [ ] T302 Write E2E test: login → create memory → query with version filter → verify results (use devs:rust-dev agent)
- [ ] T303 Write E2E test: write pipeline dedup → lineage creation → lineage query (use devs:rust-dev agent)
- [ ] T304 Write E2E test: contradiction detection → tiebreaker vote → resolution (use devs:rust-dev agent)
- [ ] T305 Write E2E test: session analysis → candidate proposal → write pipeline → memory stored (use devs:rust-dev agent)
- [ ] T306 [GIT] Commit: add end-to-end tests
- [ ] T307 Run performance benchmarks: hybrid search at 1k/10k memories, hot cache timing, secret retrieval latency (use devs:rust-dev agent)
- [ ] T308 [GIT] Commit: add performance benchmarks
- [ ] T309 Verify cross-platform binary compilation: macOS ARM64, macOS x86_64, Linux x86_64
- [ ] T310 [GIT] Commit: verify cross-platform builds
- [ ] T311 Write README.md with project overview, installation, quick start, configuration, architecture
- [ ] T312 [GIT] Commit: add README
- [ ] T313 Review retro/P11.md and extract critical learnings to CLAUDE.md (conservative)
- [ ] T314 [GIT] Commit: finalize phase 11 retro

### Phase Completion
- [ ] T315 [GIT] Push branch to origin (ensure pre-push hooks pass)
- [ ] T316 [GIT] Create/update PR to main with Phase 11 summary
- [ ] T317 [GIT] Verify all CI checks pass
- [ ] T318 [GIT] Report PR ready status
