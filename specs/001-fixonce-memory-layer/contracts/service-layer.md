# Service Layer Contracts: fixonce-memory-layer

**Spec**: 001-fixonce-memory-layer
**Created**: 2026-03-04

The service layer is the single source of business logic. MCP tools, CLI
commands, and the Web UI HTTP API all delegate to these functions. No consumer
accesses the storage layer directly.

```
  MCP Server ---+
                |
  CLI ----------+--> Service Layer --> Storage Layer --> Supabase
                |
  Web UI API ---+
```

---

## Shared TypeScript Types

These types are defined in a shared `types/` module imported by all consumers.

### Core Entities

```typescript
// Memory entity as stored
interface Memory {
  id: string;                      // UUID
  title: string;
  content: string;
  summary: string;
  memory_type: MemoryType;
  source_type: SourceType;
  created_by: CreatedBy;
  source_url: string | null;
  tags: string[];
  language: string;
  version_predicates: VersionPredicates | null;
  project_name: string | null;
  project_repo_url: string | null;
  project_workspace_path: string | null;
  confidence: number;
  surfaced_count: number;
  last_surfaced_at: string | null; // ISO 8601
  enabled: boolean;
  created_at: string;              // ISO 8601
  updated_at: string;              // ISO 8601
  embedding: number[] | null;      // 1024-dim vector
}

// Feedback entity as stored
interface Feedback {
  id: string;
  memory_id: string;
  text: string | null;
  tags: FeedbackTag[];
  suggested_action: SuggestedAction | null;
  created_at: string;
}

// Activity log entity as stored
interface ActivityLog {
  id: string;
  operation: OperationType;
  memory_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}
```

### Enums

```typescript
type MemoryType = "guidance" | "anti_pattern";

type SourceType = "correction" | "discovery" | "instruction";

type CreatedBy = "ai" | "human" | "human_modified";

type FeedbackTag =
  | "helpful"
  | "not_helpful"
  | "damaging"
  | "accurate"
  | "somewhat_accurate"
  | "somewhat_inaccurate"
  | "inaccurate"
  | "outdated";

type SuggestedAction = "keep" | "remove" | "fix";

type OperationType = "query" | "create" | "update" | "feedback" | "detect";

type SearchType = "simple" | "vector" | "hybrid";

type Verbosity = "small" | "medium" | "large";
```

### Version Predicates

```typescript
// Keys must be from the 12 allowed component keys
type ComponentKey =
  | "network"
  | "node"
  | "compact_compiler"
  | "compact_runtime"
  | "compact_js"
  | "onchain_runtime"
  | "ledger"
  | "wallet_sdk"
  | "midnight_js"
  | "dapp_connector_api"
  | "midnight_indexer"
  | "proof_server";

type VersionPredicates = Partial<Record<ComponentKey, string[]>>;

// Detected environment (single version per component)
type DetectedVersions = Partial<Record<ComponentKey, string>>;
```

### Error Type

```typescript
interface ServiceError {
  stage: string;
  reason: string;
  suggestion: string;
}
```

### Result Shapes

```typescript
// Verbosity-controlled memory projection
type MemorySmall = Pick<Memory,
  "id" | "title" | "content" | "summary" | "memory_type"
> & { relevancy_score: number };

type MemoryMedium = MemorySmall & Pick<Memory,
  "tags" | "language" | "version_predicates" | "created_by" | "source_type" |
  "created_at" | "updated_at"
>;

type MemoryLarge = MemoryMedium & Pick<Memory,
  "source_url" | "project_name" | "project_repo_url" |
  "project_workspace_path" | "confidence" | "surfaced_count" |
  "last_surfaced_at"
> & { feedback_summary: FeedbackSummary };

interface FeedbackSummary {
  total_count: number;
  tag_counts: Partial<Record<FeedbackTag, number>>;
  flagged_actions: SuggestedAction[];
}

// Overflow entry in query results
interface OverflowEntry {
  id: string;
  title: string;
  summary: string;
  relevancy_score: number;
  cache_key: string;
}
```

---

## Service Functions

### `createMemory`

```typescript
interface CreateMemoryInput {
  title: string;
  content: string;
  summary: string;
  memory_type: MemoryType;
  source_type: SourceType;
  created_by: "ai" | "human";
  language: string;
  tags?: string[];
  source_url?: string | null;
  version_predicates?: VersionPredicates | null;
  project_name?: string | null;
  project_repo_url?: string | null;
  project_workspace_path?: string | null;
  confidence?: number;
}

interface CreateMemoryResult {
  status: "created" | "replaced" | "updated" | "merged" | "rejected" | "discarded";
  memory?: Pick<Memory, "id" | "title" | "created_at">;
  dedup_outcome?: "new" | "discard" | "replace" | "update" | "merge";
  affected_memory_ids?: string[];
  reason?: string;                    // For rejected/discarded
  existing_memory_id?: string;        // For discarded (duplicate)
}
```

**Behavior**:
- `created_by: "human"` -> store immediately, trigger async embedding
- `created_by: "ai"` -> quality gate -> duplicate detection -> store/reject/dedup
- Logs `create` operation to ActivityLog

---

### `queryMemories`

