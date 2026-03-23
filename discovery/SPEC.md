# Feature Specification: fixonce-v2

**Feature Branch**: `feature/fixonce-v2`
**Created**: 2026-03-23
**Last Updated**: 2026-03-23
**Status**: Complete
**Discovery**: See `discovery/` folder for full context

---

## Problem Statement

LLM coding agents operate in isolated sessions with no persistent memory. Every new session starts from zero, unaware of corrections and lessons from previous sessions. For developers building on the Midnight Network — where language version and SDK version compatibility is a constant source of errors — this creates a costly cycle of repeated mistakes, siloed knowledge, and version-sensitive errors that agents can't track.

FixOnce v1 proved the concept as a TypeScript monorepo but hit architectural limits: client-side API key storage doesn't scale to teams, and the lack of proper auth blocks the path from internal tool to ecosystem resource.

FixOnce v2 is a clean-slate rewrite delivering a living memory system where memories compete, reinforce, decay, and replace each other — converging on what actually works.

**Impact**: Every repeated mistake costs agent time (tokens, latency) and developer time (reviewing, correcting). Version-sensitive errors compound as the Midnight ecosystem evolves.

**Target Users**: Midnight DevRel team (internal, immediate), Midnight ecosystem developers (external, future).

**Business Value**: Zero repeated agent mistakes for known problems. Knowledge that compounds with use rather than decaying into noise.

## Personas

| Persona | Description | Primary Goals |
|---------|-------------|---------------|
| DevRel Agent Operator | Aaron and the Midnight DevRel team, running LLM coding agents across Midnight projects | Zero repeated mistakes, institutional memory that compounds |
| LLM Coding Agent | Claude Code (or similar) working on a Midnight project, consuming FixOnce via CLI/hooks | Get contextually relevant memories before making mistakes, provide feedback |
| Ecosystem Developer | External developer building on Midnight (future), using FixOnce as a provided service | Access shared best practices and gotchas for their SDK/compiler versions |
| Dashboard Admin | Team member managing the memory store via web UI | Search, browse, create, edit, curate memories; monitor system health |

---

## User Scenarios & Testing

<!--
  Stories are ordered by priority (P1 first).
  Each story is independently testable and delivers standalone value.
  Stories may be revised if later discovery reveals gaps - see REVISIONS.md
-->

### User Story 1 — Developer Tooling & CI (Priority: P1)

**Revision**: v1.1 — Revised per D16: web dashboard replaced by TUI, removing TypeScript/Node tooling track. Now two tracks: Rust + Deno.

**As** the project owner, **I want** automated linting, formatting, typechecking, testing, and CI enforcement **so that** every contribution — human or agent — meets quality standards from commit one.

**Context**: Single monorepo (D2) with Cargo workspace for Rust CLI/TUI (D16) and Supabase edge functions using Deno runtime. Two tooling tracks: Rust (cargo fmt, clippy pedantic, cargo test, cargo audit) and Deno (deno lint, deno fmt, deno check) (D5). No TypeScript/Node track — web dashboard replaced by TUI (D16). Lefthook for git hooks (D3), GitHub Actions for CI (D4).

**Acceptance Scenarios**:

1. **Given** a developer clones the repo and runs the setup command, **When** they make their first commit, **Then** Lefthook pre-commit hooks automatically run: `cargo fmt --check` + `clippy` on changed Rust files, `deno fmt --check` + `deno lint` on changed edge function files

2. **Given** a developer pushes a branch and opens a PR, **When** GitHub Actions CI runs, **Then** two parallel jobs execute: (a) Rust: fmt, clippy, test, audit; (b) Deno/edge functions: deno fmt --check, deno lint, deno check. Both must pass for merge to be enabled.

3. **Given** the Rust CLI code contains `unwrap()` in a library module, **When** clippy runs with `#![deny(clippy::all, clippy::pedantic)]`, **Then** the check fails with a clear error pointing to the offending line

4. **Given** an edge function file has formatting that doesn't match `deno fmt` standards, **When** the pre-commit hook runs on that file, **Then** the commit is blocked with a message showing which files need formatting

5. **Given** a developer wants to run all checks locally before pushing, **When** they run `make check` (or equivalent), **Then** both tooling tracks run and report results

**Decisions**: D1, D2, D3, D4, D5, D16 (TUI replaces web dashboard)

---

### User Story 2 — Database Foundation & Edge Functions (Priority: P1)

**Revision**: v1.0

**As** the system, **I need** a Supabase database with pgvector, full-text search, RLS, and edge functions for all data operations **so that** every consumer (CLI, web, hooks) accesses data through a single secure API surface.

**Context**: 7 tables (D6): memory, feedback, activity_log, secrets, cli_keys, memory_lineage, contradiction_pairs. All access via edge functions (CONSTITUTION §I). Auth via Supabase auth helpers (D7). Migrations via Supabase CLI (D10). Embeddings: vector(1024) from voyage-code-3 (D8). Search: configurable search_type parameter with Postgres RPC hybrid fusion (D9).

**Acceptance Scenarios**:

1. **Given** a fresh Supabase project, **When** all migrations are run via `supabase db push`, **Then** all 7 tables exist with correct columns, constraints, indexes, and RLS policies enabled

2. **Given** the `memory` table exists, **When** a new memory is inserted via edge function, **Then** it stores: title, content, summary, memory_type, source_type, language, embedding (vector(1024)), weighted tsvector (for FTS), Midnight-specific version metadata (compact_pragma, compact_compiler, midnight_js, etc.), provenance fields (source_url, repo_url, task_summary), decay_score (default 1.0), reinforcement_score (default 0), created_at, updated_at

3. **Given** RLS is enabled on all tables, **When** an unauthenticated request attempts to query the database directly (not through edge functions), **Then** the request is denied — zero rows returned

4. **Given** a valid authenticated request to the search edge function with `search_type=hybrid`, **When** the Postgres RPC function executes, **Then** it combines full-text search (weighted tsvector with ts_rank) and vector similarity (cosine distance via pgvector) using Reciprocal Rank Fusion and returns ranked results

5. **Given** a valid authenticated request with `search_type=fts`, **When** the edge function queries, **Then** it returns full-text search results only (tsvector + ts_rank)

6. **Given** a valid authenticated request with `search_type=vector`, **When** the edge function queries, **Then** it returns vector similarity results only (cosine distance via pgvector HNSW index)

7. **Given** any edge function receives a request, **When** it processes the request, **Then** it: (a) verifies auth via `supabase.auth.getUser()`, (b) validates input schema, (c) executes the database operation, (d) logs the operation to `activity_log`, (e) returns a structured response

8. **Given** a memory is inserted, **When** the weighted tsvector is computed, **Then** title has weight A, summary has weight B, and content has weight C

**Decisions**: D6 (7 tables), D7 (Supabase auth for all), D8 (vector(1024)), D9 (configurable search_type), D10 (Supabase CLI migrations)

---

### User Story 3 — Authentication & Authorization (Priority: P1)

**Revision**: v1.1 — Revised per D16: GitHub OAuth now happens in the CLI (browser-based OAuth flow), not a web dashboard. CLI key registration done via CLI commands, not a web UI.

**As** the project owner, **I want** GitHub OAuth with org/team-based access restrictions and challenge-response public-key authentication for multi-machine deployment **so that** only authorized team members can access the system, and access is automatically revoked when someone leaves the org.

**Context**: Supabase handles GitHub OAuth natively (D7). CLI initiates GitHub OAuth via browser redirect flow (D16 — no web dashboard). Post-login edge function checks org membership via GitHub API (D11). For multi-machine deployment, CLI uses challenge-response with registered public keys: request nonce → sign with private key → edge function verifies → issues 8-hour JWT (D12, D14). Org membership re-verified every hour (cache TTL) + twice-daily cron sweep (D13).

**Two auth paths**:
- **GitHub OAuth (primary)**: CLI opens browser for GitHub OAuth → Supabase handles OAuth → edge function verifies org membership → CLI receives JWT. Used for initial authentication on each machine.
- **Public-key challenge-response (secondary)**: For deploying CLI on additional machines without repeating browser OAuth. User registers CLI public keys via CLI command (after GitHub OAuth). Each key can independently authenticate via challenge-response for an 8-hour JWT.

**Acceptance Scenarios**:

1. **Given** a user runs `fixonce login`, **When** the CLI opens their browser for GitHub OAuth, **Then** Supabase handles the OAuth flow, the post-login edge function verifies org membership, and the CLI receives a JWT on success

2. **Given** a user with a GitHub account NOT in the authorized org, **When** they complete the OAuth flow, **Then** the edge function denies access with a clear error: "Access restricted to [org name] members"

3. **Given** an authenticated user, **When** they run `fixonce keys add`, **Then** the CLI generates a keypair (if none exists), registers the public key via an edge function, and stores the private key locally

