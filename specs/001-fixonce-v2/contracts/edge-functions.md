# Edge Function API Contracts

All edge functions require a valid JWT in the `Authorization: Bearer <token>` header unless noted otherwise.
All responses use `Content-Type: application/json`.
All errors follow the structured format: `{ "error": { "type": string, "message": string, "action": string } }`.

## Memory Operations

### POST /memory-create

Create a new memory.

**Request**:
```json
{
  "title": "string (required)",
  "content": "string (required)",
  "summary": "string (required)",
  "memory_type": "gotcha | best_practice | correction | anti_pattern | discovery (required)",
  "source_type": "correction | observation | pr_feedback | manual | harvested (required)",
  "language": "string (optional)",
  "embedding": "number[1024] (required)",
  "compact_pragma": "string (optional)",
  "compact_compiler": "string (optional)",
  "midnight_js": "string (optional)",
  "indexer_version": "string (optional)",
  "node_version": "string (optional)",
  "source_url": "string (optional)",
  "repo_url": "string (optional)",
  "task_summary": "string (optional)",
  "session_id": "string (optional)",
  "embedding_status": "complete | pending | failed (default: complete)",
  "pipeline_status": "complete | incomplete (default: complete)"
}
```

**Response** (201):
```json
{
  "id": "uuid",
  "created_at": "ISO 8601 timestamp"
}
```

### GET /memory-get?id={uuid}

Retrieve a memory by ID. Does NOT return the embedding vector by default.

**Query Parameters**:
- `id` (required) — memory UUID
- `include_embedding` (optional, default false) — include raw embedding vector

**Response** (200): Full memory object (all fields except embedding unless requested).

### POST /memory-update

Update an existing memory.

**Request**:
```json
{
  "id": "uuid (required)",
  "title": "string (optional)",
  "content": "string (optional)",
  "summary": "string (optional)",
  "embedding": "number[1024] (optional — required if content changed)",
  "...any other mutable fields"
}
```

**Response** (200): `{ "id": "uuid", "updated_at": "timestamp" }`

### POST /memory-delete

Soft-delete a memory.

**Request**: `{ "id": "uuid (required)" }`
**Response** (200): `{ "id": "uuid", "deleted_at": "timestamp" }`

### POST /memory-search

Search memories with configurable search type.

**Request**:
```json
{
  "query_text": "string (required for fts/hybrid)",
  "query_embedding": "number[1024] (required for vector/hybrid)",
  "search_type": "hybrid | fts | vector (default: hybrid)",
  "limit": "integer (default: 20, max: 100)",
  "version_filters": {
    "compact_compiler": ">= 0.15",
    "midnight_js": "^0.8"
  },
  "memory_type": "string (optional filter)",
  "language": "string (optional filter)"
}
```

**Response** (200):
```json
{
  "results": [
    {
      "id": "uuid",
      "title": "string",
      "summary": "string",
      "content": "string",
      "memory_type": "string",
      "language": "string",
      "decay_score": 0.85,
      "reinforcement_score": 12,
      "rrf_score": 0.032,
      "compact_pragma": "^0.15",
      "compact_compiler": "0.15.2",
      "source_url": "string or null",
      "created_at": "timestamp",
      "updated_at": "timestamp"
    }
  ],
  "total": 42,
  "search_type": "hybrid"
}
```

## Feedback

### POST /feedback-submit

Submit feedback on a memory.

**Request**:
```json
{
  "memory_id": "uuid (required)",
  "rating": "helpful | outdated | damaging (required)",
  "context": "string (optional)"
}
```

**Response** (201): `{ "id": "uuid", "memory_id": "uuid", "rating": "string" }`

**Side Effects**:
- `helpful` → increases reinforcement_score, slows decay
- `outdated` → accelerates decay
- `damaging` → sharply accelerates decay, flags for review

## Secrets

### GET /secret-get?name={name}

Retrieve a decrypted secret. Requires authenticated user.

**Query Parameters**: `name` (required) — secret name (e.g., "VOYAGEAI_API_KEY")

**Response** (200): `{ "name": "string", "value": "string (decrypted plaintext)" }`

**Side Effects**: Logs access to activity_log (secret name, user_id — never the value).

### POST /secret-create

Create a new encrypted secret. Admin only.

**Request**: `{ "name": "string (required)", "value": "string (required, will be encrypted)" }`
**Response** (201): `{ "name": "string", "created_at": "timestamp" }`

### POST /secret-rotate-master

Re-encrypt all secrets with a new master key. Admin only.

**Request**: `{ "new_master_key": "string (required)" }`
**Response** (200): `{ "rotated_count": integer, "completed_at": "timestamp" }`

## Authentication

### POST /auth-nonce

Generate a nonce for challenge-response auth. No auth required.

**Request**: `{ "public_key": "string (base64 Ed25519 public key)" }`
**Response** (200): `{ "nonce": "string", "expires_at": "timestamp (5 min)" }`

### POST /auth-verify

Verify challenge-response signature and issue JWT.

**Request**:
```json
{
  "public_key": "string (base64)",
  "nonce": "string",
  "signature": "string (base64 Ed25519 signature of nonce)"
}
```

**Response** (200): `{ "access_token": "JWT string", "expires_at": "timestamp (8hr)" }`

### POST /auth-org-check

Check GitHub org membership. Called internally by auth middleware.

**Request**: `{ "github_access_token": "string" }`
**Response** (200): `{ "is_member": boolean, "org": "string", "cached_until": "timestamp" }`

## Key Management

### POST /keys-register

Register a CLI public key.

**Request**: `{ "public_key": "string (base64)", "label": "string (optional)" }`
**Response** (201): `{ "id": "uuid", "created_at": "timestamp" }`

### GET /keys-list

List all CLI keys for the authenticated user.

**Response** (200):
```json
{
  "keys": [
    { "id": "uuid", "label": "string", "public_key": "string (truncated)", "last_used_at": "timestamp", "created_at": "timestamp" }
  ]
}
```

### POST /keys-revoke

Revoke a CLI key.

**Request**: `{ "key_id": "uuid (required)" }`
**Response** (200): `{ "revoked": true }`

## Activity

### GET /activity-stream?since={timestamp}&limit={int}

Retrieve recent activity log entries. Used by TUI for activity feed.

**Query Parameters**:
- `since` (optional) — return entries after this timestamp
- `limit` (optional, default 50) — max entries

**Response** (200):
```json
{
  "events": [
    { "id": "uuid", "action": "string", "entity_type": "string", "entity_id": "uuid", "metadata": {}, "created_at": "timestamp" }
  ]
}
```