```typescript
interface QueryMemoriesInput {
  query: string;
  rewrite?: boolean;                // default: true
  type?: SearchType;                // default: "hybrid"
  rerank?: boolean;                 // default: true
  tags?: string[];
  language?: string;
  project_name?: string;
  memory_type?: MemoryType;
  created_after?: string;           // ISO 8601
  updated_after?: string;           // ISO 8601
  max_results?: number;             // default: 5
  max_tokens?: number;              // overrides max_results
  verbosity?: Verbosity;            // default: "small"
  version_predicates?: DetectedVersions;
}

interface QueryMemoriesResult {
  results: Array<MemorySmall | MemoryMedium | MemoryLarge>;
  overflow: OverflowEntry[];
  total_found: number;
  pipeline: {
    rewrite_used: boolean;
    search_type: SearchType;
    rerank_used: boolean;
  };
}
```

**Behavior**:
- Runs the configurable retrieval pipeline (rewrite -> search -> rerank)
- Updates `surfaced_count` and `last_surfaced_at` for returned memories
- Excludes `enabled = false` memories
- Logs `query` operation to ActivityLog

---

### `expandCacheKey`

```typescript
interface ExpandCacheKeyInput {
  cache_key: string;
  verbosity?: Verbosity;            // default: "small"
}

interface ExpandCacheKeyResult {
  memory: MemorySmall | MemoryMedium | MemoryLarge;
}
```

**Behavior**:
- Looks up memory from cache key
- Returns full content at requested verbosity
- Does NOT update surfaced_count (already counted in original query)

---

### `getMemory`

```typescript
interface GetMemoryInput {
  id: string;
  verbosity?: Verbosity;            // default: "large"
}

interface GetMemoryResult {
  memory: MemorySmall | MemoryMedium | MemoryLarge;
}
```

**Behavior**:
- Direct lookup by ID, returns memory at requested verbosity
- Does NOT filter by `enabled` (get-by-ID should always return if exists)

---

### `updateMemory`

```typescript
interface UpdateMemoryInput {
  id: string;
  title?: string;
  content?: string;
  summary?: string;
  memory_type?: MemoryType;
  source_type?: SourceType;
  source_url?: string | null;
  tags?: string[];
  language?: string;
  version_predicates?: VersionPredicates | null;
  project_name?: string | null;
  project_repo_url?: string | null;
  project_workspace_path?: string | null;
  confidence?: number;
  enabled?: boolean;
}

interface UpdateMemoryResult {
  memory: Pick<Memory, "id" | "title" | "updated_at">;
  embedding_regenerating: boolean;
}
```

**Behavior**:
- Partial update: only provided fields are changed
- If `content` or `summary` changes: clear embedding, trigger async regeneration
- `updated_at` set via trigger
- Logs `update` operation to ActivityLog
- Does NOT change `created_by` (only Web UI HTTP API does that)

---

### `submitFeedback`

```typescript
interface SubmitFeedbackInput {
  memory_id: string;
  text?: string | null;
  tags?: FeedbackTag[];
  suggested_action?: SuggestedAction | null;
}

interface SubmitFeedbackResult {
  feedback: Pick<Feedback, "id" | "memory_id" | "created_at">;
  memory_flagged: boolean;
}
```

**Behavior**:
- Creates a new Feedback row (feedback accumulates, never overwrites)
- `memory_flagged` is true when `suggested_action` is `"remove"` or `"fix"`
- Logs `feedback` operation to ActivityLog

---

### `detectEnvironment`

```typescript
interface DetectEnvironmentInput {
  project_path?: string;            // default: cwd
}

interface DetectEnvironmentResult {
  detected_versions: DetectedVersions;
  scan_sources: Partial<Record<ComponentKey, string>>;
  undetected_components: ComponentKey[];
}
```

**Behavior**:
- Scans `package.json`, lock files, config files for Midnight component versions
- Returns single detected version per component key
- Logs `detect` operation to ActivityLog

---

## Web UI HTTP API

The Web UI backend wraps service layer functions as HTTP endpoints. This is NOT
an external public API — it serves only the Web UI React frontend running on the
same machine.

### Endpoints

| Method | Path | Service Function | Notes |
|--------|------|-----------------|-------|
| `GET` | `/api/memories` | `queryMemories` | Query params map to QueryMemoriesInput |
| `POST` | `/api/memories` | `createMemory` | Body is CreateMemoryInput with `created_by: "human"` |
| `GET` | `/api/memories/:id` | `getMemory` | |
| `PATCH` | `/api/memories/:id` | `updateMemory` | Sets `created_by` to `"human_modified"` |
| `DELETE` | `/api/memories/:id` | (direct storage delete) | Hard delete with cascade |
| `POST` | `/api/memories/:id/feedback` | `submitFeedback` | |
| `GET` | `/api/memories/:id/feedback` | (direct storage query) | List feedback for a memory |
| `GET` | `/api/activity` | (direct storage query) | Activity log with pagination |
| `GET` | `/api/environment` | `detectEnvironment` | |
| `GET` | `/api/expand/:cache_key` | `expandCacheKey` | |

### Web UI-Specific Behavior

- `PATCH /api/memories/:id` always sets `created_by = "human_modified"` before
  calling `updateMemory`. This is the ONLY path that transitions `created_by`.
- `DELETE /api/memories/:id` is a hard delete not exposed via MCP or CLI in v1.
- `GET /api/activity` supports `?since=<ISO8601>` for the real-time activity
  stream (WebSocket/SSE upgrade for live updates).
- Duplicate preview: `POST /api/memories/preview-duplicates` calls the
  similarity search step of the write pipeline to show potential duplicates as
  the user types (debounced, no quality gate).

### Real-Time Activity Stream

The Web UI uses WebSocket or SSE (implementation choice deferred) to stream
activity log updates to the dashboard. The HTTP API provides the initial load
via `GET /api/activity`, and the real-time transport pushes new entries as they
are created by any consumer (MCP, CLI, or Web UI).
