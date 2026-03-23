<!--
==============================================================================
SYNC IMPACT REPORT
==============================================================================
Version change: 1.0.0 → 1.1.0 (MINOR: web dashboard removed, TUI added,
  scope references updated, commit scopes updated)

Modified principles:
  - I. Edge Functions Are the API → updated "web dashboard" → "TUI"
  - V. TypeScript Strict Mode → renamed to "Deno Strict Mode for Edge Functions"
  - VII. Comprehensive Testing → replaced frontend testing with Deno testing
  - VIII. Conventional Commits → updated scopes (removed `web`, `storage`,
    `shared`, `activity`; added `tui`, `edge-functions`, `auth`, `secrets`)
  - IX. API-First Design → updated to two codebases (Rust + Deno), not three

Added sections: None (structure unchanged)
Removed sections: None

Templates requiring updates:
  - plan-template.md: ✅ No changes needed (constitution check is dynamic)
  - spec-template.md: ✅ No changes needed (generic story format)
  - tasks-template.md: ✅ No changes needed (generic task format)

Follow-up TODOs: None
==============================================================================
-->

# FixOnce Constitution

## Core Principles

### I. Edge Functions Are the API

All database operations go through Supabase edge functions. No client — CLI, TUI, or future MCP server — queries the database directly. Row Level Security (RLS) enforces this at the database layer; edge functions enforce it at the application layer. The edge function contract is the system's source of truth.

**Rules:**
- Every data operation is an edge function, no exceptions
- RLS policies must exist for every table; deny by default
- Edge functions validate auth, input schemas, and rate limits before touching the database
- The CLI and TUI are consumers of the same API surface — no special paths

**Rationale:** FixOnce handles secrets, auth tokens, and session transcripts. A single, auditable API surface prevents accidental data exposure and makes access control enforceable. When this opens to ecosystem developers, multi-tenant isolation depends on this boundary holding.

### II. Secrets Never Touch Disk

API keys, tokens, and credentials are stored encrypted in the database and decrypted only in-memory, on-demand, via authenticated edge function requests. The CLI requests secrets, uses them, and discards them. Nothing is written to the filesystem.

**Rules:**
- The CLI must never write secrets to disk (no config files, no dotfiles, no temp files)
- Secrets are held in memory only for the duration of the operation that needs them
- Server-side encryption/decryption happens in edge functions, not in client code
- Environment variables are acceptable for Supabase connection credentials only (URL + anon key)
- Log output must never contain secrets, tokens, or keys — even in debug/verbose mode

**Rationale:** The v1 architecture stored API keys in local config files and env vars. This doesn't scale to teams and creates a credential sprawl problem. Server-side secrets with ephemeral client access is the security model that supports multi-tenant deployment.

### III. Unix CLI Philosophy

The Rust CLI is a composable tool that follows Unix conventions. It reads structured input, writes structured output, reports errors to stderr, and exits with meaningful codes.

**Rules:**
- Every command supports `--format text` (human-readable, default), `--format json`, and `--format toon`
- stdin for input, stdout for output, stderr for errors and diagnostics
- Exit code 0 for success, non-zero for failure with documented codes
- No interactive prompts in non-TTY mode; commands must be scriptable
- `--quiet` suppresses non-essential output; `--verbose` adds diagnostic detail
- Long-running operations show progress indication on TTY, silence on non-TTY

**Rationale:** The CLI serves two audiences: developers at a terminal and LLM agents consuming structured output. Unix conventions make it composable with other tools and predictable for both audiences. TOON format optimizes for agent token efficiency.

### IV. Strict Rust Discipline

The CLI is written in Rust with strict quality gates. No shortcuts in library code; pragmatic allowances in binary entry points only.

**Rules:**
- All clippy warnings enabled and treated as errors (`#![deny(clippy::all, clippy::pedantic)]`)
- `thiserror` for all public error types; structured error enums, not string errors
- No `unwrap()` or `expect()` in library code; `expect()` with descriptive messages acceptable in `main.rs` and CLI argument parsing only
- All public APIs documented with `///` doc comments
- `cargo test`, `cargo clippy`, and `cargo fmt --check` must pass before commit

**Rationale:** Solo development with heavy agent assistance demands strict compiler-enforced guardrails. Agents generate code that compiles but may cut corners on error handling. Clippy pedantic and the unwrap ban catch these before they ship.

### V. Deno Strict Mode for Edge Functions

Supabase edge functions are the only TypeScript/Deno code in the project (the web dashboard was replaced by a Rust TUI). Deno's native tooling enforces quality.

**Rules:**
- `deno lint` with default rules, warnings as errors
- `deno fmt` for formatting consistency
- `deno check` for type checking (strict by default in Deno)
- No `any` types; use `unknown` and narrow explicitly
- Zod schemas for all edge function input/output boundaries
- Edge function tests run via `deno test`

**Rationale:** Edge functions are the sole API surface between clients and the database. Type safety in these functions prevents the class of bugs where input validation fails silently or API contracts drift. Deno's native tooling is official, fast, and requires zero configuration.

### VI. Fail Fast with Actionable Errors

When something goes wrong, the system fails immediately with a clear, human-readable explanation and a suggested next step. No silent failures, no swallowed errors, no generic messages.

**Rules:**
- Every error message includes: what happened, why it likely happened, and what to do about it
- CLI errors format differently for text vs. JSON/TOON output — human gets prose, agent gets structured error codes
- Edge function errors return appropriate HTTP status codes with structured error bodies
- Auth failures surface the specific issue (expired key, revoked access, invalid signature) — not generic 401s
- Pipeline failures (LLM timeout, embedding generation failure) include retry guidance
- Never catch-and-ignore; if you catch, you must handle or re-raise with added context

