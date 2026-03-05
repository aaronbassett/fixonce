# Data Model: fixonce-memory-layer

**Spec**: 001-fixonce-memory-layer
**Created**: 2026-03-04

---

## Entities

### Memory

The core entity. Stores guidance and anti-patterns with version-scoped metadata,
embeddings for vector search, and full-text search support.

| Field | Type | Constraints | Default | Notes |
|-------|------|-------------|---------|-------|
| `id` | `uuid` | PK, NOT NULL | `gen_random_uuid()` | Auto-generated |
| `title` | `text` | NOT NULL, max 500 chars | — | Short scannable name |
| `content` | `text` | NOT NULL, max 50KB | — | Full memory in markdown |
| `summary` | `text` | NOT NULL, max 1000 chars | — | One-line summary for search results |
| `memory_type` | `memory_type` | NOT NULL | — | `guidance` or `anti_pattern` |
| `source_type` | `source_type` | NOT NULL | — | `correction`, `discovery`, or `instruction` |
| `created_by` | `created_by` | NOT NULL | — | `ai`, `human`, or `human_modified` |
| `source_url` | `text` | NULLABLE | `NULL` | Link to PR comment, CI run, etc. |
| `tags` | `text[]` | max 20 elements, each max 100 chars | `'{}'` | Freeform tags |
| `language` | `text` | NOT NULL | — | e.g., `compact`, `typescript` |
| `version_predicates` | `jsonb` | NULLABLE, keys from allowed set | `NULL` | Version constraints (see section below) |
| `project_name` | `text` | NULLABLE | `NULL` | e.g., `fixonce` |
| `project_repo_url` | `text` | NULLABLE | `NULL` | e.g., `https://github.com/org/repo` |
| `project_workspace_path` | `text` | NULLABLE | `NULL` | Local path context |
| `confidence` | `float` | NOT NULL, range 0.0-1.0 | `0.5` | Confidence score |
| `surfaced_count` | `integer` | NOT NULL, >= 0 | `0` | Times returned in queries |
| `last_surfaced_at` | `timestamptz` | NULLABLE | `NULL` | Last query return |
| `enabled` | `boolean` | NOT NULL | `true` | Kill switch |
| `created_at` | `timestamptz` | NOT NULL | `now()` | Auto-set |
| `updated_at` | `timestamptz` | NOT NULL | `now()` | Auto-updated via trigger |
| `embedding` | `vector(1024)` | NULLABLE | `NULL` | Voyage code-3 embeddings |
| `fts` | `tsvector` | GENERATED ALWAYS | — | Generated from title + content + summary + tags |

#### Relationships

- Has many `Feedback` (cascade delete)
- Has many `ActivityLog` (set null on delete)

#### Indexes

| Name | Target | Type | Purpose |
|------|--------|------|---------|
| `idx_memory_pkey` | `id` | btree (PK) | Primary key lookup |
| `idx_memory_version_predicates` | `version_predicates` | GIN | JSONB `?` operator for version filtering |
| `idx_memory_fts` | `fts` | GIN | Full-text search on tsvector |
| `idx_memory_embedding` | `embedding` | HNSW (cosine) | Vector similarity search |
| `idx_memory_tags` | `tags` | GIN | Array containment queries (`@>`) |
| `idx_memory_enabled` | `enabled` WHERE `enabled = true` | btree (partial) | Filter active memories efficiently |
| `idx_memory_language` | `language` | btree | Filter by language |
| `idx_memory_memory_type` | `memory_type` | btree | Filter by type |

---

### Feedback

Accumulates per-memory feedback from agents and humans.

| Field | Type | Constraints | Default | Notes |
|-------|------|-------------|---------|-------|
| `id` | `uuid` | PK, NOT NULL | `gen_random_uuid()` | Auto-generated |
| `memory_id` | `uuid` | FK -> Memory.id, NOT NULL | — | CASCADE on delete |
| `text` | `text` | NULLABLE | `NULL` | Free-text feedback |
| `tags` | `feedback_tag[]` | DEFAULT '{}' | `'{}'` | Structured feedback tags |
| `suggested_action` | `suggested_action` | NULLABLE | `NULL` | `keep`, `remove`, or `fix` |
| `created_at` | `timestamptz` | NOT NULL | `now()` | Auto-set |

#### Relationships

- Belongs to `Memory` (memory_id FK, CASCADE on delete)

#### Indexes

| Name | Target | Type | Purpose |
|------|--------|------|---------|
| `idx_feedback_pkey` | `id` | btree (PK) | Primary key lookup |
| `idx_feedback_memory_id` | `memory_id` | btree | Lookup feedback by memory |
| `idx_feedback_suggested_action` | `suggested_action` WHERE `suggested_action IN ('remove', 'fix')` | btree (partial) | Flag memories for human review |

