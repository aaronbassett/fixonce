# Spec Analysis: fixonce-memory-layer

Comprehensive synthesis of all discovery documents for implementation planning.

Source documents:
- `discovery/SPEC.md` — Full feature specification (7 stories, 42 functional requirements)
- `discovery/archive/DECISIONS.md` — 23 decisions (D1-D23)
- `discovery/STATE.md` — Discovery state and dependency graph
- `discovery/OPEN_QUESTIONS.md` — 7 resolved questions (Q1-Q7)
- `discovery/archive/REVISIONS.md` — No revisions recorded

---

## 1. Technical Context

### Language and Runtime
- **Language**: TypeScript (strict mode)
- **Runtime**: Node.js for server-side (MCP server, CLI, Web UI backend), browser for Web UI frontend

### Storage
- **Database**: Supabase Postgres + pgvector (D8)
  - Replaced initial SQLite + Chroma plan (D1 revised by D8)
  - Single store for metadata, FTS, and vector search
  - Hybrid queries in one SQL statement
  - Free tier sufficient for MVP
- **FTS**: `tsvector` on `title + content + summary + tags`
- **Vector**: pgvector with `vector(1024)` column, cosine distance

### Embeddings
- **Model**: Voyage AI `voyage-code-3` (D9)
  - 1024 dimensions
  - Code-optimized (purpose-built for code retrieval)
  - API-based generation
  - Stored in pgvector column

### LLM Usage
- **Provider**: OpenRouter (D14)
  - Cheap/small models (e.g., Haiku, Gemma) for:
    - Quality gate evaluation
    - Duplicate detection (4-outcome)
    - Query rewriting
    - Reranking
  - Not the same model the agent uses for its primary work

### Web UI
- **Stack**: React + Vite (D23)
- **Realtime**: WebSocket or SSE for activity stream
- **Started via**: `fixonce web` command

### Indexes
- **GIN** on `version_predicates` (jsonb `?` operator)
- **GIN** on `tsvector` (full-text search)
- **pgvector index** on `embedding` (cosine distance)
- **GIN** implied on `tags` (array type)

---

## 2. Architecture Overview

### Module Boundaries

```
                    +-----------+     +----------+     +---------+
                    | MCP Server|     |   CLI    |     | Web UI  |
                    |  (7 tools)|     |(8 cmds)  |     |(6 views)|
                    +-----+-----+     +----+-----+     +----+----+
                          |                |                 |
                          +-------+--------+---------+-------+
                                  |                  |
                           +------v------+    +------v------+
                           | Write Path  |    | Read Path   |
                           | (pipeline)  |    | (pipeline)  |
                           +------+------+    +------+------+
                                  |                  |
                           +------v------------------v------+
                           |     Storage Layer               |
                           |  (Supabase client, schema,     |
                           |   migrations, embedding gen)    |
                           +------+-------------------------+
                                  |
                           +------v------+
                           |  Supabase   |
                           |  Postgres   |
                           |  + pgvector |
                           +-------------+
```

### Write Pipeline (Story 2)
```
Memory submitted
  |-- created_by: human --> Store immediately --> Async embedding
  |-- created_by: ai -->
        |-- Quality Gate LLM (cheap model via OpenRouter)
        |     |-- Reject (vague, too specific, obvious) --> Return reason
        |     |-- Accept -->
        |           |-- Similarity search against existing memories
        |           |     |-- No similar --> Store --> Async embedding
        |           |     |-- Duplicate --> Discard incoming
        |           |     |-- Better version --> Replace existing
        |           |     |-- Additional details --> Update existing
        |           |     |-- Complementary --> Merge into new combined
```

### Read Pipeline (Story 3)
```
Context at hook point
  --> Stage 1: Query Rewriting (LLM reformulates context into search queries)
  --> Stage 2: Hybrid Search (Supabase: metadata filters + vector similarity)
  --> Stage 3: Reranking (LLM consolidates, deduplicates, ranks)
  --> Two-tier response:
       |-- Top 5: Full memory content
       |-- Next 10-20: Summary + relevancy score + cache key
```

### Shared Types/Schema Module
- Memory entity schema
- Feedback entity schema
- Activity Log entity schema
- Version predicate types (12 component keys)
- Enum types (memory_type, source_type, created_by, feedback tags, suggested_action)
- Verbosity level definitions

