# FixOnce

Persistent memory for Claude Code agents.

FixOnce captures lessons learned during coding sessions — gotchas, best
practices, corrections, and anti-patterns — and surfaces them automatically
the next time a similar situation arises. Knowledge accumulates across
sessions; Claude Code stops repeating the same mistakes.

Memories are alive: they decay over time, get reinforced by positive feedback,
compete through contradiction courts, and self-correct through deduplication.
A full RAG pipeline with hybrid search, Claude-powered reranking, and
version-aware filtering ensures the right knowledge surfaces at the right time.

---

## Installation

### Prerequisites

- Rust 1.82+ — install via [rustup](https://rustup.rs/)
- A running FixOnce backend — see [`supabase/README.md`](supabase/README.md) for setup

### From source

```bash
git clone https://github.com/aaronbassett/fixonce.git
cd fixonce
cargo install --path crates/fixonce-cli
```

### Verify

```bash
fixonce --version
```

### Configuration

Set the backend URL as an environment variable (add to your shell profile):

```bash
export FIXONCE_API_URL=https://your-project.supabase.co
```

---

## Quick start

### 1. Log in

```bash
# Opens your browser for GitHub OAuth
fixonce login
```

Or for headless/CI environments:

```bash
# Register a signing key first
fixonce keys add

# Then authenticate via challenge-response
fixonce auth
```

### 2. Create a memory

```bash
fixonce create \
  --title "Always use parameterised queries for SQL" \
  --content "Raw string interpolation in SQL queries leads to injection. Use ? or \$1 placeholders and pass values as bound parameters." \
  --summary "SQL injection prevention via parameterised queries." \
  --type best_practice \
  --source manual \
  --language sql
```

The write pipeline automatically checks for leaked credentials, assesses
quality, deduplicates against existing memories, and enriches metadata.

### 3. Search for memories

```bash
# Quick search (default pipeline: rewrite → hybrid search → rerank)
fixonce query "SQL injection prevention"

# Deep search (adds HyDE, multi-query, confidence scoring, coverage check)
fixonce query "SQL injection prevention" --deep

# Filter by version metadata
fixonce query "compact map iteration" --version compact_compiler=0.15

# JSON output for piping to other tools
fixonce query "error handling" --format json
```

### 4. Give feedback

```bash
# This memory helped
fixonce feedback <memory-id> helpful

# This memory is outdated
fixonce feedback <memory-id> outdated

# This memory caused harm
fixonce feedback <memory-id> damaging
```

Feedback directly affects memory scores: `helpful` reinforces, `outdated`
accelerates decay, `damaging` sharply reduces the score.

### 5. Launch the TUI

```bash
fixonce tui
```

An interactive terminal UI for browsing memories, viewing activity, managing
keys, and checking system health. Requires a terminal of at least 80×24.

### 6. Set up Claude Code hooks (recommended)

Make the hook scripts executable:

```bash
chmod +x hooks/*.sh
```

Add to your project's `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse":        [{ "matcher": "", "hooks": [{ "type": "command", "command": "hooks/pre-tool-use.sh" }] }],
    "PostToolUse":       [{ "matcher": "", "hooks": [{ "type": "command", "command": "hooks/post-tool-use.sh" }] }],
    "UserPromptSubmit":  [{ "matcher": "", "hooks": [{ "type": "command", "command": "hooks/user-prompt-submit.sh" }] }],
    "Stop":              [{ "matcher": "", "hooks": [{ "type": "command", "command": "hooks/stop.sh" }] }]
  }
}
```

For session-start, wire it via your shell profile:

```bash
alias claude='hooks/session-start.sh && claude'
```

All hooks enforce a 3-second timeout and always exit 0 — they never block
the agent.

---

## Command reference

### Authentication

| Command | Description |
|---------|-------------|
| `fixonce login` | Log in via GitHub OAuth (opens browser) |
| `fixonce auth` | Authenticate via Ed25519 challenge-response (headless) |
| `fixonce keys add` | Generate and register a new signing key |
| `fixonce keys list` | List all registered signing keys |
| `fixonce keys revoke <key-id>` | Revoke a signing key |

### Memory operations

| Command | Description |
|---------|-------------|
| `fixonce create [flags]` | Create a new memory (runs full write pipeline) |
| `fixonce get <id>` | Retrieve a memory by UUID |
| `fixonce update <id> [flags]` | Update a memory's fields |
| `fixonce delete <id>` | Soft-delete a memory (preserves lineage) |
| `fixonce feedback <id> <rating>` | Rate a memory: `helpful`, `outdated`, or `damaging` |

### Search and analysis

| Command | Description |
|---------|-------------|
| `fixonce query <text>` | Search memories with RAG pipeline |
| `fixonce lineage <id>` | Show a memory's full mutation history |
| `fixonce analyze <session-log>` | Extract memory candidates from a Claude Code transcript |

### Environment

| Command | Description |
|---------|-------------|
| `fixonce detect` | Detect Midnight ecosystem versions in the current project |
| `fixonce context` | Gather full project context (versions, git info, file structure) |

### Utilities

| Command | Description |
|---------|-------------|
| `fixonce config` | Display the active CLI configuration |
| `fixonce tui` | Launch the interactive terminal UI |
| `fixonce hook <event>` | Dispatch a Claude Code lifecycle hook (used by shell scripts) |

### `fixonce create` flags

| Flag | Required | Description |
|------|----------|-------------|
| `--title <TITLE>` | yes | Short descriptive title |
| `--content <CONTENT>` | yes | Full memory content |
| `--summary <SUMMARY>` | yes | One-sentence summary |
| `--type <TYPE>` | yes | `gotcha`, `best_practice`, `correction`, `anti_pattern`, `discovery` |
| `--source <SOURCE>` | yes | `correction`, `observation`, `pr_feedback`, `manual`, `harvested` |
| `--language <LANG>` | no | Programming language (e.g. `rust`, `python`, `compact`) |
| `--source-url <URL>` | no | Link to origin (PR, issue, etc.) |
| `--repo-url <URL>` | no | Repository URL |
| `--format <FORMAT>` | no | Output format: `text` (default), `json`, `toon` |
| `--skip-pipeline` | no | Skip write pipeline (credential check, quality gate, dedup) |

### `fixonce query` flags

| Flag | Description |
|------|-------------|
| `--deep` | Use the deep pipeline (multi-query, HyDE, confidence, coverage) |
| `--version <key=value>` | Filter by version metadata (e.g. `compact_compiler=0.15`) |
| `--format <FORMAT>` | Output format: `text` (default), `json`, `toon` |
| `--limit <N>` | Maximum number of results (default: 20) |

### Output formats

All commands that produce output support `--format`:

- **text** (default) — human-readable, coloured for terminals
- **json** — structured JSON, suitable for piping to `jq` or other tools
- **toon** — token-optimized notation, compact key-value format for LLM consumption

---

## How it works

### Hooks

| Hook | When | What it does |
|------|------|--------------|
| session-start | Session begins | Detects project context, surfaces top 3 critical memories |
| user-prompt-submit | User types a prompt | Injects top 5 relevant memories as context |
| pre-tool-use | Before a tool runs | Warns if proposed action matches an anti-pattern (threshold: 0.7) |
| post-tool-use | After a tool runs | Advises if output matches known issues (threshold: 0.5) |
| stop | Session ends | Surfaces session-end reminders |

### Memory lifecycle

1. **Created** — enters the write pipeline (credential scan, quality gate, dedup, enrichment)
2. **Embedded** — VoyageAI generates a 1024-dimension vector for hybrid search
3. **Surfaced** — the read pipeline finds and ranks relevant memories
4. **Reinforced** — positive feedback increases the score
5. **Decayed** — memories naturally lose relevance (30-day half-life)
6. **Soft-deleted** — when decay drops below 0.1, the memory is retired

### Memory types

| Type | Use for |
|------|---------|
| `gotcha` | Surprising behaviour that trips people up |
| `best_practice` | Recommended approaches |
| `correction` | Fixes for common mistakes |
| `anti_pattern` | Things to explicitly avoid (surfaced as warnings) |
| `discovery` | New findings or insights |

---

## License

MIT