4. **Given** a CLI instance with a registered keypair on a second machine, **When** it initiates authentication via `fixonce auth`, **Then** it: (a) requests a nonce from the auth edge function, (b) signs the nonce with its private key, (c) sends the signature + public key to the verification edge function, (d) receives an 8-hour JWT on success

5. **Given** a valid CLI JWT (from either auth path), **When** the CLI makes subsequent API requests, **Then** the JWT is accepted by all edge functions via `supabase.auth.getUser()`

6. **Given** an authenticated user whose org membership has been cached for over 1 hour, **When** they make any authenticated request, **Then** the system re-checks their org membership via GitHub API, caches the result for 1 hour, and blocks the request if membership has been revoked

7. **Given** a user who has left the authorized org, **When** the twice-daily cron job runs, **Then** their active sessions are invalidated and their account is marked as deactivated

8. **Given** a CLI JWT that has expired (>8 hours old), **When** the CLI makes a request, **Then** the request is rejected with a 401 and the CLI must re-authenticate

9. **Given** a user with multiple CLI instances (e.g., laptop + CI server), **When** they register multiple public keys via `fixonce keys add` on each machine, **Then** each key independently authenticates and receives its own JWT — revoking one key doesn't affect others

10. **Given** an authenticated user, **When** they run `fixonce keys list`, **Then** they see all their registered public keys with labels, creation dates, and last-used timestamps

11. **Given** an authenticated user, **When** they run `fixonce keys revoke <key-id>`, **Then** the key is deleted from `cli_keys` and any JWT issued for that key is invalidated

**Decisions**: D7, D11, D12, D13, D14, D16 (TUI replaces web dashboard — OAuth moves to CLI browser flow)

---

### User Story 4 — Encrypted Secrets Management (Priority: P1)

**Revision**: v1.0

**As** a CLI user, **I want** secrets (API keys like VoyageAI) stored encrypted on the server and retrieved ephemerally **so that** I never have to store sensitive credentials on disk and a database breach doesn't expose plaintext secrets.

**Context**: Edge function encryption with AES-256-GCM (D15). Encryption master key stored as Supabase environment secret — never in the database. Database `secrets` table stores only ciphertext + metadata. CLI requests a secret via authenticated edge function, receives plaintext, uses it for the operation (e.g., embedding generation), then discards it from memory. Generic secrets store (not just VoyageAI — designed for any future secret).

**Acceptance Scenarios**:

1. **Given** an admin creates a new secret via the dashboard (e.g., "VOYAGEAI_API_KEY"), **When** the create-secret edge function receives it, **Then** it encrypts the value using AES-256-GCM with the master key from the environment, stores the ciphertext + IV + name + metadata in the `secrets` table, and never logs or returns the plaintext in the response

2. **Given** an authenticated CLI request to the get-secret edge function with secret name "VOYAGEAI_API_KEY", **When** the edge function processes the request, **Then** it retrieves the ciphertext from the `secrets` table, decrypts it using the master key, returns the plaintext in the response body, and logs the access to `activity_log` (logging which secret was accessed, by whom, not the value)

3. **Given** the CLI receives a decrypted secret, **When** the operation completes (e.g., embedding generated), **Then** the CLI discards the secret from memory — it is never written to disk, config files, environment variables, or logs

4. **Given** someone gains raw read access to the database (e.g., RLS misconfigured), **When** they query the `secrets` table, **Then** they see only encrypted ciphertext — the decryption master key is not in the database and not derivable from any database content

5. **Given** an admin wants to rotate a secret, **When** they update the value via the dashboard, **Then** the old ciphertext is replaced with a newly encrypted value using the same master key — no downtime, no CLI update needed

6. **Given** the admin rotates the encryption master key, **When** the rotation edge function runs, **Then** all secrets are decrypted with the old key and re-encrypted with the new key in a single transaction

7. **Given** a secret access request from an unauthenticated or unauthorized user, **When** the get-secret edge function processes it, **Then** it returns 401/403 — secrets are never returned to unauthenticated requests

**Decisions**: D15 (Edge function AES-256-GCM encryption, master key as env secret)

---

### User Story 5 — Memory CRUD with Rich Metadata (Priority: P1)

**Revision**: v1.0

**As** an agent operator or LLM agent, **I want** to create, read, update, and delete memories with rich Midnight-specific metadata and provenance tracking **so that** each memory carries the full context needed for version-aware, source-traceable knowledge retrieval.

**Context**: Memories are the core data entity. Each memory has: content, title, summary, memory_type (gotcha, best_practice, correction, anti_pattern, discovery), source_type (correction, observation, pr_feedback, manual, harvested), language (compact, typescript, rust, etc.), embedding (vector(1024) from voyage-code-3), Midnight version metadata, provenance fields, and decay/reinforcement scores.

**Acceptance Scenarios**:

1. **Given** an authenticated CLI user, **When** they create a memory with title, content, summary, memory_type, language, and version metadata, **Then** the CLI requests the VoyageAI API key from the secrets endpoint, generates an embedding for the content, and sends the complete memory (with embedding) to the create-memory edge function, which stores it with default decay_score=1.0 and reinforcement_score=0

2. **Given** a memory about a Compact compiler gotcha, **When** it is created, **Then** it stores version metadata: compact_pragma version, compact_compiler version, and optionally midnight_js version, indexer version, node version — whichever are relevant to the memory

3. **Given** a memory originated from PR feedback, **When** it is created, **Then** it stores provenance: source_url (PR URL), repo_url (GitHub repo), task_summary (what the agent was working on), session_id (if from a session)

4. **Given** an authenticated request to get a memory by ID, **When** the edge function retrieves it, **Then** it returns the full memory including all metadata, provenance, scores, and timestamps — but NOT the raw embedding vector (too large for most consumers)

5. **Given** an authenticated request to update a memory, **When** the content or title changes, **Then** the embedding is regenerated (CLI fetches VoyageAI key, generates new embedding, sends to update edge function). If only metadata changes, no re-embedding needed.

6. **Given** an authenticated request to delete a memory, **When** the edge function processes it, **Then** the memory is soft-deleted (marked as deleted, not removed from DB) and excluded from all search results. Associated lineage and feedback records are preserved.

7. **Given** a memory is created without version metadata, **When** the edge function validates the input, **Then** it accepts the memory — version metadata is optional but encouraged. The CLI SHOULD prompt for version info when possible.

**Edge Cases**: See EC-23 through EC-27 below.

---

### User Story 6 — Write Pipeline (Priority: P1)

**Revision**: v1.0

**As** the system, **I need** a write pipeline that quality-gates, deduplicates, and enriches every memory before storage **so that** the memory store remains clean, high-signal, and free of duplicate or low-quality entries.

**Context**: All write pipeline inference uses `claude -p --output-format json`. Pipeline runs in the CLI, not in edge functions. Steps: (1) credential/PII detection, (2) quality assessment, (3) LLM-powered deduplication against existing memories, (4) metadata enrichment. Dedup has 5+ outcomes: new (store as-is), discard (duplicate/low-quality), replace (supersedes existing), update (merge new info into existing), merge (combine two memories into one).

**Quality Gate Criteria**: The quality gate is picky — it maintains a meaningful bar without being a bottleneck. Claude evaluates each candidate memory on five dimensions and returns a structured verdict (pass/fail with per-dimension scores and rationale):

| Dimension | Pass | Fail (reject) |
|-----------|------|----------------|
| **Specificity** | Describes a concrete situation with enough detail to recognize when it applies. Names specific components, versions, methods, or error messages. | Vague advice that could apply to anything ("be careful with versions", "always test your code"). |
| **Actionability** | Tells the reader what to do (or not do) in a recognizable scenario. Contains a clear recommendation, workaround, or warning. | Describes what happened without advice. Pure observation with no takeaway ("I noticed X"). |
| **Scope** | Covers one focused topic. A reader can understand the full memory in under 60 seconds. | Rambling brain dump covering multiple unrelated topics. Should be split into separate memories. |
| **Signal** | Contains a non-obvious insight — something a developer wouldn't find in the first page of official docs. A genuine discovery, gotcha, or hard-won lesson. | Restates obvious documentation, basic language features, or universally known best practices. |
| **Completeness** | Includes enough context to be useful without guessing: what scenario, what went wrong (or right), what to do about it. Version info is present when relevant. | Missing critical context — reader would need to guess when/where/why this applies. |

A memory MUST pass all five dimensions. Failure on any dimension produces a rejection with the specific dimension, score, and a rewrite suggestion explaining how to fix it. The submitter can revise and resubmit.

**Acceptance Scenarios**:

1. **Given** a candidate memory, **When** the write pipeline runs, **Then** it first scans content for credentials, API keys, private keys, and PII — rejecting any memory that contains them with a clear explanation of what was detected

