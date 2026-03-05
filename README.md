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

### Getting Started (AI)

If you're using an AI coding agent (like Claude Code), you can have it run through the entire setup for you. This works best if you have the [Supabase MCP server](https://github.com/supabase-community/supabase-mcp) configured, which allows the agent to create and configure your database automatically. Without it, the agent will fall back to the manual instructions for database setup.

Copy the prompt below and paste it into your agent to get started.

<details>
<summary>Setup prompt (click to expand)</summary>

```text
I need you to set up the FixOnce project. Walk me through each step and confirm before moving on.

## Step 1: Project setup

Check if we're already in the fixonce project directory (look for a pnpm-workspace.yaml and an apps/ directory). If we are, skip cloning. If not, ask me if I'd like you to clone it from https://github.com/aaronbassett/fixonce.git — and if so, clone it and cd into the project root.

Run `pnpm install` to install dependencies.

## Step 2: Environment file

Run `cp .env.example .env` to create the environment file.

## Step 3: Database setup

Check if you have access to the Supabase MCP server (look for Supabase-related MCP tools).

**If you DO have the Supabase MCP:**
- Ask me: "Would you like me to create a new Supabase project for FixOnce, or use an existing one?"
- If new: create a new project called "fixonce" (or whatever name I give you)
- If existing: ask me which project to use
- Once you have the project, get the project URL and anon key
- Update the `.env` file to set `SUPABASE_URL` and `SUPABASE_ANON_KEY` with the real values
- Run all 9 SQL migration files from `packages/storage/migrations/` in order (001 through 009) against the database using the MCP tools

**If you do NOT have the Supabase MCP:**
- Tell me: "I don't have access to the Supabase MCP server. You'll need to follow the manual 'Getting Started' instructions in the README to set up your database. Create a Supabase project at https://supabase.com, run the 9 SQL migrations in packages/storage/migrations/ via the SQL editor, and update .env with your SUPABASE_URL and SUPABASE_ANON_KEY."
- Wait for me to confirm that the database is set up and .env is updated before continuing.

## Step 4: API keys

Tell me: "You need two more API keys. I'm opening the pages where you can get them. Create or copy a key from each, paste them into your .env file, and let me know when you're done."

Then open these URLs in my browser:
- https://dashboard.voyageai.com/organization/api-keys
- https://openrouter.ai/settings/keys

Remind me that the keys go in `.env` as:
- `VOYAGE_API_KEY` — from Voyage AI
- `OPENROUTER_API_KEY` — from OpenRouter

Wait for me to confirm before continuing.

## Step 5: Build and verify

Run `pnpm build` and then `pnpm typecheck`. Report any errors. If everything passes, say so and move on.

## Step 6: Seed example memories

Ask me: "Would you like me to add a few example memories so you can see FixOnce in action?"

If yes, use the CLI to create 3 example memories by piping JSON to `pnpm --filter fixonce dev create`. Create memories that would be useful for a Midnight Network developer, for example:

1. A "gotcha" about Compact contract syntax
2. A "pattern" about using the Midnight JS SDK
3. A "correction" about a common mistake

## Step 7: Launch the Web UI

Start the web UI with `pnpm --filter @fixonce/web dev` and open my browser to http://localhost:5173 (or whatever port Vite reports).

Tell me: "FixOnce is running! You can browse your memories in the web UI. Check the README for instructions on setting up the MCP server and Claude Code hooks for full integration."
```

</details>

### Getting Started (Manual)

#### 1. Clone and install

```bash
git clone https://github.com/aaronbassett/fixonce.git
cd fixonce
pnpm install
```

#### 2. Set up environment variables

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

#### 3. Set up the database

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

#### 4. Build

```bash
pnpm build
```

#### 5. Verify

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