---

## 3. Complete Data Model

### Memory Entity (Story 1)

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `id` | uuid | PK | Auto-generated |
| `title` | text | NOT NULL | Short scannable name (D10) |
| `content` | text | NOT NULL | Full memory in markdown (D7) |
| `summary` | text | NOT NULL | One-line summary for search results |
| `memory_type` | enum | `guidance`, `anti_pattern` | Guidance ranks higher (D10) |
| `source_type` | enum | `correction`, `discovery`, `instruction` | |
| `created_by` | enum | `ai`, `human`, `human_modified` | (D5) |
| `source_url` | text | NULLABLE | Link to PR comment, CI run, etc. |
| `tags` | text[] | DEFAULT '{}' | Freeform tags |
| `language` | text | NOT NULL | e.g., `compact`, `typescript` |
| `version_predicates` | jsonb | NULLABLE | Version constraints (Story 4) |
| `project_name` | text | NULLABLE | e.g., `fixonce` |
| `project_repo_url` | text | NULLABLE | e.g., `https://github.com/org/repo` |
| `project_workspace_path` | text | NULLABLE | Local path context |
| `confidence` | float | DEFAULT 0.5 | Range 0.0-1.0 |
| `surfaced_count` | int | DEFAULT 0 | Times returned in queries |
| `last_surfaced_at` | timestamptz | NULLABLE | Last query return |
| `enabled` | boolean | DEFAULT true | Kill switch |
| `created_at` | timestamptz | DEFAULT now() | Auto-set |
| `updated_at` | timestamptz | DEFAULT now() | Auto-updated |
| `embedding` | vector(1024) | NULLABLE | Voyage code-3 embeddings |

### Feedback Entity (Story 5)

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `id` | uuid | PK | Auto-generated |
| `memory_id` | uuid | FK -> Memory.id, NOT NULL | |
| `text` | text | NULLABLE | Free-text feedback |
| `tags` | text[] | DEFAULT '{}' | Enum values: `helpful`, `not_helpful`, `damaging`, `accurate`, `somewhat_accurate`, `somewhat_inaccurate`, `inaccurate`, `outdated` |
| `suggested_action` | enum | `keep`, `remove`, `fix`, NULLABLE | |
| `created_at` | timestamptz | DEFAULT now() | |

### Activity Log Entity (Story 7)

| Field | Type | Constraints | Notes |
|-------|------|-------------|-------|
| `id` | uuid | PK | Auto-generated |
| `operation` | enum | `query`, `create`, `update`, `feedback`, `detect` | |
| `memory_id` | uuid | FK -> Memory.id, NULLABLE | Not all ops relate to a specific memory |
| `details` | jsonb | NOT NULL | Operation-specific payload |
| `created_at` | timestamptz | DEFAULT now() | |

### Version Predicates JSONB Structure (Story 4, D18)

```json
{
  "compact_compiler": ["0.28.0", "0.29.0"],
  "compact_runtime": ["0.14.0"],
  "network": ["preprod", "preview"],
  "midnight_js": ["3.0.0", "3.1.0"]
}
```

**Rules**:
- Keys: 12 component names from Midnight support matrix
- Values: arrays of exact version strings (not semver ranges)
- OR within a component (any listed version matches)
- AND across components (all constrained components must match)
- Missing key = no constraint on that component
- `null` = applies to all versions (universal)

**12 Supported Component Keys**:
`network`, `node`, `compact_compiler`, `compact_runtime`, `compact_js`, `onchain_runtime`, `ledger`, `wallet_sdk`, `midnight_js`, `dapp_connector_api`, `midnight_indexer`, `proof_server`

### Indexes

| Target | Type | Purpose |
|--------|------|---------|
| `version_predicates` | GIN | JSONB `?` operator for version filtering |
| `title + content + summary + tags` | GIN (tsvector) | Full-text search |
| `embedding` | pgvector (ivfflat or hnsw) | Vector similarity search |
| `tags` | GIN | Array containment queries |
| `enabled` | btree (partial) | Filter disabled memories |

---

## 4. API Surface

### MCP Tools (Story 5) — 7 Tools

