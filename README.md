# FixOnce

A shared memory layer for LLM coding agents. FixOnce captures corrections, gotchas, and discoveries from coding sessions and surfaces them contextually — turning every mistake into institutional memory.

## The Problem

LLM coding agents operate in isolated sessions with no persistent memory. Every new session starts from zero, unaware of corrections and lessons from previous sessions. This creates a costly cycle of repeated mistakes, siloed knowledge, and version-sensitive errors that agents can't track.

## How It Works

FixOnce stores memories (corrections, patterns, gotchas) in a Supabase-backed database with vector embeddings. When an agent starts a session or encounters a problem, FixOnce surfaces relevant memories using hybrid search (full-text + semantic). An LLM-powered pipeline handles quality gating, duplicate detection, query rewriting, and result reranking to keep the memory store clean and results relevant.

## Features

- **Hybrid search** — combines full-text search with pgvector semantic similarity using Reciprocal Rank Fusion
- **Write pipeline** — quality gate with credential detection, 5-outcome LLM dedup (new, discard, replace, update, merge)
- **Read pipeline** — query rewriting, hybrid/vector/FTS search, LLM reranking, verbosity projections
- **Version-aware** — filter memories by component version predicates (e.g., "compact_compiler >= 0.14.0")
- **MCP server** — 7 tools for direct Claude Code integration
- **CLI** — 9 commands for terminal-based memory management
- **Web UI** — React dashboard for browsing, creating, and managing memories
- **Claude Code hooks** — automatic memory surfacing during coding sessions

## Architecture

```
apps/
  mcp-server/     MCP server (7 tools, stdio transport)
  cli/            CLI (commander, 9 commands)
  web/            Web UI (React 19, Vite, Express 5, SSE)
  hooks/          Claude Code hooks (5 lifecycle hooks)

packages/
  shared/         Types, Zod schemas, enums, errors
  storage/        Supabase client, CRUD, search, embeddings
  pipeline/       Write/read pipelines, LLM client, projections
  activity/       Activity logging, SSE pub-sub stream
```

## Prerequisites

- [Node.js](https://nodejs.org/) v20+
- [pnpm](https://pnpm.io/) v9+
- A [Supabase](https://supabase.com/) project (for database + pgvector)
- A [Voyage AI](https://www.voyageai.com/) API key (for embeddings)
- An [OpenRouter](https://openrouter.ai/) API key (for LLM calls)

## Getting Started

### 1. Clone and install

```bash
git clone https://github.com/aaronbassett/fixonce.git
cd fixonce
pnpm install
```

### 2. Set up environment variables

```bash
cp .env.example .env
```

Edit `.env` with your credentials:

```
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_ANON_KEY=your-anon-key
VOYAGE_API_KEY=your-voyage-api-key
OPENROUTER_API_KEY=your-openrouter-api-key
```

### 3. Set up the database

Run the SQL migrations in order against your Supabase project. You can paste them into the Supabase SQL editor or use the CLI:

```bash
# Migrations are in packages/storage/migrations/
# Run them in order: 001_extensions.sql through 009_hybrid_search_rpc.sql
```

The migrations create:
- pgvector and uuid-ossp extensions
- `memory`, `feedback`, and `activity_log` tables
- Full-text search with weighted tsvector
- HNSW index for vector similarity
- Hybrid search RPC function (Reciprocal Rank Fusion)

### 4. Build

```bash
pnpm build
```

### 5. Verify

```bash
pnpm typecheck
```

## Usage

### MCP Server (recommended for Claude Code)

Add FixOnce as an MCP server in your Claude Code settings:

```json
{
  "mcpServers": {
    "fixonce": {
      "command": "node",
      "args": ["./apps/mcp-server/dist/index.js"],
      "env": {
        "SUPABASE_URL": "https://your-project.supabase.co",
        "SUPABASE_ANON_KEY": "your-anon-key",
        "VOYAGE_API_KEY": "your-voyage-api-key",
        "OPENROUTER_API_KEY": "your-openrouter-api-key"
      }
    }
  }
}
```

This gives Claude Code access to 7 tools:

| Tool | Description |
|------|-------------|
| `fixonce_create_memory` | Store a new memory (correction, gotcha, pattern) |
| `fixonce_query` | Search memories with hybrid search + reranking |
| `fixonce_expand` | Expand an overflow cache key to full memory |
| `fixonce_get_memory` | Retrieve a specific memory by ID |
| `fixonce_update_memory` | Update an existing memory |
| `fixonce_feedback` | Submit feedback on a memory (helpful, outdated, damaging) |
| `fixonce_detect_environment` | Scan project for Midnight component versions |

### Claude Code Hooks (automatic surfacing)

Copy the example settings to enable automatic memory surfacing during sessions:

```bash
cp apps/hooks/settings.example.json .claude/settings.json
```

This registers 5 hooks:

| Hook | When | What |
|------|------|------|
| `SessionStart` | Session begins | Detects environment, surfaces critical memories |
| `UserPromptSubmit` | User sends a prompt | Quick-searches for relevant memories |
| `PreToolUse` | Before Write/Edit | Blocks if anti-pattern matched (score > 0.7) |
| `PostToolUse` | After Write/Edit | Warns if anti-pattern matched (score > 0.5) |
| `Stop` | Session ends | Surfaces final critical reminders |

### CLI

```bash
# Create a memory from stdin
echo '{"title":"Use spread for Compact maps","content":"...","summary":"...","memory_type":"gotcha","source_type":"correction","language":"compact"}' | pnpm --filter fixonce dev create

# Search memories
pnpm --filter fixonce dev query "how to handle Compact map lookups"

# Detect project environment
pnpm --filter fixonce dev detect

# Get a specific memory
pnpm --filter fixonce dev get <memory-id>

# All commands support --json for machine-readable output
pnpm --filter fixonce dev query --json "compact compiler errors"
```

### Web UI

```bash
# Start the web UI (Express backend + React frontend)
pnpm --filter @fixonce/web dev
```

The dashboard provides:
- Memory search and browsing
- Memory creation and editing
- Feedback submission
- Real-time activity stream (SSE)

## Project Structure

| Package | Description |
|---------|-------------|
| `@fixonce/shared` | Types, Zod v4 schemas, enums, version keys, structured errors |
| `@fixonce/storage` | Supabase client, CRUD operations, hybrid search, Voyage AI embeddings |
| `@fixonce/pipeline` | Write pipeline (quality gate, dedup), read pipeline (rewrite, search, rerank) |
| `@fixonce/activity` | Cross-cutting activity logging with SSE pub-sub |
| `@fixonce/mcp-server` | MCP server with 7 tools for Claude Code |
| `fixonce` (CLI) | 9 commands for terminal-based management |
| `@fixonce/web` | React 19 + Vite frontend, Express 5 backend |
| `@fixonce/hooks` | 5 Claude Code lifecycle hooks |

## License

MIT
