# Data Model: FixOnce v2

## Entity Relationship Diagram

```text
┌─────────────┐     ┌──────────────┐     ┌───────────────────┐
│   memory     │────▶│   feedback    │     │  activity_log     │
│              │     │              │     │                   │
│ id (PK)      │     │ id (PK)      │     │ id (PK)           │
│ title        │     │ memory_id(FK)│     │ user_id           │
│ content      │     │ user_id      │     │ action            │
│ summary      │     │ rating       │     │ entity_type       │
│ memory_type  │     │ context      │     │ entity_id         │
│ source_type  │     │ created_at   │     │ metadata (jsonb)  │
│ language     │     └──────────────┘     │ created_at        │
│ embedding    │                          └───────────────────┘
│ fts_vector   │     ┌──────────────┐
│ version_meta │────▶│memory_lineage│     ┌───────────────────┐
│ provenance   │     │              │     │    secrets         │
│ decay_score  │     │ id (PK)      │     │                   │
│ reinf_score  │     │ memory_id(FK)│     │ id (PK)           │
│ deleted_at   │     │ parent_id(FK)│     │ name (unique)     │
│ created_at   │     │ action       │     │ ciphertext        │
│ updated_at   │     │ rationale    │     │ iv                │
└──────┬───────┘     │ created_at   │     │ created_by        │
       │             └──────────────┘     │ created_at        │
       │                                  │ updated_at        │
       │             ┌──────────────────┐ └───────────────────┘
       └────────────▶│contradiction_    │
                     │     pairs        │ ┌───────────────────┐
                     │                  │ │    cli_keys        │
                     │ id (PK)          │ │                   │
                     │ memory_a_id (FK) │ │ id (PK)           │
                     │ memory_b_id (FK) │ │ user_id (FK)      │
                     │ resolution_status│ │ public_key        │
                     │ tiebreaker_votes │ │ label             │
                     │ detected_at      │ │ last_used_at      │
                     │ resolved_at      │ │ created_at        │
                     └──────────────────┘ └───────────────────┘
```

## Table Definitions

### memory

The core entity. Stores knowledge artifacts with embeddings, metadata, and scoring.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| title | text | NOT NULL | — | Short descriptive title |
| content | text | NOT NULL | — | Full memory content |
| summary | text | NOT NULL | — | One-sentence summary |
| memory_type | text | NOT NULL, CHECK | — | Enum: gotcha, best_practice, correction, anti_pattern, discovery |
| source_type | text | NOT NULL, CHECK | — | Enum: correction, observation, pr_feedback, manual, harvested |
| language | text | — | NULL | Primary language: compact, typescript, rust, etc. |
| embedding | vector(1024) | — | NULL | voyage-code-3 embedding (nullable for pending_embedding) |
| fts_vector | tsvector | — | auto-generated | Weighted: title=A, summary=B, content=C |
| compact_pragma | text | — | NULL | Compact pragma version (e.g., "^0.15") |
| compact_compiler | text | — | NULL | Compact compiler version (e.g., "0.15.2") |
| midnight_js | text | — | NULL | midnight-js SDK version |
| indexer_version | text | — | NULL | Midnight indexer version |
| node_version | text | — | NULL | Midnight node version |
| source_url | text | — | NULL | Origin URL (PR, issue, etc.) |
| repo_url | text | — | NULL | GitHub repo URL |
| task_summary | text | — | NULL | What the agent was working on |
| session_id | text | — | NULL | Claude Code session ID |
| decay_score | float8 | NOT NULL | 1.0 | Current decay score (0.0 = fully decayed, 1.0 = fresh) |
| reinforcement_score | float8 | NOT NULL | 0 | Cumulative reinforcement from access + feedback |
| last_accessed_at | timestamptz | — | NULL | Last time this memory was returned in search results |
| embedding_status | text | NOT NULL | 'complete' | Enum: complete, pending, failed |
| pipeline_status | text | NOT NULL | 'complete' | Enum: complete, incomplete |
| deleted_at | timestamptz | — | NULL | Soft-delete timestamp (NULL = active) |
| created_at | timestamptz | NOT NULL | now() | Creation timestamp |
| updated_at | timestamptz | NOT NULL | now() | Last update timestamp |
| created_by | uuid | NOT NULL, FK | — | User who created this memory |

