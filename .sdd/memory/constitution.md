<!--
==============================================================================
SYNC IMPACT REPORT
Version: 1.0.0 (initial)
Added Principles: API-First Design, Modularity, KISS & YAGNI,
  Test Critical Paths, Fail Fast with Actionable Errors,
  Validate at System Boundaries, Protect Secrets, Semantic Versioning,
  Conventional Commits
Added Sections: Development Standards, Quality Gates, Governance
Templates requiring updates: N/A (initial constitution)
Follow-up TODOs: None
==============================================================================
-->

# FixOnce Constitution

## Core Principles

### I. API-First Design

The MCP tool interface is the primary contract. CLI and Web UI are
consumers of the same underlying operations, not independent
implementations.

- MUST define MCP tool schemas before implementing business logic
- CLI commands MUST map 1:1 to MCP tool operations
- Web UI MUST call the same service layer as MCP and CLI
- Changes to tool schemas that break existing agent integrations are
  MAJOR version bumps
- Rationale: Three interfaces (MCP, CLI, Web UI) serving the same
  operations demands a single source of truth. The MCP schema is that
  source because agents are the primary consumers.

### II. Modularity

Well-defined boundaries between components. Each module has a clear
purpose and explicit dependencies.

- Storage layer, retrieval pipeline, write pipeline, MCP server, CLI,
  and Web UI MUST be separate modules with explicit interfaces
- No circular dependencies between modules
- Each module MUST be independently testable against its interface
- Shared types live in a dedicated types/schema module
- Rationale: A modular monolith only works if the boundaries are real.
  Blurred boundaries create a monolith without the "modular."

### III. KISS & YAGNI

Do the simplest thing that works. Build what's needed now, not what
might be needed later.

- Features in the "Deferred" list MUST stay deferred until explicitly
  promoted
- No speculative abstractions — if there's only one implementation,
  don't create an interface for it
- Prefer standard library and well-known packages over custom solutions
- If you can't explain a design decision in one sentence, simplify it
- Rationale: Solo + AI development rewards simplicity. Complex
  abstractions slow down both human comprehension and agent reasoning.

### IV. Test Critical Paths

Focus testing on the paths where bugs cause real damage. Don't chase
coverage metrics.

- MUST test: retrieval pipeline (query rewriting, search, reranking),
  write pipeline (quality gate, duplicate detection), version predicate
  filtering, MCP tool input/output contracts
- SHOULD test: CLI argument parsing, Web UI data fetching
- MAY skip: UI component rendering, styling, simple CRUD wrappers
- Integration tests against real Supabase (or local equivalent) over
  mocks for storage and search operations
- Rationale: A broken retrieval pipeline surfaces wrong memories to
  agents. A broken quality gate floods the store with noise. These are
  the failure modes that matter.

### V. Fail Fast with Actionable Errors

When something goes wrong, say what happened and what to do about it.
No silent failures, no cryptic messages.

- Pipeline stage failures MUST identify which stage failed and why
- LLM API failures (OpenRouter, Voyage AI) MUST suggest checking API
  keys and connectivity
- Supabase connection failures MUST suggest checking credentials and
  network
- Never swallow exceptions in async operations (embedding generation,
  async retrieval)
- Rationale: Both agents and humans consume error output. Agents need
  structured, actionable errors to self-correct. Humans need context
  to debug.

### VI. Validate at System Boundaries

Trust internal code. Validate external input rigorously.

- MCP tool inputs MUST be validated against schemas before processing
- CLI arguments MUST be validated before calling service layer
- Memory content from agents MUST be validated (non-empty, within size
  limits) before entering the write pipeline
- Version predicates MUST conform to the defined component key format
- Internal module-to-module calls do NOT need redundant validation
- Rationale: FixOnce accepts input from LLM agents, which may produce
  malformed data. Validate once at the boundary, then trust internally.

### VII. Protect Secrets

API keys and credentials never appear in logs, error messages, memory
content, or client-facing output.

- MUST sanitize error messages from upstream APIs before surfacing
- MUST use environment variables for all credentials (Supabase,
  OpenRouter, Voyage AI)
- MUST NOT store API keys or tokens in memory content — the quality
  gate SHOULD reject memories containing credential patterns
- Rationale: FixOnce handles multiple API credentials and stores
  content that agents produce. Leaked credentials in memories would
  propagate to every agent that retrieves them.

### VIII. Semantic Versioning

Version the project and its interfaces clearly. Breaking changes are
communicated through version numbers.

- Follow semver: MAJOR for breaking changes, MINOR for new features,
  PATCH for fixes
- MCP tool schema changes that alter input/output shape are MAJOR
- New MCP tools or optional parameters are MINOR
- Memory schema migrations that alter existing field semantics are
  MAJOR
- Rationale: Agents and their configurations depend on specific MCP
  tool schemas. Breaking changes without version bumps silently break
  integrations.

### IX. Conventional Commits

All commit messages MUST follow the Conventional Commits specification.

- Format: `type(scope): subject` — e.g., `feat(mcp): add feedback tool`
- Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`,
  `chore`, `ci`, `perf`, `build`
- Scope SHOULD reflect the module: `mcp`, `cli`, `web`, `pipeline`,
  `storage`, `schema`
- Breaking changes MUST include `!` after type/scope or a
  `BREAKING CHANGE:` footer
- Rationale: Open-source project needs clear, searchable history.
  Conventional commits also enable automated changelogs and version
  bumps tied to Principle VIII (Semantic Versioning).

## Development Standards

- **Language**: TypeScript throughout (MCP server, CLI, Web UI)
- **Runtime**: Node.js for server/CLI, browser for Web UI
- **Storage**: Supabase Postgres + pgvector
- **Embeddings**: Voyage AI (voyage-code-3, 1024 dimensions)
- **LLM calls**: OpenRouter (cheap models for quality gate, dedup,
  query rewriting, reranking)
- **Web UI**: React + Vite
- **Code style**: Prefer explicit types over `any`. Use `strict` mode
  in tsconfig. Let the formatter handle formatting debates.

## Quality Gates

Before merging any change:

1. **Critical path tests pass** — retrieval pipeline, write pipeline,
   version filtering, MCP tool contracts
2. **No new `any` types** without explicit justification comment
3. **No secrets in code** — credentials via environment variables only
4. **MCP tool schema changes documented** — if tool inputs/outputs
   changed, version bump determined

## Governance

This constitution governs all development decisions for FixOnce.
When a decision conflicts with these principles, the constitution wins.

- **Amendments**: Update this document, increment the version, and
  document the change in the sync impact report header
- **Principle additions**: Require clear rationale tied to a real
  problem encountered during development
- **Principle removals**: Require evidence that the principle is
  harmful or no longer relevant
- **Override**: Any principle can be overridden in a specific case
  with an explicit justification comment in the code or PR description

**Version**: 1.0.0 | **Ratified**: 2026-03-04 | **Last Amended**: 2026-03-04