---

### ActivityLog

Append-only log of all operations for the Web UI activity stream.

| Field | Type | Constraints | Default | Notes |
|-------|------|-------------|---------|-------|
| `id` | `uuid` | PK, NOT NULL | `gen_random_uuid()` | Auto-generated |
| `operation` | `operation_type` | NOT NULL | — | `query`, `create`, `update`, `feedback`, `detect` |
| `memory_id` | `uuid` | FK -> Memory.id, NULLABLE | `NULL` | SET NULL on delete; not all ops relate to a memory |
| `details` | `jsonb` | NOT NULL | — | Operation-specific payload |
| `created_at` | `timestamptz` | NOT NULL | `now()` | Auto-set |

#### Relationships

- Belongs to `Memory` (memory_id FK, SET NULL on delete — preserves log entries)

#### Indexes

| Name | Target | Type | Purpose |
|------|--------|------|---------|
| `idx_activity_log_pkey` | `id` | btree (PK) | Primary key lookup |
| `idx_activity_log_created_at` | `created_at` | btree DESC | Activity stream ordering |
| `idx_activity_log_memory_id` | `memory_id` | btree | Filter activity by memory |
| `idx_activity_log_operation` | `operation` | btree | Filter by operation type |

---

## Enums

### `memory_type`

| Value | Description |
|-------|-------------|
| `guidance` | Positive guidance (do this). Ranks higher in results. |
| `anti_pattern` | Anti-pattern (do not do this). |

### `source_type`

| Value | Description |
|-------|-------------|
| `correction` | Learned from a mistake or correction |
| `discovery` | Discovered through exploration or research |
| `instruction` | Explicit instruction from a human |

### `created_by`

| Value | Description |
|-------|-------------|
| `ai` | Created by an AI agent |
| `human` | Created by a human (via CLI or Web UI) |
| `human_modified` | Originally AI or human, subsequently edited via Web UI |

### `feedback_tag`

| Value | Description |
|-------|-------------|
| `helpful` | Memory was helpful for the task |
| `not_helpful` | Memory was not useful |
| `damaging` | Memory caused harm or wrong direction |
| `accurate` | Memory content is factually correct |
| `somewhat_accurate` | Memory is partially correct |
| `somewhat_inaccurate` | Memory has notable inaccuracies |
| `inaccurate` | Memory is factually wrong |
| `outdated` | Memory content is no longer current |

### `suggested_action`

| Value | Description |
|-------|-------------|
| `keep` | Memory should be kept as-is |
| `remove` | Memory should be removed (flags for human review) |
| `fix` | Memory needs correction (flags for human review) |

### `operation_type`

| Value | Description |
|-------|-------------|
| `query` | Memory retrieval query |
| `create` | Memory creation |
| `update` | Memory update |
| `feedback` | Feedback submitted |
| `detect` | Environment detection |

### `search_type`

Used as a parameter value, not a database enum.

| Value | Description |
|-------|-------------|
| `simple` | FTS only, no LLM calls |
| `vector` | Vector similarity only |
| `hybrid` | FTS + vector combined (default) |

### `verbosity`

Used as a parameter value, not a database enum.

| Value | Fields Included |
|-------|-----------------|
| `small` | id, title, content, summary, memory_type, relevancy_score |
| `medium` | small + tags, language, version_predicates, created_by, source_type, created_at, updated_at |
| `large` | All fields including source_url, project details, confidence, surfaced_count, feedback summary |

---

## Version Predicates

### JSONB Structure

```json
{
  "compact_compiler": ["0.28.0", "0.29.0"],
  "compact_runtime": ["0.14.0"],
  "network": ["preprod", "preview"],
  "midnight_js": ["3.0.0", "3.1.0"]
}
```

### 12 Allowed Component Keys

| Key | Component |
|-----|-----------|
| `network` | Network environment |
| `node` | Node version |
| `compact_compiler` | Compact compiler |
| `compact_runtime` | Compact runtime |
| `compact_js` | Compact JS bindings |
| `onchain_runtime` | On-chain runtime |
| `ledger` | Ledger |
| `wallet_sdk` | Wallet SDK |
| `midnight_js` | Midnight JS SDK |
| `dapp_connector_api` | DApp Connector API |
| `midnight_indexer` | Midnight Indexer |
| `proof_server` | Proof Server |

### Matching Rules

- **Values**: Arrays of exact version strings (not semver ranges)
- **OR within a component**: Memory matches if the environment version is in the
  array for that component
- **AND across components**: All constrained components must match
- **Missing key**: No constraint on that component (matches any version)
- **`null` value**: Memory applies universally to all versions

