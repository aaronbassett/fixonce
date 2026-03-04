# Constitution Compliance Framework

**Spec**: 001-fixonce-memory-layer
**Constitution Version**: 1.0.0
**Created**: 2026-03-04

This document maps each constitutional principle to specific compliance checks
for the FixOnce memory layer. Every task and implementation decision in the plan
must pass these checks before merging.

---

## Principle I: API-First Design

### MUST Requirements

- [ ] MCP tool schemas (7 tools) are defined and reviewed BEFORE any business
      logic implementation begins
- [ ] CLI commands map 1:1 to MCP tool operations:
  | MCP Tool | CLI Command |
  |----------|-------------|
  | `fixonce_create_memory` | `fixonce create` |
  | `fixonce_query` | `fixonce query` |
  | `fixonce_expand` | (no CLI equivalent -- must add or document omission) |
  | `fixonce_get_memory` | `fixonce get` |
  | `fixonce_update_memory` | `fixonce update` |
  | `fixonce_feedback` | `fixonce feedback` |
  | `fixonce_detect_environment` | `fixonce detect` |
- [ ] Web UI calls the same service layer as MCP and CLI (no separate data
      access code in the React app)
- [ ] Breaking MCP schema changes trigger a MAJOR version bump

### SHOULD Requirements

- Schema definitions live in a single source-of-truth file that both MCP server
  and CLI import from
- CLI `--json` output matches MCP tool response shape

### Compliance Checks

1. Verify MCP tool input/output schemas exist before any implementation PR
2. Verify CLI commands call service-layer functions, not direct DB queries
3. Verify Web UI fetches from the same service layer (or an HTTP API wrapping it)
4. Grep for direct Supabase client usage outside the storage module -- none
   should exist in CLI or Web UI code

### Potential Violations to Watch

- `fixonce_expand` has no CLI equivalent in the spec (Story 6 commands list).
  The plan must either add a CLI `expand` command or explicitly justify omission
- Web UI building its own query logic instead of reusing the retrieval pipeline
- CLI formatting logic leaking into the service layer

---

## Principle II: Modularity

### MUST Requirements

- [ ] Separate modules with explicit interfaces for:
  - Storage layer (Supabase client, queries)
  - Retrieval pipeline (query rewriting, search, reranking)
  - Write pipeline (quality gate, duplicate detection, storage)
  - MCP server (tool definitions, request handling)
  - CLI (argument parsing, output formatting)
  - Web UI (React app, API layer)
- [ ] Shared types live in a dedicated types/schema module
- [ ] No circular dependencies between modules
- [ ] Each module is independently testable against its interface

### SHOULD Requirements

- Module boundaries should be enforceable (e.g., via package structure or
  ESLint import rules)
- Shared configuration (env vars, connection strings) centralized in one module

### Compliance Checks

1. Each module has its own directory with an explicit public API (index.ts or
   barrel exports)
2. Run a circular dependency checker (e.g., `madge --circular`) on every PR
3. Each module has tests that import only from its public interface, not
   internal files
4. The types/schema module has zero runtime dependencies on other modules

### Potential Violations to Watch

- Storage layer leaking Supabase-specific types into the pipeline modules
- Retrieval pipeline importing directly from the write pipeline (or vice versa)
- MCP server containing business logic instead of delegating to pipelines
- Web UI importing from MCP server code directly

---

## Principle III: KISS & YAGNI

### MUST Requirements

- [ ] The following features remain DEFERRED and must not appear in the plan:
  - Reinforcement scoring (confidence auto-adjustment)
  - Contradiction detection between memories
  - Memory clustering / auto-categorization
  - Team scoping / multi-tenant isolation
  - Bulk operations (batch create/update/delete)
  - Exportable memory packs
- [ ] No speculative abstractions -- if there is only one implementation, no
      interface/abstract class wrapping it

### SHOULD Requirements

- Prefer standard library and well-known packages over custom solutions
- Design decisions explainable in one sentence

### Compliance Checks

1. Search codebase for deferred feature keywords: `reinforcement`, `cluster`,
   `team_scope`, `bulk_`, `export_pack`, `contradiction`
2. No abstract base classes or interfaces with a single implementation
3. No plugin systems, event buses, or middleware chains unless justified by
   current requirements
4. No generic "provider" abstractions over Supabase, OpenRouter, or Voyage AI
   (there is only one implementation of each)

### Potential Violations to Watch

- Over-engineering the retrieval pipeline with pluggable stages when the spec
  defines exactly three fixed stages
- Creating an abstract `EmbeddingProvider` when only Voyage AI is used
- Creating an abstract `LLMProvider` when only OpenRouter is used
- Adding a generic storage abstraction over Supabase "for future flexibility"
- Premature optimization of search (e.g., caching layers, query result pools)
  before measuring real performance

---

## Principle IV: Test Critical Paths

### MUST Test

- [ ] Retrieval pipeline: query rewriting produces usable search queries
- [ ] Retrieval pipeline: hybrid search returns correct results for metadata +
      vector queries
- [ ] Retrieval pipeline: reranking orders results correctly
- [ ] Write pipeline: quality gate accepts actionable memories, rejects vague
      ones