**Indexes**:
- HNSW on `embedding` (for vector similarity, lists=100, probes=10)
- GIN on `fts_vector` (for full-text search)
- btree on `memory_type`
- btree on `created_by`
- btree on `deleted_at` (partial index WHERE deleted_at IS NULL)
- btree on `decay_score` (for threshold queries)

### feedback

Agent and human ratings on memories.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| memory_id | uuid | NOT NULL, FK → memory(id) | — | Memory being rated |
| user_id | uuid | NOT NULL | — | Who submitted the feedback |
| rating | text | NOT NULL, CHECK | — | Enum: helpful, outdated, damaging |
| context | text | — | NULL | Optional context for the rating |
| created_at | timestamptz | NOT NULL | now() | Feedback timestamp |

**Indexes**: btree on `memory_id`, btree on `user_id`

### activity_log

System-wide event log for auditing and real-time activity streaming.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| user_id | uuid | — | NULL | Who performed the action (NULL for system actions) |
| action | text | NOT NULL | — | Action type: create, update, delete, search, feedback, secret_access, auth, etc. |
| entity_type | text | NOT NULL | — | What was acted on: memory, feedback, secret, cli_key, etc. |
| entity_id | uuid | — | NULL | ID of the entity (NULL for search actions) |
| metadata | jsonb | — | '{}' | Additional context (search query, feedback rating, etc.) |
| created_at | timestamptz | NOT NULL | now() | Event timestamp |

**Indexes**: btree on `created_at` (for retention policy and activity streaming)
**Retention**: 90 days, enforced by pg_cron scheduled cleanup

### secrets

Server-side encrypted API keys and credentials.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| name | text | NOT NULL, UNIQUE | — | Secret name (e.g., "VOYAGEAI_API_KEY") |
| ciphertext | bytea | NOT NULL | — | AES-256-GCM encrypted value |
| iv | bytea | NOT NULL | — | Initialization vector for AES-256-GCM |
| created_by | uuid | NOT NULL | — | Admin who created the secret |
| created_at | timestamptz | NOT NULL | now() | Creation timestamp |
| updated_at | timestamptz | NOT NULL | now() | Last update timestamp |

**Indexes**: unique on `name`

### cli_keys

CLI public keys registered by users for challenge-response auth.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| user_id | uuid | NOT NULL, FK → auth.users(id) | — | Key owner |
| public_key | text | NOT NULL, UNIQUE | — | Ed25519 public key (base64 encoded) |
| label | text | — | NULL | User-assigned label (e.g., "laptop", "ci-server") |
| last_used_at | timestamptz | — | NULL | Last successful authentication |
| created_at | timestamptz | NOT NULL | now() | Registration timestamp |

**Indexes**: unique on `public_key`, btree on `user_id`

### memory_lineage

Provenance chain tracking replacements, merges, and updates.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| memory_id | uuid | NOT NULL, FK → memory(id) | — | The memory this lineage belongs to |
| parent_id | uuid | FK → memory(id) | NULL | The parent memory (for replace/update) |
| action | text | NOT NULL, CHECK | — | Enum: replace, update, merge, feedback, create |
| rationale | text | — | NULL | Why this action occurred (from Claude dedup or feedback) |
| metadata | jsonb | — | '{}' | Additional lineage context |
| created_at | timestamptz | NOT NULL | now() | When this lineage event occurred |

**Indexes**: btree on `memory_id`, btree on `parent_id`

### contradiction_pairs

Flagged memory contradictions with resolution tracking.

