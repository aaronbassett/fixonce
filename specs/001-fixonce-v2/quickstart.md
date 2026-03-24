# Quickstart: FixOnce v2 Development

## Prerequisites

- Rust toolchain (stable, latest) — install via [rustup](https://rustup.rs/)
- Deno runtime — install via [deno.land](https://deno.land/)
- Supabase CLI — install via `brew install supabase/tap/supabase`
- Lefthook — install via `brew install lefthook` or `cargo install lefthook`
- Claude Code CLI — for inference pipeline (`claude -p`)
- A Supabase project (for database + edge functions)
- A VoyageAI API key (for embeddings)

## Initial Setup

```bash
# Clone and enter repo
git clone https://github.com/devrel-ai/fixonce.git
cd fixonce

# Install Lefthook git hooks
lefthook install

# Verify Rust toolchain
cargo --version
rustc --version

# Verify Deno
deno --version

# Verify Supabase CLI
supabase --version
```

## Database Setup

```bash
# Link to your Supabase project
supabase link --project-ref <your-project-ref>

# Run all migrations
supabase db push

# Deploy edge functions
supabase functions deploy
```

## Environment Configuration

The CLI needs the backend API URL:

```bash
export FIXONCE_API_URL=https://your-project.supabase.co
```

All other secrets (VoyageAI API key, etc.) are stored encrypted on the server and retrieved ephemerally.

## Available Commands

### Quality Checks

```bash
# Run all checks (Rust + Deno)
make check

# Rust only
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo audit

# Deno only (edge functions)
cd supabase/functions
deno fmt --check
deno lint
deno check **/*.ts
```

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run the CLI
cargo run -- --help
cargo run -- query "compact map iteration"
cargo run -- tui
```

### Test

```bash
# All Rust tests
cargo test

# Specific crate
cargo test -p fixonce-core
cargo test -p fixonce-cli
cargo test -p fixonce-hooks

# Edge function tests
cd supabase/functions
deno test
```

## Project Structure

```
crates/
  fixonce-cli/      # Binary crate — CLI + TUI entry point
  fixonce-core/     # Library crate — shared logic, API client, pipelines
  fixonce-hooks/    # Library crate — Claude Code hook implementations

supabase/
  migrations/       # SQL migrations (Supabase CLI managed)
  functions/        # Deno edge functions

hooks/              # Claude Code hook shell scripts
```

## CI/CD

- **PR checks**: GitHub Actions runs Rust (fmt, clippy, test, audit) and Deno (fmt, lint, check) in parallel
- **Release**: Tagged releases trigger cross-platform binary builds (macOS ARM64, macOS x86_64, Linux x86_64)
- **Edge functions**: Deploy to Supabase on merge to `main`