- [ ] Write pipeline: duplicate detection resolves to correct outcome (discard,
      replace, update, merge)
- [ ] Version predicate filtering: all 7 scenarios from Story 4
- [ ] MCP tool contracts: each of the 7 tools accepts valid input and rejects
      invalid input

### SHOULD Test

- [ ] CLI argument parsing and validation
- [ ] Web UI data fetching and state management
- [ ] Two-tier result budgeting (top 5 + overflow)
- [ ] Cache key expansion

### MAY Skip

- UI component rendering and styling
- Simple CRUD wrappers (direct pass-through to storage)
- Output formatting (human-readable display)

### Compliance Checks

1. CI pipeline includes tests for all MUST items
2. Integration tests run against real Supabase (or local equivalent), not mocks,
   for storage and search operations
3. Pipeline stage tests can run in isolation (mocking adjacent stages)
4. MCP tool contract tests validate both happy path and error responses

### Potential Violations to Watch

- Mocking Supabase for search tests instead of running against real pgvector
- Testing only happy paths in the quality gate (must test rejection cases)
- Skipping version predicate edge cases (AND across components, null predicates)
- No integration test for the full pipeline end-to-end (rewrite -> search ->
  rerank)

---

## Principle V: Fail Fast with Actionable Errors

### MUST Requirements

- [ ] Pipeline stage failures identify which stage failed and why:
  - "Query rewriting failed: [reason]"
  - "Hybrid search failed: [reason]"
  - "Reranking failed: [reason]"
  - "Quality gate failed: [reason]"
  - "Duplicate detection failed: [reason]"
- [ ] LLM API failures (OpenRouter, Voyage AI) suggest checking API keys and
      connectivity
- [ ] Supabase connection failures suggest checking credentials and network
- [ ] No swallowed exceptions in async operations:
  - Embedding generation
  - Async retrieval (mid-run injection)
  - Quality gate evaluation
  - Duplicate detection

### SHOULD Requirements

- Structured error objects with `stage`, `reason`, and `suggestion` fields
- MCP tool errors return structured error responses, not generic "internal error"

### Compliance Checks

1. Every `catch` block either re-throws with context or returns an actionable
   error
2. No bare `catch (e) {}` or `catch (e) { console.log(e) }` patterns
3. Async functions that call external APIs (OpenRouter, Voyage AI, Supabase)
   have explicit error handling with actionable messages
4. Pipeline stages wrap errors with stage identification

### Potential Violations to Watch

- Embedding generation failing silently (memory appears stored but never becomes
  vector-searchable with no indication why)
- OpenRouter rate limits surfaced as generic "500 Internal Server Error"
- Supabase connection timeout surfaced without suggesting credential check
- AsyncIterable mid-run injection dropping errors silently

---

## Principle VI: Validate at System Boundaries

### MUST Validate

- [ ] MCP tool inputs validated against schemas before processing
  - All 7 tools: required fields present, correct types, enum values valid
  - `fixonce_query`: pipeline toggles are booleans, verbosity is valid enum,
    max_results > 0
  - `fixonce_feedback`: tags are from allowed enum list, suggested_action valid
  - `fixonce_create_memory`: content non-empty, within size limits
- [ ] CLI arguments validated before calling service layer
  - Required flags present for each command
  - UUID format for memory IDs
  - Enum values for `--source-type`, `--memory-type`, `--action`, `--tags`
- [ ] Memory content validated:
  - Non-empty
  - Within size limits (define max content length)
  - Version predicates conform to component key format (keys from allowed list)
- [ ] Version predicate keys validated against the 12 supported component keys

### MUST NOT Validate

- [ ] Internal module-to-module calls (e.g., pipeline calling storage)

### Compliance Checks

1. MCP tool handlers validate input before calling service functions
2. CLI command handlers validate args before calling service functions
3. Service-layer functions do NOT re-validate what boundaries already checked
4. Schema validation uses a library (e.g., Zod) with shared schemas

### Potential Violations to Watch

- Redundant validation inside the storage layer for data already validated at
  MCP/CLI boundary
- Missing validation on `version_predicates` component keys (accepting arbitrary
  keys)
- Missing size limit on memory `content` field (agents could submit enormous
  content)
- CLI not validating UUID format before passing to service layer

---

## Principle VII: Protect Secrets

### MUST Requirements

- [ ] Environment variables used for all credentials:
  - `SUPABASE_URL`
  - `SUPABASE_SERVICE_KEY` (or `SUPABASE_ANON_KEY`)
  - `OPENROUTER_API_KEY`
  - `VOYAGE_API_KEY`
- [ ] Error messages from upstream APIs sanitized before surfacing
  - OpenRouter errors may contain API key fragments
  - Supabase errors may contain connection strings
  - Voyage AI errors may contain request headers
- [ ] Quality gate SHOULD reject memories containing credential patterns
  - Regex patterns for: API keys, tokens, passwords, connection strings
  - Applied to `content` and `title` fields during write pipeline

### MUST NOT

