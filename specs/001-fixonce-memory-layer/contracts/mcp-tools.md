# MCP Tool Contracts: fixonce-memory-layer

**Spec**: 001-fixonce-memory-layer
**Created**: 2026-03-04
**Tools**: 7

All tools follow the MCP tool protocol. Input validation occurs at the MCP
boundary before calling the service layer. Errors use structured format per
Constitution Principle V.

---

## Error Response Format

All tools return errors in this structured format:

```json
{
  "error": {
    "stage": "validation | quality_gate | duplicate_detection | search | rewrite | rerank | embedding | storage",
    "reason": "Human-readable description of what went wrong",
    "suggestion": "Actionable next step to resolve the issue"
  }
}
```

---

## 1. `fixonce_create_memory`

Submit a new memory. AI-created memories pass through quality gate and duplicate
detection. Human-created memories are stored immediately.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `title` | `string` | yes | — | Non-empty, max 500 chars |
| `content` | `string` | yes | — | Non-empty, max 50KB |
| `summary` | `string` | yes | — | Non-empty, max 1000 chars |
| `memory_type` | `string` | yes | — | `"guidance"` or `"anti_pattern"` |
| `source_type` | `string` | yes | — | `"correction"`, `"discovery"`, or `"instruction"` |
| `created_by` | `string` | yes | — | `"ai"` or `"human"` |
| `language` | `string` | yes | — | Non-empty |
| `tags` | `string[]` | no | `[]` | Max 20 elements, each max 100 chars |
| `source_url` | `string` | no | `null` | Valid URL or null |
| `version_predicates` | `object` | no | `null` | Keys from 12 allowed components, values are string arrays |
| `project_name` | `string` | no | `null` | |
| `project_repo_url` | `string` | no | `null` | |
| `project_workspace_path` | `string` | no | `null` | |
| `confidence` | `number` | no | `0.5` | 0.0 to 1.0 |

### Output Schema

**Success** (created_by: human — direct store):

```json
{
  "status": "created",
  "memory": { "id": "uuid", "title": "...", "created_at": "..." }
}
```

**Success** (created_by: ai — passed quality gate + dedup):

```json
{
  "status": "created | replaced | updated | merged",
  "memory": { "id": "uuid", "title": "...", "created_at": "..." },
  "dedup_outcome": "new | discard | replace | update | merge",
  "affected_memory_ids": ["uuid", "..."]
}
```

**Rejected** (quality gate):

```json
{
  "status": "rejected",
  "reason": "Memory is too vague to be actionable"
}
```

**Discarded** (duplicate):

```json
{
  "status": "discarded",
  "reason": "Duplicate of existing memory",
  "existing_memory_id": "uuid"
}
```

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "title is required" | "Provide a non-empty title" |
| `quality_gate` | "OpenRouter API call failed" | "Check OPENROUTER_API_KEY and connectivity" |
| `duplicate_detection` | "Embedding generation timed out" | "Check VOYAGE_API_KEY and connectivity" |
| `storage` | "Supabase insert failed" | "Check SUPABASE_URL and SUPABASE_SERVICE_KEY" |

---

## 2. `fixonce_query`

Query memories with a configurable retrieval pipeline. Supports simple filtered
lists (no LLM) through full hybrid search with rewriting and reranking.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `query` | `string` | yes | — | Non-empty |
| `rewrite` | `boolean` | no | `true` | LLM query rewriting toggle |
| `type` | `string` | no | `"hybrid"` | `"simple"`, `"vector"`, or `"hybrid"` |
| `rerank` | `boolean` | no | `true` | LLM reranking toggle |
| `tags` | `string[]` | no | `null` | Filter by tags |
| `language` | `string` | no | `null` | Filter by language |
| `project_name` | `string` | no | `null` | Filter by project |
| `memory_type` | `string` | no | `null` | `"guidance"` or `"anti_pattern"` |
| `created_after` | `string` | no | `null` | ISO 8601 datetime |
| `updated_after` | `string` | no | `null` | ISO 8601 datetime |
| `max_results` | `integer` | no | `5` | 1 to 50 |
| `max_tokens` | `integer` | no | `null` | Approximate token budget; overrides max_results |
| `verbosity` | `string` | no | `"small"` | `"small"`, `"medium"`, or `"large"` |
| `version_predicates` | `object` | no | `null` | Environment versions to filter against |

### Output Schema

```json
{
  "results": [
    {
      "id": "uuid",
      "title": "...",
      "content": "...",
      "summary": "...",
      "memory_type": "guidance",
      "relevancy_score": 0.92
      // additional fields per verbosity level
    }
  ],
  "overflow": [
    {
      "id": "uuid",
      "title": "...",
      "summary": "...",
      "relevancy_score": 0.71,
      "cache_key": "ck_abc123"
    }
  ],
  "total_found": 25,
  "pipeline": {
    "rewrite_used": true,
    "search_type": "hybrid",
    "rerank_used": true
  }
}
```

**Verbosity field sets:**

- `small`: id, title, content, summary, memory_type, relevancy_score
- `medium`: small + tags, language, version_predicates, created_by, source_type,
  created_at, updated_at
- `large`: all fields including source_url, project_name, project_repo_url,
  project_workspace_path, confidence, surfaced_count, last_surfaced_at, feedback
  summary (counts by tag, any flagged actions)

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "query is required" | "Provide a non-empty query string" |
| `rewrite` | "Query rewriting failed: OpenRouter timeout" | "Retry or set rewrite=false to skip" |
| `search` | "Supabase query failed" | "Check SUPABASE_URL and SUPABASE_SERVICE_KEY" |
| `rerank` | "Reranking failed: model returned invalid JSON" | "Retry or set rerank=false to skip" |

