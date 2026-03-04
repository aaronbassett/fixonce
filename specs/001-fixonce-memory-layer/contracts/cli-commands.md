# CLI Command Contracts: fixonce-memory-layer

**Spec**: 001-fixonce-memory-layer
**Created**: 2026-03-04
**Commands**: 9 (8 original + `expand` added per constitution compliance)

All commands support `--json` flag for machine-readable output matching the
corresponding MCP tool response shape. Human-readable output is the default.

---

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Output as JSON (matches MCP tool response shape) |
| `--help` | Show help for the command |

---

## 1. `fixonce create`

Create a new memory. Maps to MCP tool `fixonce_create_memory`.

### Arguments and Flags

| Flag | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `--title` | `string` | yes | — | Memory title (max 500 chars) |
| `--content` | `string` | conditional | — | Memory content. Required unless stdin is piped. |
| `--summary` | `string` | yes | — | One-line summary (max 1000 chars) |
| `--memory-type` | `string` | yes | — | `guidance` or `anti_pattern` |
| `--source-type` | `string` | yes | — | `correction`, `discovery`, or `instruction` |
| `--language` | `string` | yes | — | e.g., `compact`, `typescript` |
| `--tags` | `string` | no | — | Comma-separated tags |
| `--source-url` | `string` | no | — | URL reference |
| `--version` | `string` | no | — | JSON string for version predicates |
| `--project-name` | `string` | no | — | Project name |
| `--project-repo-url` | `string` | no | — | Project repo URL |
| `--confidence` | `number` | no | `0.5` | 0.0 to 1.0 |

### stdin Support

Content can be piped via stdin:

```bash
cat error_context.md | fixonce create \
  --title "Compact sealed field error" \
  --summary "Sealed fields cannot be read in circuits" \
  --memory-type guidance \
  --source-type correction \
  --language compact
```

When stdin is piped and `--content` is also provided, `--content` takes
precedence.

### Human Output

```
Created memory abc123-...
  Title: Compact sealed field error
  Type: guidance
  Status: created
```

---

## 2. `fixonce query`

Query memories. Maps to MCP tool `fixonce_query`.

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<query>` | `string` | yes | — | Search query (positional argument) |
| `--no-rewrite` | `boolean` | no | `false` | Disable LLM query rewriting |
| `--type` | `string` | no | `hybrid` | `simple`, `vector`, or `hybrid` |
| `--no-rerank` | `boolean` | no | `false` | Disable LLM reranking |
| `--tags` | `string` | no | — | Comma-separated tag filter |
| `--language` | `string` | no | — | Language filter |
| `--project-name` | `string` | no | — | Project filter |
| `--memory-type` | `string` | no | — | `guidance` or `anti_pattern` |
| `--created-after` | `string` | no | — | ISO 8601 datetime |
| `--updated-after` | `string` | no | — | ISO 8601 datetime |
| `--max-results` | `integer` | no | `5` | 1 to 50 |
| `--max-tokens` | `integer` | no | — | Token budget (overrides max-results) |
| `--verbosity` | `string` | no | `small` | `small`, `medium`, or `large` |
| `--version` | `string` | no | — | JSON string of version predicates to filter against |

### Simple Mode Shorthand

```bash
fixonce query "sealed field error" --no-rewrite --type simple --no-rerank
```

### Human Output

```
Found 3 results (25 total matches)

1. [guidance] Compact sealed field error (0.92)
   Sealed fields cannot be read directly in circuits...

2. [anti_pattern] Using reveal on sealed fields (0.85)
   Never use reveal() to bypass sealed access...

3. [guidance] Witness function patterns (0.78)
   Use witness functions to access sealed state...

--- Overflow (2 more) ---
  - "Version predicate AND logic" (0.71) [ck_abc123]
  - "Compact type casting" (0.65) [ck_def456]

Use `fixonce expand <cache_key>` to see full content.
```

---

## 3. `fixonce expand`

Expand a cache key to full memory content. Maps to MCP tool `fixonce_expand`.

Added per constitution compliance review (Principle I: 1:1 CLI/MCP mapping).

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<cache_key>` | `string` | yes | — | Cache key from query overflow |
| `--verbosity` | `string` | no | `small` | `small`, `medium`, or `large` |

### Human Output

```
[guidance] Version predicate AND logic

All constrained components must match for a memory to be surfaced.
If a memory has version_predicates for both compact_compiler and network,
the environment must match BOTH...

  Language: compact
  Tags: version-predicates, filtering
  Created: 2026-02-15
```