### Query Patterns

Filter memories matching a detected environment:

```sql
-- Check if memory applies to compact_compiler 0.28.0
WHERE version_predicates IS NULL
   OR NOT version_predicates ? 'compact_compiler'
   OR version_predicates->'compact_compiler' @> '"0.28.0"'::jsonb;
```

Multi-component filter (AND across):

```sql
WHERE (
    version_predicates IS NULL
    OR (
      (NOT version_predicates ? 'compact_compiler'
       OR version_predicates->'compact_compiler' @> '"0.28.0"'::jsonb)
      AND
      (NOT version_predicates ? 'network'
       OR version_predicates->'network' @> '"preprod"'::jsonb)
    )
);
```

The GIN index on `version_predicates` supports the `?` (has key) operator.

---

## State Transitions

### Memory Lifecycle

```
                  create (enabled=true)
                        |
                        v
  +--------+    +---------------+    +---------------+
  | created | -> | active        | -> | disabled      |
  +--------+    | (enabled=true) |    | (enabled=false)|
                +---------------+    +-------+-------+
                       ^                     |
                       |    re-enable        |
                       +---------------------+
                                             |
                                             v
                                      +----------+
                                      | deleted  |
                                      | (row     |
                                      | removed) |
                                      +----------+
```

- **created -> active**: Immediate on successful creation (human) or after
  passing quality gate + dedup (AI)
- **active -> disabled**: `enabled` set to `false` via update
- **disabled -> active**: `enabled` set to `true` via update
- **disabled -> deleted**: Row deleted (cascades to feedback)
- Disabled memories are excluded from retrieval queries but visible in Web UI

### `created_by` Transitions

```
  ai -------> human_modified (on Web UI edit)
  human ----> human_modified (on Web UI edit)
  human_modified stays human_modified (idempotent)
```

- Set at creation time to `ai` or `human`
- Any edit via the Web UI transitions to `human_modified`
- CLI/MCP updates do NOT change `created_by` (only Web UI does)

### Embedding State

```
  NULL (pending) --> populated (after async generation) --> regenerated (on content update)
```

- On memory creation: `embedding = NULL`, async job generates embedding
- On content update: `embedding = NULL` (cleared), async job regenerates
- Memories with `embedding = NULL` are still FTS-searchable but not
  vector-searchable

---

## Validation Rules

### Memory

| Field | Rule |
|-------|------|
| `content` | Non-empty, max 50KB |
| `title` | Non-empty, max 500 characters |
| `summary` | Non-empty, max 1000 characters |
| `tags` | Max 20 tags, each max 100 characters |
| `language` | Non-empty |
| `confidence` | Float between 0.0 and 1.0 inclusive |
| `version_predicates` | If present, all keys must be from the 12 allowed component keys; values must be arrays of strings |

### Feedback

| Field | Rule |
|-------|------|
| `memory_id` | Must reference existing Memory |
| `tags` | Each value must be a valid `feedback_tag` enum value |
| `suggested_action` | Must be a valid `suggested_action` enum value if present |
| At least one of | `text`, `tags`, or `suggested_action` must be provided |

### ActivityLog

| Field | Rule |
|-------|------|
| `operation` | Must be a valid `operation_type` enum value |
| `details` | Must be valid JSON, non-null |

---

## Supabase-Specific Details

### Full-Text Search (tsvector)

Generated column using `coalesce` to handle nullable array fields:

```sql
ALTER TABLE memory ADD COLUMN fts tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
    setweight(to_tsvector('english', coalesce(content, '')), 'C') ||
    setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'D')
  ) STORED;
```

Weights: title (A) > summary (B) > content (C) > tags (D).

### pgvector

- Column: `vector(1024)` for Voyage AI `voyage-code-3` embeddings
- Index: HNSW with cosine distance operator (`<=>`)

```sql
CREATE INDEX idx_memory_embedding ON memory
  USING hnsw (embedding vector_cosine_ops);
```

### JSONB GIN Index

```sql
CREATE INDEX idx_memory_version_predicates ON memory
  USING gin (version_predicates jsonb_path_ops);
```

### Row-Level Security

The MVP is single-user. RLS is not required for v1, but the schema should not
preclude adding it later. Note: If Supabase anon key is used from the Web UI
frontend, RLS policies will be needed to restrict access. For v1, use the
service key from server-side code only.

### Trigger: `updated_at` Auto-Refresh

```sql
CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER memory_updated_at
  BEFORE UPDATE ON memory
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();
```

### Extensions Required

```sql
CREATE EXTENSION IF NOT EXISTS vector;     -- pgvector
CREATE EXTENSION IF NOT EXISTS "uuid-ossp"; -- uuid generation (or use gen_random_uuid())
```