| Tool | Description | Key Parameters |
|------|-------------|----------------|
| `fixonce_create_memory` | Submit a new memory | All memory fields; routes through quality gate + dedup for AI, direct store for human |
| `fixonce_query` | Query memories with configurable pipeline | See full schema below |
| `fixonce_expand` | Expand cache key to full content | `cache_key: string`, `verbosity: small\|medium\|large` |
| `fixonce_get_memory` | Get a specific memory by ID | `id: uuid` |
| `fixonce_update_memory` | Update an existing memory | `id: uuid`, partial fields; regenerates embedding if content changed |
| `fixonce_feedback` | Provide feedback on a memory | See full schema below |
| `fixonce_detect_environment` | Scan project files for Midnight versions | Returns JSON matching version predicate key format |

#### `fixonce_query` Full Parameter Schema

```
fixonce_query:
  # Search
  query: string                              # Context or search text

  # Pipeline control
  rewrite: bool = true                       # LLM query rewriting
  type: simple|vector|hybrid = hybrid        # Search type
  rerank: bool = true                        # LLM reranking pass

  # Filters
  tags: string[]?                            # Filter by tags
  language: string?                          # Filter by language
  project_name: string?                      # Filter by project
  memory_type: guidance|anti_pattern?        # Filter by type
  created_after: datetime?                   # Date filter
  updated_after: datetime?                   # Date filter

  # Result budgeting (one or the other)
  max_results: int = 5                       # Max full results
  max_tokens: int?                           # Approx token budget (overrides max_results)

  # Result shape
  verbosity: small|medium|large = small      # Response detail level
```

**Verbosity Levels**:
- `small`: id, title, content, summary, memory_type, relevancy_score
- `medium`: small + tags, language, version_predicates, created_by, source_type, created_at, updated_at
- `large`: all fields including source_url, project details, confidence, surfaced_count, feedback summary

**Simple mode** (`rewrite=false, type=simple, rerank=false`): Acts as filtered list, no LLM calls (D21).

#### `fixonce_feedback` Full Parameter Schema

```
fixonce_feedback:
  memory_id: uuid                            # Required
  text: string?                              # Free-text feedback
  tags: enum[]?                              # helpful, not_helpful, damaging, accurate,
                                             # somewhat_accurate, somewhat_inaccurate,
                                             # inaccurate, outdated
  suggested_action: keep|remove|fix?         # Suggested action
```

**Behavior**:
- Multiple feedback entries accumulate per memory
- Any `remove` or `fix` feedback flags the memory for immediate human review in Web UI
- Memories with negative feedback are de-ranked but still surfaced with a warning

### CLI Commands (Story 6) — 8 Commands

| Command | Description | Key Flags |
|---------|-------------|-----------|
| `fixonce create` | Create a memory | `--title`, `--content`, `--language`, `--source-type`, stdin pipe support |
| `fixonce query` | Query memories | Mirrors MCP `fixonce_query`: `--no-rewrite`, `--type`, `--no-rerank`, `--language`, `--max-results`, `--json` |
| `fixonce get` | Get memory by ID | `<uuid>`, `--json` |
| `fixonce update` | Update a memory | `<uuid>`, partial field flags |
| `fixonce feedback` | Provide feedback | `<uuid>`, `--tags`, `--action`, `--text` |
| `fixonce detect` | Detect Midnight versions | Scans project files (renamed from `env` per D22) |
| `fixonce serve` | Start MCP server | Separate process from Web UI |
| `fixonce web` | Start Web UI server | Separate process from MCP server |

**Output Modes**: Human-readable by default, `--json` flag for machine consumption.

### Hook Integration Points (Story 3) — 5 Hooks + 2 Patterns

| Hook | Context | Mode | Use Case |
|------|---------|------|----------|
| `SessionStart` | Project deps, SDK versions, compiler version | Blocking, project-level | Surface critical version-specific memories |
| `UserPromptSubmit` | User prompt + project context | Blocking quick + async deep (mid-run injection) | Task-specific memories |
| `PreToolUse` | Tool name + arguments | Blocking | Check writes against anti-patterns |
| `PostToolUse` | Tool result | Blocking | Flag anti-patterns in what was written |
| `Stop` | Session context | Blocking | Final critical error check |
| Agent-initiated | Whatever context agent provides | On-demand via MCP | Explicit memory queries |
| Agent Teams monitor | Watches other agents' activity | Proactive via SendMessage | Continuous monitoring |