- [ ] API keys never in source code, logs, or memory content
- [ ] No credentials in error messages returned to MCP clients or CLI output
- [ ] No `.env` files committed to repository

### Compliance Checks

1. Grep codebase for hardcoded API keys, tokens, connection strings
2. Error handling code strips/replaces upstream error details before returning
3. Quality gate includes a credential pattern check
4. `.gitignore` includes `.env*` patterns
5. CI check for secrets in committed code (e.g., git-secrets or similar)

### Potential Violations to Watch

- OpenRouter API error messages passed through verbatim to MCP tool response
- Supabase connection errors containing the full connection URL with credentials
- An agent storing an API key as a "memory" and it passing the quality gate
- Debug logging that includes request headers with auth tokens

---

## Principle VIII: Semantic Versioning

### MUST Requirements

- [ ] MCP tool schema changes that alter input/output shape = MAJOR version bump
- [ ] New MCP tools or optional parameters = MINOR version bump
- [ ] Memory schema migrations altering existing field semantics = MAJOR
- [ ] Bug fixes with no schema changes = PATCH

### Specific Version Bump Triggers

| Change | Version Bump |
|--------|-------------|
| Rename or remove an MCP tool | MAJOR |
| Change required params of an MCP tool | MAJOR |
| Change response shape of an MCP tool | MAJOR |
| Add new MCP tool | MINOR |
| Add optional param to existing tool | MINOR |
| Change `version_predicates` structure | MAJOR |
| Add new feedback tag to enum | MINOR |
| Add new verbosity level | MINOR |
| Fix a bug in retrieval pipeline | PATCH |

### Compliance Checks

1. Every PR that touches MCP tool schemas includes a version bump determination
2. CHANGELOG maintained with version entries
3. Memory schema migrations reviewed for semantic changes

### Potential Violations to Watch

- Adding a required field to `fixonce_query` without a MAJOR bump
- Changing the shape of query results (e.g., renaming `relevancy_score`) without
  a MAJOR bump
- Altering `version_predicates` format without a MAJOR bump

---

## Principle IX: Conventional Commits

### MUST Requirements

- [ ] All commits follow: `type(scope): subject`
- [ ] Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`,
      `chore`, `ci`, `perf`, `build`
- [ ] Breaking changes include `!` or `BREAKING CHANGE:` footer

### Recommended Scopes

| Scope | Module |
|-------|--------|
| `mcp` | MCP server, tool definitions |
| `cli` | CLI commands, argument parsing |
| `web` | Web UI (React app) |
| `pipeline` | Retrieval and write pipelines |
| `storage` | Supabase storage layer |
| `schema` | Shared types, database schema |

### Compliance Checks

1. Pre-commit hook or CI check enforces conventional commit format
2. Breaking changes in commit messages match actual version bumps
3. Scope reflects the module most affected

### Potential Violations to Watch

- Generic scopes like `core` or `misc` instead of the defined module scopes
- Missing `!` on commits that change MCP tool schemas
- `feat` used for bug fixes or `fix` used for new features

---

## Development Standards Compliance

### TypeScript Strict Mode

- [ ] `tsconfig.json` has `"strict": true`
- [ ] No `any` types without explicit justification comment
- [ ] Explicit return types on public API functions

### Credentials

- [ ] All credentials via environment variables
- [ ] No default/fallback values for credentials in code
- [ ] Startup fails fast if required env vars are missing

### Compliance Checks

1. `tsconfig.json` strict mode verified in CI
2. ESLint rule `@typescript-eslint/no-explicit-any` set to `error` (with
   per-line overrides requiring comment)
3. Application startup validates all required env vars present

---

## Quality Gates for Merging

Every PR must pass these gates before merge:

### Gate 1: Critical Path Tests Pass

- [ ] Retrieval pipeline tests (rewrite, search, rerank)
- [ ] Write pipeline tests (quality gate, duplicate detection)
- [ ] Version filtering tests (all 7 scenarios)
- [ ] MCP tool contract tests (all 7 tools)

### Gate 2: No New `any` Types

- [ ] Zero new `any` without justification comment
- [ ] ESLint check passes

### Gate 3: No Secrets in Code

- [ ] No credentials, API keys, or tokens in source
- [ ] Secret scanning passes

### Gate 4: MCP Schema Changes Documented

- [ ] If tool inputs/outputs changed, version bump determined
- [ ] CHANGELOG updated if applicable

---

## Compliance Verification Checklist (For Plan Review)

Use this checklist when reviewing the implementation plan:

- [ ] Plan tasks include MCP schema definition BEFORE implementation tasks
- [ ] Plan has separate modules for storage, retrieval pipeline, write pipeline,
      MCP server, CLI, Web UI
- [ ] Plan does NOT include any deferred features
- [ ] Plan includes tests for all MUST-test items
- [ ] Plan includes error handling strategy for each external API
- [ ] Plan includes input validation at MCP and CLI boundaries
- [ ] Plan includes credential management via env vars
- [ ] Plan includes version bump strategy for schema changes
- [ ] Plan enforces conventional commit format
- [ ] `fixonce_expand` CLI equivalent is addressed (add command or justify
      omission)