---

## 3. `fixonce_expand`

Expand a cache key from a query overflow result to full memory content.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `cache_key` | `string` | yes | — | Non-empty, valid cache key format |
| `verbosity` | `string` | no | `"small"` | `"small"`, `"medium"`, or `"large"` |

### Output Schema

```json
{
  "memory": {
    "id": "uuid",
    "title": "...",
    "content": "...",
    "summary": "...",
    "memory_type": "guidance"
    // additional fields per verbosity level
  }
}
```

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "cache_key is required" | "Provide a cache key from a query overflow result" |
| `storage` | "Cache key expired or not found" | "Re-run the query to get fresh cache keys" |

---

## 4. `fixonce_get_memory`

Get a specific memory by ID.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `id` | `string` | yes | — | Valid UUID |
| `verbosity` | `string` | no | `"large"` | `"small"`, `"medium"`, or `"large"` |

### Output Schema

```json
{
  "memory": {
    "id": "uuid",
    "title": "...",
    "content": "...",
    // all fields per verbosity level
  }
}
```

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "id must be a valid UUID" | "Provide a valid UUID" |
| `storage` | "Memory not found" | "Check the memory ID is correct" |

---

## 5. `fixonce_update_memory`

Update an existing memory. Regenerates embedding if content changes.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `id` | `string` | yes | — | Valid UUID |
| `title` | `string` | no | — | Non-empty if provided, max 500 chars |
| `content` | `string` | no | — | Non-empty if provided, max 50KB |
| `summary` | `string` | no | — | Non-empty if provided, max 1000 chars |
| `memory_type` | `string` | no | — | `"guidance"` or `"anti_pattern"` |
| `source_type` | `string` | no | — | `"correction"`, `"discovery"`, or `"instruction"` |
| `source_url` | `string` | no | — | Valid URL or null |
| `tags` | `string[]` | no | — | Max 20 elements, each max 100 chars |
| `language` | `string` | no | — | Non-empty if provided |
| `version_predicates` | `object` | no | — | Keys from 12 allowed components, values are string arrays |
| `project_name` | `string` | no | — | |
| `project_repo_url` | `string` | no | — | |
| `project_workspace_path` | `string` | no | — | |
| `confidence` | `number` | no | — | 0.0 to 1.0 |
| `enabled` | `boolean` | no | — | |

At least one field besides `id` must be provided.

### Output Schema

```json
{
  "memory": {
    "id": "uuid",
    "title": "...",
    "updated_at": "...",
    "embedding_regenerating": true
  }
}
```

`embedding_regenerating` is `true` when content or summary changed, indicating
the embedding will be regenerated asynchronously.

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "At least one field to update is required" | "Provide at least one field to update" |
| `validation` | "id must be a valid UUID" | "Provide a valid UUID" |
| `storage` | "Memory not found" | "Check the memory ID is correct" |

---

## 6. `fixonce_feedback`

Provide feedback on a memory. Multiple feedback entries accumulate per memory.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `memory_id` | `string` | yes | — | Valid UUID |
| `text` | `string` | no | `null` | Free-text feedback |
| `tags` | `string[]` | no | `[]` | Values from: `helpful`, `not_helpful`, `damaging`, `accurate`, `somewhat_accurate`, `somewhat_inaccurate`, `inaccurate`, `outdated` |
| `suggested_action` | `string` | no | `null` | `"keep"`, `"remove"`, or `"fix"` |

At least one of `text`, `tags`, or `suggested_action` must be provided.

### Output Schema

```json
{
  "feedback": {
    "id": "uuid",
    "memory_id": "uuid",
    "created_at": "..."
  },
  "memory_flagged": true
}
```

`memory_flagged` is `true` when `suggested_action` is `"remove"` or `"fix"`,
indicating the memory is now flagged for human review in the Web UI.

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "memory_id is required" | "Provide the UUID of the memory to give feedback on" |
| `validation` | "At least one of text, tags, or suggested_action is required" | "Provide feedback content" |
| `validation` | "Invalid feedback tag: 'wrong_value'" | "Use one of: helpful, not_helpful, damaging, accurate, somewhat_accurate, somewhat_inaccurate, inaccurate, outdated" |
| `storage` | "Memory not found" | "Check the memory ID is correct" |

---

## 7. `fixonce_detect_environment`

Scan project files for Midnight SDK and toolchain versions. Returns a version
map matching the `version_predicates` key format.

### Input Schema

| Parameter | Type | Required | Default | Constraints |
|-----------|------|----------|---------|-------------|
| `project_path` | `string` | no | Current working directory | Valid filesystem path |

### Output Schema

```json
{
  "detected_versions": {
    "compact_compiler": "0.28.0",
    "compact_runtime": "0.14.0",
    "midnight_js": "3.0.0",
    "network": "preprod"
  },
  "scan_sources": {
    "compact_compiler": "package.json (devDependencies)",
    "compact_runtime": "package.json (dependencies)",
    "midnight_js": "package.json (dependencies)",
    "network": ".midnight/config.json"
  },
  "undetected_components": [
    "node", "compact_js", "onchain_runtime", "ledger",
    "wallet_sdk", "dapp_connector_api", "midnight_indexer", "proof_server"
  ]
}
```

### Errors

| Stage | Example Reason | Suggestion |
|-------|---------------|------------|
| `validation` | "project_path does not exist" | "Provide a valid directory path" |
| `storage` | "No package.json found in project" | "Run from a project root with package.json" |