2. **Given** a candidate memory that passes credential detection, **When** the quality gate runs via `claude -p`, **Then** Claude evaluates the five quality dimensions (specificity, actionability, scope, signal, completeness), returns a structured verdict with per-dimension pass/fail and rationale, and rejects memories that fail any dimension with a rewrite suggestion

3. **Given** a candidate memory rejected by the quality gate, **When** the rejection is returned to the CLI, **Then** the output includes: which dimension(s) failed, why, and a concrete suggestion for how to revise the memory to pass (e.g., "Add the Compact compiler version where this was observed" or "Split this into separate memories for map operations and iterator patterns")

4. **Given** a candidate memory that passes quality gating, **When** the dedup step runs via `claude -p`, **Then** Claude compares it against the top-N most similar existing memories (by embedding cosine similarity) and returns one of: `new` (no significant overlap), `discard` (duplicate of existing), `replace` (supersedes an existing memory), `update` (new info should be merged into existing), `merge` (combine candidate with existing into a new unified memory)

5. **Given** a dedup outcome of `replace`, **When** the pipeline processes it, **Then** the existing memory is soft-deleted, the new memory is stored, and a lineage record is created linking old → new

6. **Given** a dedup outcome of `update`, **When** the pipeline processes it, **Then** the existing memory's content is updated (via Claude to synthesize old + new), the embedding is regenerated, and a lineage record captures the change

7. **Given** a dedup outcome of `merge`, **When** the pipeline processes it, **Then** both the candidate and existing memory are soft-deleted, a new unified memory is created, and lineage records link both originals to the merged result

8. **Given** a memory that passes all pipeline stages, **When** it is stored, **Then** the pipeline also enriches metadata: auto-detecting language if not specified, suggesting memory_type if ambiguous, and flagging if version metadata is missing

9. **Given** negative feedback patterns on existing memories (e.g., multiple "outdated" ratings), **When** the write pipeline detects this, **Then** it may propose an anti-memory based on the pattern — a first-class "don't do this" artifact (see Story 9)

---

### User Story 7 — Read Pipeline (Priority: P1)

**Revision**: v1.0

**As** an LLM agent or CLI user, **I want** a full suite of RAG query techniques, search modes, and result refinement **so that** I get the most relevant memories for my context, regardless of how I phrase my query.

**Context**: All read pipeline inference uses `claude -p --output-format json`. Pipeline runs in the CLI. The search endpoint supports `search_type` parameter (hybrid|fts|vector, D9). The pipeline is composable — techniques can be combined per query.

**Query Techniques** (each implemented as a CLI pipeline stage):

- **Query Rewriting**: Claude rewrites the user's natural language query for better retrieval
- **Multi-Query Generation**: Claude generates 3-5 variant queries to broaden recall
- **Step-Back Queries**: Claude generates a more abstract version of the query to find broader principles
- **HyDE (Pseudo-Answer Generation)**: Claude generates a hypothetical answer, then searches for memories similar to that answer
- **Decomposition**: Claude breaks complex queries into sub-queries, searches each independently, merges results
- **Retrieve-Read-Retrieve**: Initial retrieval → Claude reads results → refined second retrieval based on gaps
- **Query Refinement**: Claude iteratively refines the query based on initial results
- **Contradiction Detection**: Claude checks if returned memories contradict each other

**Score Definitions**:

| Score | Range | Source | Description |
|-------|-------|--------|-------------|
| `relevance_score` | 0.0–1.0 | Search layer (algorithmic) | How well the memory's content matches the query text/embedding. Produced by hybrid search RRF, FTS ts_rank, or vector cosine similarity. Present on all results. "Does this match your query?" |
| `confidence_score` | 0.0–1.0 | Confidence Assessment stage (Claude) | How likely this memory correctly and reliably answers the query. Claude evaluates: does the memory address what was asked? Is the advice current? Is it specific enough to act on? Only present when Confidence Assessment runs (deep pipeline). `null` in default pipeline. "Is this answer trustworthy?" |

**Ranking**: Results are ranked by `relevance_score` (default pipeline). When Confidence Assessment runs (`--deep`), results are re-ranked by `confidence_score` after the initial relevance ranking. Trust-Aware Reranking further adjusts using `reinforcement_score`.

**Result Refinement** (each implemented as a CLI pipeline stage):

- **Confidence Assessment**: Claude scores each result 0.0–1.0 on trustworthiness for this query. Only runs in deep pipeline (`--deep`). Evaluates: does the memory address the question? Is the advice current for the queried versions? Is it specific enough to act on without guessing?
- **Relevance Reranking**: Claude reranks results by relevance to the original query
- **Trust-Aware Reranking**: Rerank by memory reinforcement score and feedback history
- **Freshness Reranking**: Boost recently updated or accessed memories
- **Deduplication**: Remove near-duplicate results from the result set
- **Coverage Balancing**: Ensure results cover different aspects of the query, not just the most similar
- **Answerability Scoring**: Claude scores whether the result set actually answers the query

**Search Modes** (composable with query techniques):

- **Hybrid Search**: FTS + vector via Postgres RPC (D9), default
- **Metadata Filtering**: Filter by version predicates, language, memory_type, source_type
- **Graph-Assisted Retrieval**: Follow lineage and contradiction links to find related memories
- **Parent-Child Retrieval**: When a memory is part of a merge/replace chain, retrieve the full chain
- **Field-Aware Retrieval**: Weight matches differently based on which field matched (title > summary > content)
- **Passage Compression**: Claude compresses long memories into concise summaries for the result set

**Acceptance Scenarios**:

1. **Given** a simple query `fixonce query "compact map iteration"`, **When** the default pipeline runs, **Then** it applies: query rewriting → hybrid search → relevance reranking → return results formatted per `--format` flag

2. **Given** a query with `--deep` flag, **When** the extended pipeline runs, **Then** it applies: multi-query generation → HyDE → hybrid search → retrieve-read-retrieve → confidence assessment → relevance reranking → coverage balancing → return results

3. **Given** a query with `--version "compact_compiler >= 0.15"`, **When** the pipeline runs, **Then** metadata filtering is applied to restrict results to memories tagged with compact_compiler version 0.15 or higher

4. **Given** a query that returns contradictory results, **When** contradiction detection runs, **Then** the output includes a warning: "These memories may contradict each other: [memory A] vs [memory B]" and flags them for potential contradiction court resolution

5. **Given** a query with `--format json`, **When** results are returned, **Then** each result includes: memory_id, title, summary, relevance_score, confidence_score (null unless `--deep`), decay_score, reinforcement_score, version_metadata, and a truncated content preview

6. **Given** a query with `--format toon`, **When** results are returned, **Then** results are formatted in TOON for optimal LLM token efficiency

7. **Given** an empty result set after all pipeline stages, **When** the CLI presents results, **Then** it suggests: alternative queries, broader search terms, or whether to remove version filters

---

### User Story 8 — Memory Dynamics (Priority: P1)

**Revision**: v2.0 — Added decay/reinforcement formula (H1 fix).

**As** the system, **I need** memories to decay, reinforce, and self-correct over time **so that** the memory store converges on what actually works rather than accumulating stale or low-quality information.

**Context**: Hybrid decay: temporal (unused memories fade) + event-driven (new SDK release invalidates version-pinned memories). Reinforcement: frequent access and positive feedback increase memory strength. Agent feedback: helpful/outdated/damaging ratings. Memories compete — high-scoring memories surface more, low-scoring memories fade.

**Decay/Reinforcement Formula**:

Reinforcement modulates the decay rate. Higher reinforcement = slower decay. A memory that is continuously accessed effectively cannot reach the soft-deletion threshold, but if access stops, it decays at the normal rate.

```
decay_score = e^(-effective_λ * t)

where:
  base_λ        = ln(2) / half_life_days          (default half_life = 30 days)
  effective_λ   = base_λ / (1 + α * reinforcement_score)   (α = 0.01, configurable)
  t             = days since last access (last_accessed_at)
```

**Reinforcement score updates**:
- "helpful" feedback: `reinforcement_score += 1`
- Memory returned in search results (accessed): `reinforcement_score += 0.5`
- "outdated" feedback: `reinforcement_score -= 2` (can go negative)
- "damaging" feedback: `reinforcement_score -= 10` (sharp penalty)

**Event-driven decay acceleration**:
- When triggered (e.g., new SDK version), multiply `effective_λ` by an acceleration factor (default 3x)
- Acceleration lasts until the next decay recalculation
- Formula: `accelerated_λ = effective_λ * event_boost` (default `event_boost = 3.0`, configurable)

**Soft-deletion threshold**: `decay_score < 0.1` (configurable). Memories below this are soft-deleted.

**"Near threshold" indicator**: Memories with `decay_score` between 0.05 and 0.15 are marked as "aging — may be outdated" in search results (EC-28).