---

## 5. Story Dependencies

### Dependency Graph

```
Story 1: Memory Storage & Schema [P1, FOUNDATION]
  |
  +---> Story 2: Memory Creation (Write Path) [P1]
  |       |
  |       +---> Story 4 integrates version_predicates into write path
  |
  +---> Story 3: Memory Retrieval (Read Path) [P1]
  |       |
  |       +---> Story 4 integrates version filtering into read path
  |
  +---> Story 4: Version-Scoped Metadata [P1]
  |       (integrates into Stories 2 & 3)
  |
  +---> Story 5: MCP Server Interface [P1]
  |       (exposes Stories 2 & 3 as MCP tools)
  |
  +---> Story 6: CLI Interface [P2]
  |       (exposes Stories 2 & 3 as CLI commands)
  |
  +---> Story 7: Web UI [P2]
          (exposes Stories 1, 2, 3 as GUI)
```

### Implementation Ordering

1. **Story 1** (Storage & Schema) — Must be first. Foundation for everything.
2. **Story 4** (Version-Scoped Metadata) — Can be implemented alongside Story 1 since it defines the version_predicates format and detection logic. Integrates into Stories 2 & 3.
3. **Story 2** (Write Path) — Depends on Story 1 schema being complete. Includes quality gate, dedup, async embedding.
4. **Story 3** (Read Path) — Depends on Story 1. Can be developed in parallel with Story 2 once schema exists. Includes retrieval pipeline, hook integration.
5. **Story 5** (MCP Server) — Depends on Stories 2 & 3 for the pipelines it exposes.
6. **Story 6** (CLI) — Depends on Stories 2 & 3. Can be developed in parallel with Story 5.
7. **Story 7** (Web UI) — Depends on Stories 1, 2, 3. Highest dependency count; should be last or parallel with 5/6.

### Parallelization Opportunities
- Stories 2 and 3 can be developed in parallel once Story 1 is done
- Stories 5 and 6 can be developed in parallel once Stories 2 and 3 are done
- Story 7 can start its scaffolding (React + Vite) early, but views depend on backend pipelines

---

## 6. Key Design Decisions

### D8: Supabase over SQLite + Chroma
**Why**: Single store eliminates dual-store sync complexity. Hybrid queries (metadata filter + vector similarity) execute in one SQL statement. Free tier sufficient for MVP. User confirmed hosted services are acceptable.
**Impact**: Revises D1. No Chroma dependency. Requires Supabase project setup.

### D11: Quality Gate AI-only (Humans Bypass)
**Why**: Humans have already curated their input. AI needs filtering for noise. Web UI shows possible duplicates as user types (debounced) to prevent duplicates without blocking.
**Impact**: Two code paths in write pipeline based on `created_by`.

### D13: 4-Outcome Duplicate Detection
**Why**: Simple threshold-based dedup is too crude. LLM can make nuanced decisions about whether memories complement, supersede, or duplicate each other.
**Outcomes**: discard (true duplicate), replace (better version), update (additional details), merge (complementary into new combined).
**Impact**: LLM call at write time adds latency + cost. Merge outcome creates new memory and disables originals.

### D15: Multi-Hook Integration
**Why**: Different hook points provide different context levels and urgency. Layered approach ensures memories surface at the right moment.
**5 hooks**: SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop
**Plus**: Agent-initiated MCP calls, Agent Teams monitor (experimental)

### D16: Two-Tier Budgeting
**Why**: Respects context window budget while not discarding potentially relevant memories. Agent has agency to pull more detail on demand.
**Format**: Top 5 full + next 10-20 as summaries with cache keys.

### D18: Version Predicates (JSONB Arrays)
**Why**: Array-of-versions is simpler than semver ranges given the small number of Midnight versions. JSONB `?` operator is GIN-indexable.
**Logic**: OR within a component, AND across components. Missing key = unconstrained.

### D19: Feedback Model (Replaces Disable/Flag)
**Why**: Richer signal than binary flag. Multiple agents provide independent feedback. Natural escalation to human review. Preserves memory for review rather than silently disabling.
**Impact**: Separate Feedback entity. Dashboard flagging for `remove`/`fix` suggestions.