**Rationale:** FixOnce depends on multiple external services (Supabase, VoyageAI, Claude). When any link in the chain breaks, the user needs to know which one and how to fix it. Agents consuming the CLI need structured error codes to decide whether to retry or escalate.

### VII. Comprehensive Testing

Every component has tests proportional to its risk. Pipeline logic and data integrity paths have thorough test coverage. Integration tests run against real services where feasible.

**Rules:**
- Rust: `cargo test` covers all library modules; integration tests for CLI commands
- Pipeline logic (quality gating, dedup, reranking, decay) requires unit tests for every branch
- Auth flows require integration tests against Supabase
- Embedding generation requires integration tests against VoyageAI (or recorded responses)
- Edge functions require tests that verify RLS enforcement
- Deno edge functions: tests via `deno test` required for auth and RLS logic
- No mocks for things you own; mock only external service boundaries
- Tests must be deterministic — no flaky tests allowed in CI

**Rationale:** This system's core value is trust: developers trust the memories it surfaces, and agents trust the pipeline results. A memory that surfaces stale info or a dedup that merges incorrectly erodes that trust. Comprehensive tests protect the trust boundary.

### VIII. Conventional Commits

All commits follow the Conventional Commits specification with Angular convention. This applies to every commit, whether authored by a human or an agent.

**Rules:**
- Format: `type(scope): subject` — lowercase type, lowercase scope, imperative mood subject
- Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `ci`, `perf`, `style`
- Scopes match component names: `cli`, `tui`, `edge-functions`, `pipeline`, `auth`, `secrets`, `hooks`, `ci`
- Breaking changes use `!` suffix: `feat(cli)!: change output format`
- Body explains *why*, not *what* — the diff shows the what
- Agent-authored commits include `Co-Authored-By: Claude Code <noreply@anthropic.com>`

**Rationale:** With a solo developer and multiple agents contributing, commit history is the primary audit trail. Conventional commits make the history searchable, parseable, and useful for automated changelog generation.

### IX. API-First Design

Edge function contracts are designed before implementation. The CLI, TUI, and any future consumers are built against these contracts, not against database schemas.

**Rules:**
- New features start with the edge function request/response schema (Zod)
- Database schema changes are implementation details behind the edge function contract
- Breaking API changes require a version bump and migration path
- Edge function contracts are the shared language between Rust CLI and Supabase backend

**Rationale:** FixOnce has two distinct codebases (Rust CLI/TUI and Deno edge functions) consuming the same data. API-first design prevents the "works in the CLI but breaks in the edge function" class of bugs by establishing a single contract all consumers implement against.

### X. Simplicity Until Proven Otherwise

Start with the simplest approach. Add complexity only when the simple approach demonstrably fails. Complexity requires justification.

**Rules:**
- No abstraction layers until the third concrete use case (Rule of Three)
- No speculative features — build when there's a real need, not "just in case"
- If a pipeline technique doesn't measurably improve results, remove it
- Prefer explicit code over clever code; the next reader is an agent with no context
- Configuration options must earn their existence — every knob is a maintenance burden

**Rationale:** The feature set is already ambitious (full RAG pipeline, memory dynamics, multi-format CLI, auth, secrets management). Unnecessary complexity on top of inherent complexity compounds into unmaintainability. Every abstraction must justify itself against the alternative of just writing the code directly.

## Security Requirements

### Authentication & Authorization
- GitHub OAuth via Supabase with org/team-based access restrictions
- CLI uses public-key authentication — users register CLI public keys to their account
- Every edge function request is authenticated; no anonymous access to data operations
- Role-based access for admin vs. standard operations in the TUI

### Data Protection
- Secrets encrypted at rest in the database
- All API communication over HTTPS
- Session transcript data treated as sensitive — not logged, not cached to disk
- Memory content may contain code snippets from private repos — access-controlled accordingly

### Supply Chain
- Dependency updates reviewed before merging (Rust: `cargo audit`, JS: `npm audit`)
- No unnecessary dependencies — prefer standard library where feasible
- Lock files committed and verified in CI

## Development Workflow

### Branch Strategy
- Trunk-based development: short-lived feature branches off `main`
- Branch naming: `type/short-description` (e.g., `feat/memory-decay`, `fix/auth-timeout`)
- Merge via pull request; squash merge preferred for clean history

### CI Pipeline
- Rust: `cargo fmt --check`, `cargo clippy`, `cargo test`, `cargo audit` on every PR
- Deno edge functions: `deno fmt --check`, `deno lint`, `deno check` on every PR
- Edge functions: deploy to staging on PR, production on merge to `main`
- All checks must pass before merge; no bypassing CI

### Continuous Delivery
- `main` is always deployable
- CLI binary releases are tagged and published as GitHub release artifacts
- Edge functions deploy to Supabase on merge to `main`

## Governance

This constitution governs all development on FixOnce v2. It supersedes informal conventions, previous v1 practices, and agent default behaviors.

- **Compliance is required** — all PRs must be consistent with these principles
- **Agents must follow this constitution** — it takes precedence over general coding conventions
- **Amendments require documentation** — changes to this constitution must include rationale and be committed as a versioned change
- **Principles are ordered by priority** — in cases of conflict, earlier principles take precedence (e.g., "Secrets Never Touch Disk" overrides convenience optimizations)
- **Pragmatism clause** — if a principle blocks progress with no clear benefit in a specific case, document the exception and the reasoning in the commit message

**Version**: 1.1.0 | **Ratified**: 2026-03-23 | **Last Amended**: 2026-03-23