---

## 4. `fixonce get`

Get a specific memory by ID. Maps to MCP tool `fixonce_get_memory`.

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<id>` | `string` | yes | — | Memory UUID |
| `--verbosity` | `string` | no | `large` | `small`, `medium`, or `large` |

### Human Output

```
[guidance] Compact sealed field error

Sealed fields cannot be read directly in circuits. Use witness functions
to provide sealed values to circuits, then use disclose() to make them
available...

  ID: abc123-def456-...
  Language: compact
  Type: guidance | correction
  Created by: ai
  Tags: compact, sealed, circuits
  Confidence: 0.85
  Surfaced: 12 times (last: 2026-03-01)
  Versions: compact_compiler: [0.28.0, 0.29.0]
  Created: 2026-02-10
  Updated: 2026-02-15
  Feedback: 3 helpful, 1 accurate
```

---

## 5. `fixonce update`

Update an existing memory. Maps to MCP tool `fixonce_update_memory`.

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<id>` | `string` | yes | — | Memory UUID |
| `--title` | `string` | no | — | New title |
| `--content` | `string` | no | — | New content (also accepts stdin) |
| `--summary` | `string` | no | — | New summary |
| `--memory-type` | `string` | no | — | `guidance` or `anti_pattern` |
| `--source-type` | `string` | no | — | `correction`, `discovery`, or `instruction` |
| `--language` | `string` | no | — | New language |
| `--tags` | `string` | no | — | Comma-separated tags (replaces existing) |
| `--source-url` | `string` | no | — | New source URL |
| `--version` | `string` | no | — | JSON string for version predicates |
| `--project-name` | `string` | no | — | Project name |
| `--project-repo-url` | `string` | no | — | Project repo URL |
| `--confidence` | `number` | no | — | 0.0 to 1.0 |
| `--enable` | `boolean` | no | — | Set enabled=true |
| `--disable` | `boolean` | no | — | Set enabled=false |

At least one update flag must be provided.

### stdin Support

```bash
cat updated_content.md | fixonce update abc123-... --title "Updated title"
```

### Human Output

```
Updated memory abc123-...
  Title: Updated title
  Embedding: regenerating (content changed)
```

---

## 6. `fixonce feedback`

Provide feedback on a memory. Maps to MCP tool `fixonce_feedback`.

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<memory_id>` | `string` | yes | — | Memory UUID |
| `--text` | `string` | no | — | Free-text feedback |
| `--tags` | `string` | no | — | Comma-separated: `helpful`, `not_helpful`, `damaging`, `accurate`, `somewhat_accurate`, `somewhat_inaccurate`, `inaccurate`, `outdated` |
| `--action` | `string` | no | — | `keep`, `remove`, or `fix` |

At least one of `--text`, `--tags`, or `--action` must be provided.

### Human Output

```
Feedback recorded for memory abc123-...
  Tags: helpful, accurate
  Action: keep
```

---

## 7. `fixonce detect`

Detect Midnight SDK versions in the current project. Maps to MCP tool
`fixonce_detect_environment`.

### Arguments and Flags

| Argument / Flag | Type | Required | Default | Description |
|-----------------|------|----------|---------|-------------|
| `<path>` | `string` | no | `.` | Project directory to scan |

### Human Output

```
Detected Midnight environment:

  compact_compiler: 0.28.0 (from package.json)
  compact_runtime:  0.14.0 (from package.json)
  midnight_js:      3.0.0  (from package.json)
  network:          preprod (from .midnight/config.json)

Undetected: node, compact_js, onchain_runtime, ledger, wallet_sdk,
            dapp_connector_api, midnight_indexer, proof_server
```

---

## 8. `fixonce serve`

Start the MCP server. No corresponding MCP tool (infrastructure command).

### Arguments and Flags

| Flag | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `--port` | `integer` | no | stdio | Port for HTTP transport (default is stdio for MCP) |

### Notes

- Default transport: stdio (standard MCP protocol)
- Optional HTTP transport for debugging/testing
- Separate process from Web UI

---

## 9. `fixonce web`

Start the Web UI server. No corresponding MCP tool (infrastructure command).

### Arguments and Flags

| Flag | Type | Required | Default | Description |
|------|------|----------|---------|-------------|
| `--port` | `integer` | no | `3000` | Web UI port |
| `--open` | `boolean` | no | `true` | Open browser on start |

### Notes

- Starts a Vite dev server (or serves built assets in production)
- Calls the same service layer as MCP and CLI
- Separate process from MCP server
