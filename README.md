# FixOnce

Persistent memory for Claude Code agents.

FixOnce captures lessons learned during coding sessions — gotchas, best
practices, corrections, and anti-patterns — and surfaces them automatically
the next time a similar situation arises.  Knowledge accumulates across
sessions; Claude Code stops repeating the same mistakes.

---

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│  Claude Code Agent                                                │
│                                                                   │
│  session-start ──► user-prompt-submit ──► pre/post-tool-use ──►  │
│       │                   │                       │               │
│       └───────────────────┴───────────────────────┘               │
│                           │ hook events                           │
└───────────────────────────┼───────────────────────────────────────┘
                            │
                            ▼
┌────────────────────────────────────────┐
│  fixonce-hooks  (Rust)                 │
│  Shell-script adapters → hook binary  │
│  Hard timeout: 3 s  ·  Always exit 0  │
└────────────────────┬───────────────────┘
                     │
                     ▼
┌────────────────────────────────────────┐
│  fixonce-cli  (Rust / Clap + Tokio)    │
│                                        │
│  15 sub-commands (see below)           │
│  TUI (Ratatui)  ·  JSON / text output  │
└────────────────────┬───────────────────┘
                     │
                     ▼
┌────────────────────────────────────────┐
│  fixonce-core  (Rust library crate)    │
│                                        │
│  ┌──────────────┐  ┌────────────────┐  │
│  │  Write       │  │  Read          │  │
│  │  Pipeline    │  │  Pipeline      │  │
│  │              │  │                │  │
│  │ cred-check   │  │ query-techs    │  │
│  │ quality-gate │  │ search-modes   │  │
│  │ dedup        │  │ result-refine  │  │
│  │ enrichment   │  │                │  │
│  └──────────────┘  └────────────────┘  │
│                                        │
│  Memory model  ·  Dynamics  ·          │
│  Lineage  ·  Contradictions  ·         │
│  Signatures  ·  Hot-cache              │
│                                        │
│  Auth (Ed25519 + JWT)                  │
│  Detect (Midnight ecosystem)           │
│  Output (text / JSON / toon)           │
└────────────────────┬───────────────────┘
                     │
                     ▼
┌────────────────────────────────────────┐
│  Supabase Backend                      │
│  PostgreSQL + pgvector                 │
│  Edge Functions (Deno)                 │
│  Voyage AI embeddings                  │
└────────────────────────────────────────┘
```

### Crate layout

| Crate | Purpose |
|-------|---------|
| `fixonce-core` | Pure library: memory model, pipelines, auth, detection, output |
| `fixonce-cli` | Clap CLI binary: 15 commands + Ratatui TUI |
| `fixonce-hooks` | Hook handler logic (called by shell scripts) |

---

## Installation

### Prerequisites

- Rust 1.82+ (`rustup update stable`)
- A Supabase project with the FixOnce schema applied (see `supabase/`)
- `cargo` in `PATH`

### Build and install from source

```bash
git clone https://github.com/aaronbassett/fixonce
cd fixonce
cargo install --path crates/fixonce-cli
```

Verify the installation:

```bash
fixonce --version
```

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FIXONCE_API_URL` | `https://fixonce.supabase.co` | Backend API base URL |

---

## Quick start

### 1. Authenticate

```bash
# Browser-based OAuth (GitHub)
fixonce login

# Machine-to-machine challenge-response (headless environments)
fixonce auth
```

### 2. Create your first memory

```bash
fixonce create \
  --title "Always use parameterised queries for SQL" \
  --content "Raw string interpolation in SQL queries leads to injection. Use ? or $1 placeholders and pass values as bound parameters." \
  --summary "SQL injection prevention via parameterised queries." \
  --type best_practice \
  --source manual \
  --language sql
```

### 3. Query memories during a session

```bash
fixonce query "SQL injection prevention"
```

### 4. Launch the TUI

```bash
fixonce tui
```

### 5. Set up Claude Code hooks (optional but recommended)

```bash
chmod +x hooks/*.sh
```

Add to `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse":        [{"matcher": "", "hooks": [{"type": "command", "command": "hooks/pre-tool-use.sh"}]}],
    "PostToolUse":       [{"matcher": "", "hooks": [{"type": "command", "command": "hooks/post-tool-use.sh"}]}],
    "UserPromptSubmit":  [{"matcher": "", "hooks": [{"type": "command", "command": "hooks/user-prompt-submit.sh"}]}],
    "Stop":              [{"matcher": "", "hooks": [{"type": "command", "command": "hooks/stop.sh"}]}]
  }
}
```

> There is no native `session-start` hook.  Wire it via your shell RC:
> ```bash
> alias claude='hooks/session-start.sh && claude'
> ```

---

## CLI Reference — all 15 commands

### Authentication

| Command | Description |
|---------|-------------|
| `fixonce login` | Log in via GitHub OAuth (opens browser) |
| `fixonce auth` | Authenticate via Ed25519 challenge-response (headless) |
| `fixonce keys add` | Generate and register a new signing key |
| `fixonce keys list` | List all registered signing keys |
| `fixonce keys revoke <key-id>` | Revoke a signing key |

### Memory CRUD

