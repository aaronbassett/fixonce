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
- **CLI** — 10 commands for terminal-based memory management
- **Web UI** — React dashboard for browsing, creating, and managing memories
- **Claude Code hooks** — automatic memory surfacing during coding sessions

## Architecture

```
apps/
  mcp-server/     MCP server (7 tools, stdio transport)
  cli/            CLI (commander, 10 commands)
  web/            Web UI (React 19, Vite, Express 5, SSE)
  hooks/          Claude Code hooks (5 lifecycle hooks)

packages/
  shared/         Types, Zod schemas, enums, errors
  storage/        Supabase client, CRUD, search, embeddings
  pipeline/       Write/read pipelines, LLM client, projections
  activity/       Activity logging, SSE pub-sub stream
```

## Prerequisites

- A [Supabase](https://supabase.com/) project (for database + pgvector)
- A [Voyage AI](https://www.voyageai.com/) API key (for embeddings)
- An [OpenRouter](https://openrouter.ai/) API key (for LLM calls)

## Installation

Install globally from npm:

```bash
npm install -g fixonce
```

Or run directly with npx:

```bash
npx fixonce
```

Individual components are also available:

```bash
npx fixonce         # CLI
npx fixonce-mcp     # MCP server
npx fixonce-web     # Web UI
```

All packages are published under the `@fixonce` scope. The commands above are shorthand wrappers — you can also install the scoped packages directly:

```bash
npm install -g @fixonce/cli
npm install -g @fixonce/mcp-server
npm install -g @fixonce/web
```

## Configuration

FixOnce requires four settings. You can configure them via a settings file or environment variables.

### Settings file (recommended)

Run the config command to create and edit your settings file:

```bash
npx fixonce config
```

This creates `~/.config/fixonce/settings.json` and opens it in your `$EDITOR`. Fill in your API keys:

```json
{
  "supabaseUrl": "https://your-project.supabase.co",
  "supabaseAnonKey": "your-anon-key",
  "voyageApiKey": "your-voyage-api-key",
  "openrouterApiKey": "your-openrouter-api-key"
}
```

### Environment variables

Alternatively, export environment variables in your shell or shell profile (e.g. `~/.zshrc`, `~/.bashrc`):

```bash
export FIXONCE_SUPABASE_URL=https://your-project.supabase.co
export FIXONCE_SUPABASE_ANON_KEY=your-anon-key
export FIXONCE_VOYAGE_API_KEY=your-voyage-api-key
export FIXONCE_OPENROUTER_API_KEY=your-openrouter-api-key
```

Environment variables take priority over the settings file, so you can use them for per-project overrides.

| Variable | Settings key | Description | Where to get it |
|----------|-------------|-------------|-----------------|
| `FIXONCE_SUPABASE_URL` | `supabaseUrl` | Your Supabase project URL | [Supabase dashboard](https://supabase.com/dashboard) → Project Settings → API |
| `FIXONCE_SUPABASE_ANON_KEY` | `supabaseAnonKey` | Your Supabase anonymous key | Same page as above |
| `FIXONCE_VOYAGE_API_KEY` | `voyageApiKey` | Voyage AI API key (for embeddings) | [Voyage AI dashboard](https://dashboard.voyageai.com/organization/api-keys) |
| `FIXONCE_OPENROUTER_API_KEY` | `openrouterApiKey` | OpenRouter API key (for LLM calls) | [OpenRouter settings](https://openrouter.ai/settings/keys) |

When configuring the MCP server, you can also pass these directly via the `env` block in your settings (see [MCP Server](#mcp-server-recommended-for-claude-code) below).

## Database Setup

FixOnce stores memories in a Supabase Postgres database with pgvector. Run the SQL migrations in order against your Supabase project — either via the [SQL editor](https://supabase.com/dashboard) or the Supabase CLI:

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

## Usage

### MCP Server (recommended for Claude Code)

Run the MCP server directly:

```bash
npx fixonce-mcp
```

Or add it to your Claude Code MCP settings (`~/.claude/settings.json` or `.claude/settings.json`):

```json
{
  "mcpServers": {
    "fixonce": {
      "command": "npx",
      "args": ["fixonce-mcp"],
      "env": {
        "FIXONCE_SUPABASE_URL": "https://your-project.supabase.co",
        "FIXONCE_SUPABASE_ANON_KEY": "your-anon-key",
        "FIXONCE_VOYAGE_API_KEY": "your-voyage-api-key",
        "FIXONCE_OPENROUTER_API_KEY": "your-openrouter-api-key"
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
| `fixonce_detect_environment` | Scan project for component versions |

### CLI

```bash
# Create a memory from stdin
echo '{"title":"Use spread for Compact maps","content":"...","summary":"...","memory_type":"gotcha","source_type":"correction","language":"compact"}' | npx fixonce create

# Search memories
npx fixonce query "how to handle Compact map lookups"

# Detect project environment
npx fixonce detect

# Get a specific memory
npx fixonce get <memory-id>

# Configure API keys
npx fixonce config

# All commands support --json for machine-readable output
npx fixonce query --json "compact compiler errors"
```

### Web UI

```bash
npx fixonce-web
```

The dashboard provides:
- Memory search and browsing
- Memory creation and editing
- Feedback submission
- Real-time activity stream (SSE)

### Claude Code Hooks (automatic surfacing)

If you're developing FixOnce locally, copy the example settings to enable automatic memory surfacing during sessions:

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

## Developing

```bash
git clone https://github.com/aaronbassett/fixonce.git
cd fixonce
pnpm install
pnpm build
```

## Project Structure

| Package | npm | Description |
|---------|-----|-------------|
| `@fixonce/shared` | | Types, Zod v4 schemas, enums, version keys, structured errors |
| `@fixonce/storage` | | Supabase client, CRUD operations, hybrid search, Voyage AI embeddings |
| `@fixonce/pipeline` | | Write pipeline (quality gate, dedup), read pipeline (rewrite, search, rerank) |
| `@fixonce/activity` | | Cross-cutting activity logging with SSE pub-sub |
| `@fixonce/cli` | `fixonce` | 10 commands for terminal-based management |
| `@fixonce/mcp-server` | `fixonce-mcp` | MCP server with 7 tools for Claude Code |
| `@fixonce/web` | `fixonce-web` | React 19 + Vite frontend, Express 5 backend |
| `@fixonce/hooks` | | 5 Claude Code lifecycle hooks |

## License

MIT