| Column | Type | Constraints | Default | Description |
|--------|------|-------------|---------|-------------|
| id | uuid | PK | gen_random_uuid() | Unique identifier |
| memory_a_id | uuid | NOT NULL, FK → memory(id) | — | First memory in the pair |
| memory_b_id | uuid | NOT NULL, FK → memory(id) | — | Second memory in the pair |
| resolution_status | text | NOT NULL, CHECK | 'open' | Enum: open, resolved, dismissed |
| tiebreaker_votes | jsonb | NOT NULL | '[]' | Array of {user_id, voted_for, context, created_at} |
| detected_at | timestamptz | NOT NULL | now() | When the contradiction was first detected |
| resolved_at | timestamptz | — | NULL | When the contradiction was resolved |

**Indexes**: unique on `(memory_a_id, memory_b_id)` (prevent duplicate pairs), btree on `resolution_status`

## Enums

| Enum | Values | Used In |
|------|--------|---------|
| memory_type | gotcha, best_practice, correction, anti_pattern, discovery | memory.memory_type |
| source_type | correction, observation, pr_feedback, manual, harvested | memory.source_type |
| feedback_rating | helpful, outdated, damaging | feedback.rating |
| lineage_action | replace, update, merge, feedback, create | memory_lineage.action |
| resolution_status | open, resolved, dismissed | contradiction_pairs.resolution_status |
| embedding_status | complete, pending, failed | memory.embedding_status |
| pipeline_status | complete, incomplete | memory.pipeline_status |

## RLS Policy Summary

All tables use deny-by-default RLS. Access requires a valid JWT with `auth.uid()`.

| Table | SELECT | INSERT | UPDATE | DELETE |
|-------|--------|--------|--------|--------|
| memory | auth.uid() IS NOT NULL AND deleted_at IS NULL | auth.uid() IS NOT NULL | created_by = auth.uid() | created_by = auth.uid() (soft-delete only) |
| feedback | auth.uid() IS NOT NULL | auth.uid() IS NOT NULL | DENY | DENY |
| activity_log | auth.uid() IS NOT NULL | service_role only (edge functions) | DENY | service_role only (cron cleanup) |
| secrets | DENY (edge functions use service_role) | service_role only | service_role only | service_role only |
| cli_keys | user_id = auth.uid() | auth.uid() IS NOT NULL | user_id = auth.uid() | user_id = auth.uid() |
| memory_lineage | auth.uid() IS NOT NULL | service_role only | DENY | DENY |
| contradiction_pairs | auth.uid() IS NOT NULL | auth.uid() IS NOT NULL | auth.uid() IS NOT NULL | DENY |

## Key Relationships

- **memory → feedback**: One-to-many. A memory can have many feedback ratings.
- **memory → memory_lineage**: One-to-many. A memory can have many lineage events.
- **memory → contradiction_pairs**: Many-to-many. A memory can be in multiple contradiction pairs.
- **auth.users → cli_keys**: One-to-many. A user can have many registered CLI keys.
- **auth.users → memory**: One-to-many (via created_by). A user can create many memories.

## Postgres RPC Functions

### hybrid_search(query_text, query_embedding, search_type, limit, version_filters)

Combines full-text and vector search using Reciprocal Rank Fusion.

**Parameters**:
- `query_text` (text) — for FTS
- `query_embedding` (vector(1024)) — for vector similarity
- `search_type` (text, default 'hybrid') — 'hybrid', 'fts', 'vector'
- `result_limit` (int, default 20) — max results
- `version_filters` (jsonb, default '{}') — key-value version predicates

**Returns**: table of (memory_id, title, summary, content, memory_type, language, version metadata, decay_score, reinforcement_score, rrf_score, rank)

**Algorithm** (hybrid mode):
1. Run FTS: `ts_rank(fts_vector, plainto_tsquery(query_text))` → ranked list
2. Run vector: `embedding <=> query_embedding` (cosine distance) → ranked list
3. RRF fusion: `score = 1/(60 + rank_fts) + 1/(60 + rank_vector)`
4. Apply version filters as WHERE clauses
5. Filter: `deleted_at IS NULL AND decay_score > threshold`
6. Order by RRF score descending
7. Limit to result_limit