| Command | Description |
|---------|-------------|
| `fixonce create [FLAGS]` | Create a new memory (see flags below) |
| `fixonce get <id>` | Retrieve a memory by UUID |
| `fixonce update <id> [FLAGS]` | Partially update a memory |
| `fixonce delete <id>` | Soft-delete a memory |
| `fixonce feedback <id> <helpful\|outdated\|damaging>` | Submit feedback that adjusts the memory's decay/reinforcement scores |

### Intelligence

| Command | Description |
|---------|-------------|
| `fixonce query <text>` | Run the full read pipeline (vector search + Claude refinement) |
| `fixonce lineage <id>` | Show the mutation history chain for a memory |
| `fixonce analyze <session-log>` | Extract memory candidates from a Claude Code session transcript |

### Environment

| Command | Description |
|---------|-------------|
| `fixonce detect` | Detect Midnight ecosystem versions in the current project |
| `fixonce context` | Gather full project context (versions + git branch + file structure) |

### Utilities

| Command | Description |
|---------|-------------|
| `fixonce config` | Display the active CLI configuration |
| `fixonce tui` | Launch the interactive Ratatui terminal UI |
| `fixonce hook <event>` | Dispatch a Claude Code lifecycle hook (called by shell scripts) |

### `fixonce create` flags

```
--title <TITLE>       Memory title (required)
--content <CONTENT>   Full memory content (required)
--summary <SUMMARY>   One-sentence summary (required)
--type <TYPE>         gotcha | best_practice | correction | anti_pattern | discovery
--source <SOURCE>     correction | observation | pr_feedback | manual | harvested
--language <LANG>     Programming language tag (e.g. rust, python, typescript)
--source-url <URL>    Link to the original source or issue
--repo-url <URL>      Repository URL
--format <FORMAT>     text | json (output format)
```

### Output formats

Most commands support `--format text` (default) or `--format json`.

---

## Configuration reference

FixOnce reads configuration from environment variables only; there is no
configuration file.

| Variable | Description | Default |
|----------|-------------|---------|
| `FIXONCE_API_URL` | Supabase project URL | `https://fixonce.supabase.co` |

Credentials (JWT and Ed25519 keys) are stored exclusively in the OS keyring
using the `keyring` crate.  They are **never** written to disk in plain text.

---

## Memory model

A memory record captures a piece of developer knowledge:

| Field | Type | Description |
|-------|------|-------------|
| `id` | UUID | Unique identifier |
| `title` | string | Short descriptive title |
| `content` | string | Full knowledge payload |
| `summary` | string | One-sentence summary for search ranking |
| `memory_type` | enum | `gotcha`, `best_practice`, `correction`, `anti_pattern`, `discovery` |
| `source_type` | enum | `correction`, `observation`, `pr_feedback`, `manual`, `harvested` |
| `language` | string? | Programming language tag |
| `decay_score` | float | Relevance weight (1.0 = fresh, approaches 0.0 over time) |
| `reinforcement_score` | float | Boosted by helpful feedback |
| `embedding_status` | enum | `complete`, `pending`, `failed` |
| `pipeline_status` | enum | `complete`, `incomplete` |

### Decay and reinforcement

Memories naturally decay with a 30-day half-life:

```
decay_score = initial × 0.5^(days_elapsed / 30)
```

Positive feedback (`helpful`) reinforces the score; `damaging` feedback
reduces it.  When the decay score drops below `0.1` the memory is eligible
for soft-deletion.

---

## Hook behaviour

| Hook | Trigger | Action |
|------|---------|--------|
| `session-start` | Session begins | Surfaces top 3 critical memories |
| `user-prompt-submit` | User submits prompt | Injects top 5 relevant memories |
| `pre-tool-use` | Before tool executes (similarity > 0.7) | Warns on anti-pattern matches |
| `post-tool-use` | After tool executes (similarity > 0.5) | Advises on related memories |
| `stop` | Session ends | Surfaces session-end reminders |

All hooks enforce a **3-second hard timeout** and always exit `0` — they are
warn-only and never block the agent.

---

## Development

### Build

```bash
cargo build --workspace
```

### Test

```bash
cargo test --workspace
```

The test suite includes:

- **Unit tests** inline in every module (`#[cfg(test)]`)
- **Integration tests** in `crates/fixonce-core/tests/`
  - `e2e_memory_lifecycle.rs` — memory create/serialize/format/decay cycle
  - `e2e_auth_flow.rs` — JWT expiry, keypair generation, nonce signing
  - `e2e_write_pipeline.rs` — credential detection, quality gate, dedup, enrichment
  - `e2e_dynamics.rs` — contradiction resolution, decay simulation, lineage chains
  - `e2e_detection.rs` — Midnight version detection, project context gathering
  - `bench_hot_cache.rs` — performance assertions (insert+query 50 items < 50ms)

### Lint

```bash
cargo clippy --workspace -- -D warnings
```

### Format

```bash
cargo fmt --all
```

---

## Contributing

1. Fork the repository and create a feature branch from `main`.
2. Write tests for every new behaviour — the project targets 100% logical
   coverage of pure functions.
3. Run `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
   before opening a PR.
4. Keep commits small and focused.  Commit messages follow the Conventional
   Commits convention: `feat:`, `fix:`, `test:`, `docs:`, `chore:`, `refactor:`.
5. PRs that add network-dependent code must mock the external calls in tests.

All contributions are welcome: bug fixes, documentation, new memory types,
additional language detection hints, and performance improvements.

---

## License

MIT — see `LICENSE`.