---

## 7. Deferred Items (Out of v1 Scope)

| Item | Source | Notes |
|------|--------|-------|
| Reinforcement scoring and lifecycle | D4 | Confidence decay, automatic flagging of stale memories |
| Contradiction detection and resolution | D4 | Reranker conflict handling between memories |
| Memory composition and clustering | D4 | Grouping related memories into coherent guidance clusters |
| Team-scoped sharing | D3 | Multi-user/multi-agent sharing, requires auth |
| Bulk operations in Web UI | D23 | Multi-select disable/delete/tag |
| Exportable memory packs | STATE.md | Curated collections for community sharing |

---

## 8. Risks and Unknowns

### High Risk

**Agent Teams is experimental (Story 3, Scenario 10)**
- D17 explicitly notes Agent Teams is experimental
- File watcher mechanism needed for monitor pattern
- PostToolUse hook on Write/Edit as notification trigger
- STATE.md marks Story 3 as "Medium" revision risk specifically because of this
- **Mitigation**: This is one scenario in Story 3, not the core path. Can be deferred or simplified without blocking the rest of Story 3.

**AsyncIterable<SDKUserMessage> for mid-run injection (Story 3, Scenario 3)**
- UserPromptSubmit does a blocking quick check + async deep search
- Deep search results injected mid-run via `AsyncIterable<SDKUserMessage>`
- Requires SDK research to confirm this API exists and works as expected
- **Mitigation**: If not available, fall back to returning all results in the blocking phase (slower but functional).

### Medium Risk

**Performance of multi-stage retrieval pipeline (Story 3)**
- Three LLM calls per query (rewrite, search, rerank) plus embedding generation
- UserPromptSubmit blocking check must be sub-second (Q5 resolved)
- SessionStart can take 1-2 seconds
- **Mitigation**: Pipeline is configurable — `rewrite=false, type=simple, rerank=false` skips all LLM calls.

**OpenRouter model selection**
- D14 says "cheap model" but specific model not yet chosen
- Quality gate prompt must work well with smaller models (Haiku, Gemma mentioned)
- Different tasks (quality gate, dedup, rewriting, reranking) may need different models
- **Mitigation**: OpenRouter provides model flexibility; can be tuned post-v1.

### Low Risk

**Monorepo structure decisions**
- Spec describes multiple packages (MCP server, CLI, Web UI, shared types) but monorepo tooling not specified
- Need to decide on workspace manager (npm workspaces, pnpm, turborepo, etc.)
- **Mitigation**: Standard TypeScript monorepo patterns are well-established.

**Supabase free tier limits**
- Free tier used for MVP; may hit limits under heavy agent use
- **Mitigation**: Supabase paid tier is affordable; usage will be low in early adoption.

**Embedding cost per memory write**
- Voyage AI API call per memory creation/update
- **Mitigation**: Async generation means writes are not blocked. Cost is per-memory, not per-query.

---

## 9. Implementation Notes

### Things the Spec Explicitly Does NOT Define
- Specific OpenRouter model IDs for quality gate / dedup / rewriting / reranking
- Monorepo structure (workspace layout, build tooling)
- Supabase migration strategy or tooling
- Authentication mechanism for Supabase (API key management)
- Exact similarity threshold for dedup (LLM-driven, not threshold-based per D13)
- Cache key format or TTL
- WebSocket vs SSE choice for realtime (spec says "WebSocket or SSE")
- Environment variable names for API keys (Supabase, Voyage AI, OpenRouter)
- Error handling / retry strategies for external API calls
- Rate limiting strategy for LLM calls

### Cross-Cutting Concerns
- **Activity logging (FR-042)**: ALL operations must be logged for the activity stream (Story 7). This is a cross-cutting concern that must be built into every write path, read path, and feedback operation.
- **Embedding regeneration (FR-006)**: Must trigger on any content update, including dedup merge/update outcomes and Web UI edits.
- **created_by state machine**: `ai` or `human` at creation; Web UI edits change to `human_modified` (FR-040).
- **Disabled memory filtering (FR-005)**: Every retrieval query must exclude `enabled = false`.