**Worked Example**:
- A memory with `reinforcement_score=0` and `half_life=30`: at 30 days without access, `decay_score ≈ 0.5`. At 100 days, `decay_score ≈ 0.1` (soft-deleted).
- Same memory with `reinforcement_score=50`: `effective_λ` is halved (1 + 0.01 * 50 = 1.5). At 30 days, `decay_score ≈ 0.63`. At 100 days, `decay_score ≈ 0.25` — still alive.
- If that memory stops being accessed and reinforcement_score stays at 50, it eventually reaches 0.1 at ~150 days instead of 100.
- If a version release fires event-driven acceleration (3x), the 30-day half-life effectively becomes 10 days for that memory.

**Acceptance Scenarios**:

1. **Given** a memory with decay_score=1.0, reinforcement_score=0, and half_life=30 days, **When** 30 days pass without access, **Then** the decay function computes decay_score ≈ 0.5 (standard exponential decay)

2. **Given** a memory with reinforcement_score=50 and half_life=30 days, **When** 30 days pass without access, **Then** the effective half-life is longer (effective_λ is divided by 1.5), resulting in decay_score ≈ 0.63 — significantly slower than an unreinforced memory

3. **Given** a memory tagged with `compact_compiler=0.14`, **When** a new compiler version 0.15 is released and an event-driven decay trigger fires with event_boost=3.0, **Then** the memory's effective_λ is tripled, causing it to decay 3x faster than normal until the next recalculation

4. **Given** a memory that is queried and returned in search results, **When** the agent provides positive feedback ("helpful"), **Then** reinforcement_score increases by 1, last_accessed_at resets to now, and the decay clock restarts from decay_score=1.0

5. **Given** a memory that receives "outdated" feedback, **When** the feedback is processed, **Then** reinforcement_score decreases by 2 and the feedback is recorded for pattern analysis. If reinforcement_score goes negative, effective_λ increases (decay accelerates beyond base rate).

6. **Given** a memory that receives "damaging" feedback, **When** the feedback is processed, **Then** reinforcement_score decreases by 10, the memory is flagged for review, and a candidate anti-memory may be proposed by the write pipeline

7. **Given** a memory whose decay_score drops below 0.1, **When** the cleanup process runs, **Then** the memory is soft-deleted — it stops appearing in search results but lineage records are preserved

8. **Given** two memories about the same topic with different reinforcement scores, **When** both appear in search results, **Then** the higher-reinforcement memory ranks above the lower one (trust-aware reranking from Story 7 uses this score)

9. **Given** a configurable decay half-life, **When** an admin changes the half-life via the TUI or CLI, **Then** all future decay calculations use the new half-life — existing decay_scores are not retroactively recalculated

10. **Given** a memory with decay_score between 0.05 and 0.15, **When** it appears in search results, **Then** it is displayed with an "aging — may be outdated" indicator (EC-28)

---

### User Story 9 — Anti-Memories (Priority: P1)

**Revision**: v1.0

**As** an LLM agent, **I want** the system to proactively surface "do NOT do this" warnings when I'm about to make a known mistake **so that** I avoid repeating documented anti-patterns.

**Context**: Anti-memories are first-class memory artifacts with memory_type="anti_pattern". They have their own embeddings, version constraints, and decay behavior. They are surfaced proactively via Claude Code hooks (Story 16) when the system detects an agent is headed toward a known mistake. Intervention is warn-only (CONSTITUTION).

**Acceptance Scenarios**:

1. **Given** a memory is created with memory_type="anti_pattern", **When** it is stored, **Then** it includes: the anti-pattern description (what NOT to do), the reason (why it's harmful), the alternative (what to do instead), and version constraints (which versions this applies to)

2. **Given** an anti-memory about "never use Map.from() with Compact 0.15+", **When** an agent queries about Compact map operations with version >= 0.15, **Then** the anti-memory is surfaced with higher priority than regular memories about maps

3. **Given** multiple "outdated" feedback ratings on a regular memory, **When** the write pipeline detects this pattern, **Then** it proposes creating an anti-memory that captures the negative lesson — requiring admin approval before storage

4. **Given** an anti-memory exists, **When** it receives positive feedback ("this warning saved me"), **Then** its reinforcement score increases — anti-memories strengthen through use just like regular memories

5. **Given** an anti-memory whose version constraint no longer applies (e.g., the bug was fixed in a newer SDK version), **When** event-driven decay fires for that version, **Then** the anti-memory decays faster — it may become irrelevant

6. **Given** search results that include both a regular memory and a contradicting anti-memory, **When** results are presented, **Then** the anti-memory is clearly marked as a warning and presented alongside (not replacing) the regular memory

---

### User Story 10 — Contradiction Detection & Resolution (Priority: P1)

**Revision**: v1.0

**As** the system, **I need** to detect when two memories contradict each other and facilitate agent-mediated resolution **so that** conflicting knowledge is resolved through use rather than silently confusing agents.

**Context**: Contradictions are detected during read pipeline (Story 7) and during write pipeline dedup (Story 6). The `contradiction_pairs` table stores flagged pairs with resolution status. Agents act as tiebreakers — when they encounter a contradiction, they try approaches and report which worked.

**Contradiction Criteria**: Two memories are considered contradictory when they recommend mutually exclusive approaches for the same component, version range, or task. Claude evaluates contradiction via a structured prompt comparing the directionality of advice (e.g., "use X" vs "avoid X" for the same scenario). Contradictions between memories targeting different version ranges are NOT contradictions — they are version-specific advice.

**Tiebreaker Vote Structure**: Each vote records: `{ user_id, timestamp, voted_for: memory_a | memory_b, context: string }`. Resolution requires 3+ votes from distinct users/agents. Votes are unweighted (equal). Repeated votes from the same user on the same pair are deduplicated (last vote wins). Votes do not expire.

**Acceptance Scenarios**:

1. **Given** the read pipeline returns two memories that contradict each other (detected by Claude via contradiction detection), **When** results are presented, **Then** the contradiction is flagged: "These memories may conflict: [A] vs [B]. Your feedback will help resolve this."

2. **Given** a flagged contradiction pair, **When** stored in `contradiction_pairs`, **Then** it records: memory_a_id, memory_b_id, detected_at, resolution_status (open|resolved|dismissed), agent_feedback (array of tiebreaker results), resolved_at

3. **Given** an agent encounters a contradiction, **When** it tries one approach and reports feedback ("memory A was correct"), **Then** the feedback is recorded as a tiebreaker vote on the contradiction pair

4. **Given** a contradiction pair with 3+ tiebreaker votes favoring one memory, **When** the resolution threshold is met, **Then** the losing memory's decay_score is accelerated, the winning memory's reinforcement_score increases, and the contradiction is marked as resolved

5. **Given** a resolved contradiction where the losing memory was an anti-memory, **When** the resolution is applied, **Then** the anti-memory is soft-deleted (it was wrong) and the resolution is logged in lineage

6. **Given** a new memory is submitted that contradicts an existing memory, **When** the write pipeline detects the contradiction during dedup, **Then** instead of a simple dedup outcome, it creates a contradiction pair and stores both memories — letting agent feedback resolve the conflict over time

---

### User Story 11 — Memory Lineage (Priority: P1)

**Revision**: v1.0

**As** an admin or auditor, **I want** every memory to carry its full provenance chain — what it replaced, what merged into it, what feedback changed it — **so that** I can audit why a memory exists in its current form.

**Context**: The `memory_lineage` table stores parent-child-merge relationships. Lineage is always stored but returned only on explicit request (not in standard query responses). Created automatically by write pipeline (Story 6) on replace/update/merge operations.

**Acceptance Scenarios**:

1. **Given** a memory that replaced an older memory (write pipeline outcome: `replace`), **When** lineage is queried for the new memory, **Then** it shows: "Replaced memory [old_id] on [date]. Reason: [dedup rationale from Claude]"

2. **Given** a memory that resulted from merging two others (outcome: `merge`), **When** lineage is queried, **Then** it shows both parent memories and the merge rationale

3. **Given** a memory that was updated with new information (outcome: `update`), **When** lineage is queried, **Then** it shows the update history: what changed, when, and the update rationale

4. **Given** a memory with feedback history (helpful/outdated/damaging ratings), **When** lineage is queried, **Then** feedback events are included in the provenance chain

5. **Given** a standard query via `fixonce query`, **When** results are returned, **Then** lineage is NOT included — only returned when explicitly requested via `fixonce lineage <memory-id>` or `--include-lineage` flag

6. **Given** a soft-deleted memory, **When** its lineage is queried, **Then** the lineage is still accessible — soft deletion does not destroy provenance

---

### User Story 12 — Memory Signatures (Priority: P1)

**Revision**: v1.0

**As** the system, **I want** to pre-compute relevance fingerprints for each memory and cache a hot set of likely-relevant memories per session **so that** agents have relevant knowledge ready before they even make their first query.

**Context**: Memory signatures are pre-computed based on correlated files, error patterns, and SDK method calls. When an agent starts a session and runs environment detection (Story 14), FixOnce computes a session relevance profile and caches a hot set of memories whose signatures overlap.

**Acceptance Scenarios**:

1. **Given** a memory about Compact map operations, **When** its signature is computed, **Then** it includes: related file patterns (*.compact), error patterns (map-related compiler errors), SDK method calls (Map.set, Map.get), and version ranges

2. **Given** an agent starts a session in a project using Compact 0.15 + midnight-js 0.8, **When** environment detection runs and reports these versions, **Then** the CLI computes a session relevance profile and pre-fetches memories whose signatures overlap with this project's tech profile

3. **Given** a pre-cached hot set of 20 memories, **When** the agent makes its first query, **Then** the query pipeline first checks the hot cache before making a full search — returning cached results in under 50ms if they match

4. **Given** a memory is created mid-session that would be relevant to the current session profile, **When** the signature is computed, **Then** the hot cache is NOT automatically refreshed — the agent must explicitly request a cache refresh or it refreshes on the next query that misses the cache

5. **Given** a session with no detected environment (bare repo, no version files), **When** the hot cache is computed, **Then** it falls back to the most universally relevant memories (highest reinforcement scores, no version filtering)

---

### User Story 13 — Rust CLI & TUI (Priority: P1)

**Revision**: v1.0

**As** an agent operator, LLM agent, or admin, **I want** a single Rust binary that provides both a scriptable CLI and a rich TUI for administration **so that** all FixOnce operations are available from a single, fast, zero-dependency tool.

**Context**: Built with Rust. TUI uses ratatui (D16). Single binary distribution — no runtime dependencies. CLI commands for agents and scripting; TUI mode for admin browsing/search/edit/feedback/activity. Auth: GitHub OAuth (browser flow) for initial login, public-key challenge-response for multi-machine (Story 3). Output formats: text (human), JSON (machine), TOON (LLM-optimized). All inference shells out to `claude -p --output-format json`.

**CLI Commands**:
- `fixonce login` — GitHub OAuth via browser
- `fixonce auth` — Challenge-response with registered public key
- `fixonce keys add|list|revoke` — Manage CLI public keys
- `fixonce create` — Create a memory (runs write pipeline)
- `fixonce query <text>` — Query memories (runs read pipeline)
- `fixonce get <id>` — Get a specific memory by ID
- `fixonce update <id>` — Update a memory
- `fixonce delete <id>` — Soft-delete a memory
- `fixonce feedback <id> <rating>` — Submit feedback on a memory
- `fixonce lineage <id>` — View a memory's provenance chain
- `fixonce detect` — Detect project environment and versions
- `fixonce context` — Gather project context for memory creation
- `fixonce analyze <session-log>` — Analyze session transcript for candidate memories
- `fixonce config` — Configure CLI settings
- `fixonce tui` — Launch the admin TUI

**TUI Features** (ratatui):
- Memory search and browsing with real-time filtering
- Memory detail view with metadata, provenance, scores
- Memory creation and editing forms
- Feedback submission
- Activity stream (polling edge function for recent activity_log entries)
- Key management
- Secret management (admin only)
- System health overview (memory count, avg scores, decay stats)

**Acceptance Scenarios**:

1. **Given** a user installs FixOnce, **When** they run `fixonce --version`, **Then** the single binary responds with the version — no runtime dependencies required

2. **Given** a user runs `fixonce query "compact map gotcha" --format json`, **When** the pipeline completes, **Then** structured JSON is written to stdout with results array, each containing memory_id, title, summary, scores, and version metadata

3. **Given** a user runs `fixonce query "compact map gotcha" --format toon`, **When** the pipeline completes, **Then** TOON-formatted output is written to stdout, optimized for LLM token efficiency

4. **Given** a user runs `fixonce tui`, **When** the TUI launches, **Then** they see a main screen with: search bar, memory list, activity feed sidebar, and keyboard navigation hints

5. **Given** the TUI is running, **When** the user navigates to a memory and presses Enter, **Then** they see the full memory detail including content, metadata, version info, provenance, decay/reinforcement scores, and feedback history

6. **Given** the TUI is running, **When** the user presses 'c' to create a new memory, **Then** a form appears with fields for title, content, summary, memory_type, language, version metadata — and on submit, the write pipeline runs in the background

7. **Given** the CLI is run in a non-TTY environment (piped, cron, agent), **When** any command runs, **Then** no interactive prompts or TUI elements appear — pure text/JSON/TOON output to stdout, errors to stderr

8. **Given** any CLI command fails, **When** the error is displayed, **Then** it includes: what happened, why, and what to do about it (per CONSTITUTION §VI). JSON/TOON format includes structured error codes.

9. **Given** a CLI command encounters an error (auth failure, network timeout, invalid input, etc.), **When** the error is displayed in `--format json`, **Then** the output is: `{ "error": { "code": "ERR_AUTH_EXPIRED", "message": "JWT expired", "cause": "Your session token is older than 8 hours", "action": "Run fixonce login or fixonce auth to re-authenticate" } }`. In `--format text`, the same information renders as a human-readable message with the action highlighted. In `--format toon`, the structured error is TOON-encoded.

---

### User Story 14 — Environment Detection & Context Gathering (Priority: P1)

**Revision**: v1.0

**As** an LLM agent starting a session, **I want** to automatically detect what Midnight components and versions are present in the current project **so that** queries and memory creation include accurate version context.

**Context**: The CLI scans the project directory for version indicators: package.json (midnight-js versions), Compact source files (pragma versions), compiler config files, and other ecosystem markers. Context gathering collects project metadata for enriching memory creation.

**Acceptance Scenarios**:

1. **Given** a project directory with `package.json` containing `@aspect-build/midnight-js: "0.8.3"`, **When** `fixonce detect` runs, **Then** it reports: `midnight_js: 0.8.3`

2. **Given** a project with Compact source files containing `pragma midnight ^0.15`, **When** `fixonce detect` runs, **Then** it reports: `compact_pragma: ^0.15`

3. **Given** a project with a Compact compiler config specifying version 0.15.2, **When** `fixonce detect` runs, **Then** it reports: `compact_compiler: 0.15.2`

4. **Given** a project with no Midnight-specific files, **When** `fixonce detect` runs, **Then** it reports: "No Midnight components detected" and exits cleanly

5. **Given** a detected environment, **When** `fixonce detect --format json` runs, **Then** it outputs a structured JSON object with all detected component versions, suitable for passing to other commands

6. **Given** a user runs `fixonce context`, **When** the command executes, **Then** it gathers: detected versions, git remote URL, current branch, recent commit messages, and project file structure — formatted as context for memory creation

7. **Given** detected environment data, **When** `fixonce query` is run without explicit `--version` flags, **Then** the detected versions are automatically used for metadata filtering in the read pipeline

---

### User Story 15 — Session Transcript Analysis (Priority: P1)

**Revision**: v2.0 — Rewritten with two-pass architecture after examining actual Claude Code transcript format.

**As** an agent operator, **I want** the CLI to analyze Claude Code session transcripts and propose candidate memories **so that** implicit learnings from coding sessions are captured without manual memory authoring.

**Context**: Passive harvesting, CLI-initiated (not automatic hook-based — that's deferred). Uses a two-pass architecture to minimize token usage:

**Transcript Format**: Claude Code sessions are stored as JSONL files at `~/.claude/projects/{project-hash}/{session-id}.jsonl`. Each line is a JSON object with a `type` field. Key types: `user` (user messages + tool results), `assistant` (Claude responses with text, thinking, and tool_use blocks), `system` (hook results), `progress` (hook execution). Every line has: `uuid`, `parentUuid`, `timestamp`, `sessionId`, `cwd`, `version`, `gitBranch`.

**Two-Pass Architecture**:
- **Pass 1 (Rust, no tokens)**: Parse JSONL → build exchange graph from parent/child UUIDs → group into logical exchanges (user prompt → assistant response → tool uses → tool results) → detect correction signals via heuristics → produce a compact session outline with block IDs
- **Pass 2 (Claude, targeted tokens)**: Read compact outline → identify exchanges that look like learnable moments → request full content for specific blocks by ID → propose candidate memories from the expanded blocks

**Correction Signal Heuristics** (Rust-detected, no LLM needed):
- `approach_changed`: Assistant used a different tool/file/method after a failure
- `error_recovery`: Tool use failed (exit code != 0) then was retried with different input
- `user_correction`: User message following assistant action contains negative signals ("no", "don't", "wrong", "instead", "actually")
- `multiple_edits_same_file`: Same file edited 3+ times in one exchange (iterating on a problem)
- `thinking_doubt`: Thinking block contains uncertainty markers ("wait", "actually", "I was wrong", "let me reconsider")

**Session Outline Format** (output of Pass 1, input to Pass 2):
```json
{
  "session_id": "uuid",
  "project": "/path/to/project",
  "git_branch": "feature/...",
  "duration_minutes": 42,
  "exchange_count": 38,
  "model": "claude-opus-4-6",
  "exchanges": [
    {
      "id": "EX-001",
      "timestamp": "ISO8601",
      "user_prompt_preview": "first 200 chars of user message",
      "tools_used": ["Write src/foo.rs"],
      "outcome": "success",
      "signals": [],
      "token_cost": 1200
    },
    {
      "id": "EX-007",
      "timestamp": "ISO8601",
      "user_prompt_preview": "Fix the compiler error in...",
      "tools_used": ["Read src/foo.rs", "Edit src/foo.rs", "Edit src/foo.rs"],
      "outcome": "success_after_retry",
      "signals": ["approach_changed", "error_recovery"],
      "retry_count": 2,
      "token_cost": 4500
    }
  ],
  "signal_summary": {
    "approach_changed": 3,
    "error_recovery": 2,
    "user_correction": 1,
    "multiple_edits_same_file": 4,
    "thinking_doubt": 2
  }
}
```

**Acceptance Scenarios**:

1. **Given** a Claude Code session JSONL file, **When** `fixonce analyze <path>` runs, **Then** Pass 1 (Rust) parses the JSONL, builds the exchange graph, detects correction signals, and produces a session outline — all without any LLM calls

2. **Given** the session outline, **When** Pass 2 runs via `claude -p`, **Then** Claude reads the outline, identifies which exchanges (by ID) look like learnable moments based on signals and context, and requests expansion of those specific blocks

3. **Given** Claude requests expansion of EX-007, **When** the CLI expands that block, **Then** it returns the full user message, assistant response (text + thinking), tool inputs, and tool results for that exchange only — not the entire session

4. **Given** the expanded blocks, **When** Claude proposes candidate memories, **Then** each candidate includes: suggested title, content, memory_type, language, version metadata, confidence score (0-1, threshold 0.7 to propose), and provenance (session_id, repo_url from gitBranch/cwd, task summary inferred from user prompts)

5. **Given** proposed candidates, **When** presented to the user (TTY mode), **Then** each candidate is shown with its confidence score and source exchange ID, and the user can: accept (runs write pipeline), edit then accept, skip, or reject

6. **Given** a session with no correction signals (all exchanges outcome=success, zero signals), **When** Pass 1 completes, **Then** the CLI reports "No learnable moments detected in this session" and skips Pass 2 entirely — zero LLM tokens spent

7. **Given** a session outline where Claude finds no learnable moments despite signals, **When** Pass 2 completes, **Then** Claude reports "No candidate memories detected" — it doesn't force low-quality proposals

8. **Given** an accepted candidate, **When** it enters the write pipeline, **Then** it goes through the full quality gating and dedup process — analysis does NOT bypass the write pipeline

9. **Given** a session log >100MB (EC-39), **When** Pass 1 parses it, **Then** the Rust parser streams the JSONL line-by-line without loading the full file into memory, and optionally limits analysis to the most recent N exchanges if specified via `--last <N>`

10. **Given** a file that is not valid JSONL or doesn't match the Claude Code transcript schema, **When** `fixonce analyze` attempts to parse it, **Then** it reports: "Unrecognized session format. Expected Claude Code JSONL transcript." with the specific parse error (EC-40)

11. **Given** expanded blocks containing credentials or PII, **When** Claude proposes candidate memories, **Then** the credential/PII detection from the write pipeline catches and strips sensitive content before the memory is stored

---

### User Story 16 — Claude Code Hooks (Priority: P1)

**Revision**: v1.0

**As** an LLM coding agent, **I want** FixOnce to automatically surface relevant memories during my coding session **so that** I benefit from institutional knowledge without having to explicitly query for it.

**Context**: Claude Code lifecycle hooks that integrate with the FixOnce CLI. All interventions are advisory only — warn, never block (CONSTITUTION). Hooks use the CLI under the hood, which shells out to `claude -p` for any inference needed.

**Hooks**:

| Hook | Event | Action |
|------|-------|--------|
| SessionStart | Session begins | Run `fixonce detect` for environment, pre-populate hot cache (Story 12), surface critical memories (high reinforcement, matching version) |
| UserPromptSubmit | User sends prompt | Quick-search for relevant memories based on prompt text — lightweight, no full pipeline |
| PreToolUse | Before Write/Edit | Check if the proposed file content matches any anti-memory patterns. If match score > 0.7, surface warning. |
| PostToolUse | After Write/Edit | Check if the written content matches any anti-memory patterns. If match score > 0.5, surface advisory. |
| Stop | Session ends | Surface any remaining critical reminders for the project context |

**Acceptance Scenarios**:

1. **Given** a Claude Code session starts in a Midnight project, **When** the SessionStart hook fires, **Then** it runs `fixonce detect`, populates the hot cache, and surfaces the top 3 most critical memories for this project's version profile

2. **Given** a user submits a prompt mentioning "compact maps", **When** the UserPromptSubmit hook fires, **Then** it runs a lightweight query (basic rewriting + hybrid search, no deep pipeline) and surfaces any highly relevant memories as context

3. **Given** an agent is about to write code using `Map.from()` in a Compact 0.15+ project, **When** the PreToolUse hook fires, **Then** it detects the anti-memory "never use Map.from() with Compact 0.15+" and surfaces a warning: "Warning: This pattern is known to cause issues. See memory [ID] for details."

4. **Given** the PreToolUse hook surfaces a warning, **When** the agent proceeds anyway, **Then** the action is NOT blocked — the warning is advisory only per CONSTITUTION

5. **Given** a coding session ends, **When** the Stop hook fires, **Then** it surfaces any critical reminders (e.g., "Remember to pin your Compact compiler version in CI") that are relevant to the session's detected environment

6. **Given** all hooks, **When** they execute, **Then** they complete within 3 seconds to avoid disrupting the agent's workflow — hooks that take longer should be async or skipped with a timeout warning

---

## Edge Cases

| ID | Scenario | Handling | Stories Affected |
|----|----------|----------|------------------|
| EC-01 | Agent-authored commits bypass git hooks (e.g., `--no-verify`) | CI is the safety net; CI MUST NOT be bypassable. Branch protection rules enforce this. | Story 1 |
| EC-02 | Commit only changes Rust files | Lefthook MUST only run Rust hooks (not TypeScript/Deno) for speed — use glob-based file matching | Story 1 |
| EC-03 | Commit only changes Markdown/docs | Hooks SHOULD skip code quality checks entirely — only format checks on .md files if applicable | Story 1 |
| EC-04 | `cargo audit` finds a vulnerability in a dependency | CI SHOULD warn (advisory) but NOT block merge. All other checks remain blocking. | Story 1 |
| EC-05 | CI caching is stale after Cargo.lock or pnpm-lock.yaml changes | Cache keys MUST include lockfile hashes so caches invalidate on dependency changes | Story 1 |
| EC-06 | Migration run against project that already has some tables | Supabase CLI handles idempotent migration tracking | Story 2 |
| EC-07 | Embedding column receives vector with wrong dimensions (not 1024) | Database constraint MUST reject the insert | Story 2 |
| EC-08 | Full-text search query contains special characters (`&&`, `!!`) | tsvector query MUST sanitize input to prevent syntax errors | Story 2 |
| EC-09 | Vector search returns no results above similarity threshold | Return empty array, not error | Story 2 |
| EC-10 | Edge function receives malformed JSON body | Return 400 with structured error per CONSTITUTION §VI | Story 2 |
| EC-11 | Activity log grows unbounded | Retention policy: 90 days (configurable), enforced by scheduled cleanup | Story 2 |
| EC-12 | GitHub API rate limit hit during org membership check | Cache the last known status and retry on next request. Cron job should use authenticated GitHub API with higher rate limits. | Story 3 |
| EC-13 | User's GitHub account is deleted (not just left org) | Treat as membership revocation — deactivate account on next check | Story 3 |
| EC-14 | CLI sends a challenge-response with a public key not registered to any account | Return 401 "Unknown public key. Register this key from the web dashboard." | Story 3 |
| EC-15 | User attempts to register a malformed or weak public key | Validate key format and minimum strength before storing. Reject with clear error. | Story 3 |
| EC-16 | Multiple users try to register the same public key | Reject — public keys MUST be unique across all users | Story 3 |
| EC-17 | Cron job runs while user is mid-request and revokes their access | Current request completes (JWT is still valid for that request). Next request fails if JWT expires or on-request check catches revocation. | Story 3 |
| EC-18 | Master encryption key is lost or corrupted | All secrets become unrecoverable. MUST have a documented backup/recovery procedure for the master key. | Story 4 |
| EC-19 | Secret name doesn't exist when CLI requests it | Return 404 with clear error: "Secret 'X' not found. Create it from the dashboard." | Story 4 |
| EC-20 | Concurrent requests to update the same secret | Last-write-wins with optimistic locking (updated_at check). Return 409 on conflict. | Story 4 |
| EC-21 | Edge function memory limits during decryption of many secrets | Secrets are retrieved one at a time, never bulk-decrypted. Each request decrypts only the requested secret. | Story 4 |
| EC-22 | CLI process crashes between receiving secret and discarding it | OS process memory is freed on crash — acceptable risk. Secret never reaches disk regardless. | Story 4 |
| EC-23 | Memory content exceeds embedding model's token limit | Chunk content, embed each chunk, store primary embedding from most relevant chunk. Content stored in full. | Story 5 |
| EC-24 | Memory created with conflicting version metadata (e.g., compact_pragma=0.14 but content references 0.15 features) | Accept and store — the write pipeline quality gate (Story 6) should catch and flag this, not the CRUD layer | Story 5 |
| EC-25 | Embedding generation fails (VoyageAI down) | CLI retries with exponential backoff (3 attempts). On final failure, stores memory without embedding and marks it as "pending_embedding" for later retry. | Story 5 |
| EC-26 | Write pipeline Claude call times out | CLI retries once. On second failure, store the memory with a flag "pipeline_incomplete" — let it through with reduced confidence rather than losing the knowledge | Story 6 |
| EC-27 | Dedup comparison set is empty (first memory in the store) | Skip dedup step entirely — outcome is always "new" | Story 6 |
| EC-28 | Query returns memories with decay_score near threshold | Include them but visually indicate low confidence — "This memory is aging and may be outdated" | Story 7 |
| EC-29 | All pipeline Claude calls fail (Claude outage) | Return raw search results without LLM processing, clearly marked as "unranked" — degraded but functional | Story 7 |
| EC-30 | Decay cron job runs while a memory is being queried | Use database-level row locking. Query sees the pre-decay score; decay applies after query completes. | Story 8 |
| EC-31 | Memory has reinforcement_score=100 and decay_score=0.05 — heavily used but also decaying | reinforcement_score slows but does not prevent decay. If it drops below threshold, it's soft-deleted even if heavily reinforced. Exceptional memories should not decay below threshold due to continuous use. | Story 8 |
| EC-32 | Contradiction pair involves a soft-deleted memory | Dismiss the contradiction — resolved by deletion | Story 10 |
| EC-33 | Same two memories flagged as contradictory multiple times | Deduplicate contradiction pairs — only one active pair per memory combination | Story 10 |
| EC-34 | Session hot cache hits memory limit (project matches thousands of memories) | Cap hot cache at 50 memories, ranked by signature overlap score | Story 12 |
| EC-35 | TUI launched in a terminal with very small dimensions (< 80x24) | Show a minimum-size warning and fall back to simplified layout | Story 13 |
| EC-36 | CLI piped to another command (non-TTY) and user accidentally runs `fixonce tui` | Detect non-TTY, print error: "TUI requires an interactive terminal. Use CLI commands for scripted use." | Story 13 |
| EC-37 | `claude -p` is not installed or not in PATH | CLI MUST detect this on first inference-requiring command and report: "Claude CLI not found. Install it from https://claude.com/claude-code" | Story 13 |
| EC-38 | Project has no git remote (local-only repo) | `fixonce detect` reports git info as "local only" — no repo_url for provenance | Story 14 |
| EC-39 | Session log file is extremely large (>100MB) | Warn user, process in chunks, or offer to analyze only the most recent N exchanges | Story 15 |
| EC-40 | Session log format is unrecognized | Report: "Unrecognized session format. Expected Claude Code session log." with supported formats listed | Story 15 |
| EC-41 | Hook timeout (>3 seconds) | Skip the hook with a warning: "FixOnce hook timed out. Continuing without memory surfacing." | Story 16 |
| EC-42 | FixOnce CLI not installed but hooks are configured | Hook script detects missing CLI, logs warning, and exits 0 (never block the agent) | Story 16 |
| EC-43 | Hook fires but user is not authenticated | Skip memory surfacing silently — hooks should never prompt for auth | Story 16 |

---

## Requirements

### Functional Requirements

| ID | Requirement | Stories | Confidence |
|----|-------------|---------|------------|
| FR-001 | Lefthook pre-commit hooks MUST check formatting and linting for changed files only, scoped to the correct tooling track (Rust/TypeScript/Deno) | Story 1 | High |
| FR-002 | GitHub Actions CI MUST run all quality checks (lint, format, typecheck, test) in parallel jobs on every PR to `main` | Story 1 | High |
| FR-003 | GitHub Actions MUST require all check jobs to pass before merge is enabled (branch protection rule) | Story 1 | High |
| FR-004 | CI MUST cache Rust dependencies (cargo registry + target) and Node dependencies (pnpm store) with lockfile-based cache keys | Story 1 | High |
| FR-005 | A single top-level command (e.g., `make check`) MUST run all quality checks for all three tooling tracks locally | Story 1 | High |
| FR-006 | `cargo audit` MUST run in CI as an advisory check (non-blocking) | Story 1 | High |
| FR-007 | Lefthook MUST be auto-installed on repo setup (e.g., via postinstall hook or documented setup step) | Story 1 | High |
| FR-008 | All 7 tables MUST have RLS enabled with deny-by-default policies | Story 2 | High |
| FR-009 | The `memory` table MUST have an HNSW index on the embedding column for vector similarity search | Story 2 | High |
| FR-010 | The `memory` table MUST have a GIN index on the tsvector column for full-text search | Story 2 | High |
| FR-011 | Every edge function MUST verify authentication via `supabase.auth.getUser()` before any database operation | Story 2 | High |
| FR-012 | Every edge function MUST validate input against a Zod schema before processing | Story 2 | High |
| FR-013 | Every mutating edge function MUST log the operation to `activity_log` with: user_id, action, entity_type, entity_id, metadata, timestamp | Story 2 | High |
| FR-014 | The search edge function MUST accept a `search_type` parameter (hybrid\|fts\|vector) defaulting to hybrid | Story 2 | High |
| FR-015 | The hybrid search Postgres RPC function MUST combine FTS ts_rank and vector cosine similarity using Reciprocal Rank Fusion | Story 2 | High |
| FR-016 | Migrations MUST be managed via Supabase CLI (`supabase migration new`, `supabase db push`) | Story 2 | High |
| FR-017 | The embedding column MUST enforce `vector(1024)` dimension constraint | Story 2 | High |
| FR-018 | The `activity_log` table MUST have a retention policy of 90 days (configurable) | Story 2 | High |
| FR-019 | Web dashboard MUST authenticate via GitHub OAuth through Supabase, with post-login org membership verification | Story 3 | High |
| FR-020 | Post-login edge function MUST check the user's GitHub org membership via GitHub API and deny access for non-members | Story 3 | High |
| FR-021 | Org membership MUST be re-verified on every authenticated request with a 1-hour cache TTL | Story 3 | High |
| FR-022 | A Supabase cron job MUST run twice daily to sweep all active users and deactivate those no longer in the authorized org | Story 3 | High |
| FR-023 | CLI MUST authenticate via challenge-response: request nonce → sign with private key → verify → receive 8-hour JWT | Story 3 | High |
| FR-024 | CLI JWTs MUST expire after 8 hours | Story 3 | High |
| FR-025 | Users MUST be able to register zero or more CLI public keys from the web dashboard | Story 3 | High |
| FR-026 | Public keys MUST be unique across all users — no two users can register the same key | Story 3 | High |
| FR-027 | Revoking a CLI public key MUST immediately invalidate any JWT issued for that key | Story 3 | High |
| FR-028 | Secrets MUST be encrypted using AES-256-GCM in edge functions. The encryption master key MUST be a Supabase environment secret, never stored in the database. | Story 4 | High |
| FR-029 | The `secrets` table MUST store only: name, ciphertext, IV, metadata (created_at, updated_at, created_by). Never plaintext. | Story 4 | High |
| FR-030 | The get-secret edge function MUST return decrypted plaintext only to authenticated, authorized users. | Story 4 | High |
| FR-031 | The CLI MUST never write received secrets to disk (no config files, temp files, environment variable persistence, or logs) | Story 4 | High |
| FR-032 | Secret access MUST be logged to `activity_log` with: secret name, user_id, timestamp. Never the secret value. | Story 4 | High |
| FR-033 | A master key rotation procedure MUST exist that re-encrypts all secrets in a single transaction | Story 4 | High |
| FR-034 | Secrets MUST be retrieved one at a time per request — no bulk decryption endpoint | Story 4 | High |
| FR-035 | Memory CRUD edge functions MUST validate all input against Zod schemas including memory_type enum, source_type enum, and version metadata format | Story 5 | High |
| FR-036 | Memory creation MUST generate a voyage-code-3 embedding (1024 dims) for the content via VoyageAI API | Story 5 | High |
| FR-037 | Memory deletion MUST be soft-delete (mark as deleted, preserve in DB) | Story 5 | High |
| FR-038 | The raw embedding vector MUST NOT be returned in standard get/query responses — only via explicit flag | Story 5 | High |
| FR-039 | The write pipeline MUST scan for credentials, API keys, and PII before any other processing | Story 6 | High |
| FR-040 | The write pipeline MUST use `claude -p` for quality gating and dedup — Claude is the sole inference engine | Story 6 | High |
| FR-041 | Dedup MUST compare against top-N most similar existing memories by cosine similarity and return one of: new, discard, replace, update, merge | Story 6 | High |
| FR-042 | Replace/update/merge outcomes MUST create lineage records linking old memories to new | Story 6 | High |
| FR-043 | The read pipeline MUST support all listed query techniques as composable pipeline stages | Story 7 | High |
| FR-044 | The read pipeline MUST support all listed result refinement techniques as composable pipeline stages | Story 7 | High |
| FR-045 | Every CLI query command MUST support `--format text`, `--format json`, and `--format toon` output | Story 7 | High |
| FR-046 | Memory decay MUST combine temporal decay (configurable half-life) with event-driven acceleration | Story 8 | High |
| FR-047 | Positive feedback ("helpful") MUST increase reinforcement_score and slow decay | Story 8 | High |
| FR-048 | "Damaging" feedback MUST sharply accelerate decay and flag for review | Story 8 | High |
| FR-049 | Memories below configurable decay threshold MUST be soft-deleted automatically | Story 8 | High |
| FR-050 | Anti-memories MUST include: anti-pattern description, reason, alternative, and version constraints | Story 9 | High |
| FR-051 | Anti-memories MUST be surfaced with higher priority than regular memories when version constraints match | Story 9 | High |
| FR-052 | Contradiction pairs MUST store: both memory IDs, detection date, resolution status, tiebreaker votes | Story 10 | High |
| FR-053 | Contradiction resolution MUST require 3+ tiebreaker votes before applying decay/reinforcement | Story 10 | High |
| FR-054 | Lineage MUST be stored automatically on all replace/update/merge/feedback operations | Story 11 | High |
| FR-055 | Lineage MUST NOT be returned in standard query responses — only via explicit request | Story 11 | High |
| FR-056 | Memory signatures MUST be pre-computed on memory creation/update based on content analysis | Story 12 | High |
| FR-057 | Session hot cache MUST be capped at 50 memories, ranked by signature overlap with session profile | Story 12 | High |
| FR-058 | Hot cache queries MUST return results in under 50ms | Story 12 | High |
| FR-059 | The CLI MUST be a single statically-linked Rust binary with zero runtime dependencies | Story 13 | High |
| FR-060 | Every CLI command MUST support `--format text` (default), `--format json`, and `--format toon` output | Story 13 | High |
| FR-061 | The TUI MUST be built with ratatui and provide: search, browse, create, edit, feedback, activity stream, key management, and system health | Story 13 | High |
| FR-062 | The CLI MUST detect non-TTY mode and suppress all interactive elements (TUI, prompts, progress bars) | Story 13 | High |
| FR-063 | CLI errors MUST include: what happened, why, and what to do about it — in both human-readable and structured formats | Story 13 | High |
| FR-064 | `fixonce detect` MUST scan for: package.json (midnight-js), Compact pragma versions, compiler config, and other Midnight ecosystem markers | Story 14 | High |
| FR-065 | Detected environment MUST be automatically used for metadata filtering in queries unless overridden by explicit flags | Story 14 | High |
| FR-066 | `fixonce context` MUST gather: detected versions, git remote URL, branch, recent commits, and project structure | Story 14 | High |
| FR-067 | `fixonce analyze` MUST use `claude -p` to identify corrections, discoveries, gotchas, and best practices from session transcripts | Story 15 | High |
| FR-068 | Proposed candidate memories from analysis MUST go through the full write pipeline — no bypass | Story 15 | High |
| FR-069 | All Claude Code hooks MUST be advisory only — warn, never block agent actions | Story 16 | High |
| FR-070 | SessionStart hook MUST run environment detection and populate the hot cache | Story 16 | High |
| FR-071 | PreToolUse hook MUST check proposed content against anti-memory patterns and surface warnings for matches above 0.7 score | Story 16 | High |
| FR-072 | All hooks MUST complete within 3 seconds or timeout gracefully | Story 16 | High |
| FR-073 | Hooks MUST never prompt for authentication or block on missing CLI | Story 16 | High |

### Key Entities

| Entity | Description | Key Attributes |
|--------|-------------|----------------|
| Memory | A stored piece of knowledge (correction, gotcha, best practice, anti-pattern, discovery) | content, embeddings, metadata, provenance, decay score, reinforcement score |
| Anti-Memory | A first-class "do NOT do this" artifact | Same as Memory plus: proactive surfacing rules, pattern-match triggers |
| Memory Lineage | The provenance chain of a memory | parent memory, replacement history, merge history, feedback history |
| Memory Signature | Pre-computed relevance fingerprint | correlated files, error patterns, SDK method calls, session relevance score |
| Secret | An encrypted API key or credential stored server-side | encrypted value, access scope, last accessed, rotation status |
| Feedback | Agent or human evaluation of a memory | memory ID, rating (helpful/outdated/damaging), context, timestamp |

---

## Success Criteria

| ID | Criterion | Measurement | Stories |
|----|-----------|-------------|---------|
| SC-001 | No commit can reach `main` without passing all lint, format, typecheck, and test checks | Branch protection rules enforced; 0 unprotected merges | Story 1 |
| SC-002 | Local quality checks are runnable with a single command and match CI behavior | `make check` exits 0 when CI would pass, exits non-zero when CI would fail | Story 1 |
| SC-003 | Pre-commit hooks complete quickly for typical commits | Lefthook hooks complete in under 30 seconds for commits touching 5-10 files | Story 1 |
| SC-004 | Edge functions return structured errors | 0 generic 500 errors in normal operation — all errors have type, message, and suggested action | Story 2 |
| SC-005 | RLS blocks all unauthorized access | 100% of direct database queries from unauthenticated clients return zero rows | Story 2 |
| SC-006 | Hybrid search performs well at scale | Results returned within 500ms for a database with 10,000 memories | Story 2 |
| SC-007 | Unauthorized users cannot access any data | Non-org members receive clear denial at login; ex-members lose access within 1 hour of departure | Story 3 |
| SC-008 | CLI authentication is fast and reliable | Challenge-response completes in under 2 seconds; JWT issuance adds no perceptible latency | Story 3 |
| SC-009 | Secrets are unrecoverable from a database dump | Raw database export contains zero plaintext secrets — only ciphertext verifiable by inspection | Story 4 |
| SC-010 | Secret retrieval is fast | get-secret edge function responds in under 300ms including decryption | Story 4 |
| SC-011 | Memory CRUD is reliable | Create/read/update/delete operations succeed on first attempt 99%+ of the time | Story 5 |
| SC-012 | Write pipeline catches duplicates | Less than 5% of stored memories are near-duplicates of existing memories | Story 6 |
| SC-013 | Write pipeline blocks credentials | 100% of memories containing API keys, private keys, or passwords are rejected before storage | Story 6 |
| SC-014 | Read pipeline returns relevant results | Top-3 results include the correct answer for 80%+ of well-formed queries (measured by agent feedback) | Story 7 |
| SC-015 | Memory dynamics converge | Over 90 days of use, average memory quality (measured by positive feedback ratio) improves monotonically | Story 8 |
| SC-016 | Anti-memories prevent mistakes | Agent feedback "this warning saved me" ratio exceeds 60% for surfaced anti-memories | Story 9 |
| SC-017 | Contradictions resolve within 7 days | 80%+ of detected contradictions reach resolution threshold within 7 days of detection | Story 10 |
| SC-018 | Hot cache provides fast cold-start | First query in a new session returns results in under 200ms when hot cache is populated | Story 12 |
| SC-019 | CLI binary is portable | Single binary runs on macOS (ARM + x86), Linux (x86_64), without any runtime dependencies | Story 13 |
| SC-020 | TUI is responsive | TUI renders and responds to input within 100ms for all navigation operations | Story 13 |
| SC-021 | Environment detection is accurate | Correctly identifies Midnight component versions for 95%+ of standard project layouts | Story 14 |
| SC-022 | Session analysis surfaces real learnings | At least 50% of proposed candidate memories from transcript analysis receive "accept" from the operator | Story 15 |
| SC-023 | Hooks don't slow agents down | All hooks complete within 3 seconds, 95th percentile | Story 16 |
| SC-024 | Hooks prevent known mistakes | PreToolUse anti-memory warnings reduce repeated anti-pattern occurrences by 50%+ over 30 days | Story 16 |

---

## Appendix: Story Revision History

*Major revisions to graduated stories. Full details in `archive/REVISIONS.md`*

| Date | Story | Change | Reason |
|------|-------|--------|--------|
| *No revisions yet* | - | - | - |
