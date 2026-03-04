# Decision Log: fixonce-memory-layer

*Chronological record of all decisions made during discovery.*

---

[Decision entries will be added as decisions are made]

## D1: Local-first deployment model — 2026-03-04

**Context**: Need to determine deployment architecture for v1

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Local-first: SQLite + Chroma for storage, external LLM APIs where needed. Not 100% offline required.

**Rationale**: Simplest path to MVP, no infrastructure overhead for users

**Implications**:
Storage backend decided (SQLite + Chroma). No hosted service needed. Team scoping deferred.

**Stories Affected**: [Stories not specified]

**Related Questions**: [Questions not specified]

---

## D2: Midnight-first, not ecosystem-agnostic — 2026-03-04

**Context**: Whether v1 should be generic or Midnight-specific

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Build MVP for Midnight ecosystem first, generalize later

**Rationale**: Concrete target ecosystem avoids premature abstraction. Midnight's version coupling makes it an ideal first case.

**Implications**:
Version predicates can be Midnight-specific in v1. No need for plugin/extension architecture yet.

**Stories Affected**: [Stories not specified]

**Related Questions**: [Questions not specified]

---

## D3: Defer team scoping — 2026-03-04

**Context**: How to handle multi-user/multi-agent sharing

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Single local server, no team/auth scoping in v1

**Rationale**: Reduces complexity. Local-first model means single user/machine.

**Implications**:
Q4 resolved. No auth system needed. Team sharing is a post-v1 concern.

**Stories Affected**: [Stories not specified]

**Related Questions**: Q4

---

## D4: v1 scope: stories 1-6 and 11 — 2026-03-04

**Context**: Which proto-stories are in scope for v1

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Core v1: Memory Creation, Memory Retrieval, MCP Server, CLI, Web UI, Version-Scoped Metadata, Memory Storage & Schema

**Rationale**: Covers full read/write loop with all three interfaces and version awareness

**Implications**:
Reinforcement scoring, contradiction detection, and memory clustering deferred to post-v1

**Stories Affected**: [Stories not specified]

**Related Questions**: [Questions not specified]

---

## D5: Memory authorship tracking — 2026-03-04

**Context**: How to track who created a memory

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Enum created_by: ai, human, human_modified. Plus optional source_url for provenance link (PR comment, CI workflow, etc.)

**Rationale**: Captures authorship without overcomplicating. source_url provides traceability back to the triggering event.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 1

**Related Questions**: Q3

---

## D6: Rich project context fields — 2026-03-04

**Context**: How detailed should project scoping be

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Capture full project context: project name, repo URL, workspace path. Collect more now, trim later if unused.

**Rationale**: Can't backfill data we didn't collect. Better to have unused fields than missing data.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 1

**Related Questions**: Q3,Q6

---

## D7: Memory content format is markdown — 2026-03-04

**Context**: Whether memory content should be plain text or markdown

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Markdown. Standard format for LLM context injection — models are trained on it, adds minimal token overhead, improves readability.

**Rationale**: LLMs handle markdown natively. No downside vs plain text.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 1

**Related Questions**: [Questions not specified]

---

## D8: Supabase for storage (Postgres + pgvector) — 2026-03-04

**Context**: Whether to use SQLite + Chroma (two stores) or Supabase (single store)

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Use Supabase. Single store for metadata, FTS, and vector search. Eliminates dual-store sync.

**Rationale**: Hybrid queries in one SQL statement. Free tier sufficient for MVP. User confirmed hosted services are acceptable.

**Implications**:
Revises D1 storage backend. No Chroma dependency. Single query for filtered vector search. Need Supabase project setup.

**Stories Affected**: Story 1, Story 3

**Related Questions**: Q7

---

## D9: Voyage AI voyage-code-3 for embeddings — 2026-03-04

**Context**: Which embedding model to use for vector search

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Voyage AI voyage-code-3. Code-optimized embeddings generated via API, stored in Supabase pgvector.

**Rationale**: Purpose-built for code retrieval. Memories are about code patterns — code-specific model outperforms generic ones. No need to split by content type.

**Implications**:
Voyage AI API dependency. Need to determine vector dimensions for pgvector column. Embedding cost per memory write.

**Stories Affected**: Story 1, Story 2, Story 3

**Related Questions**: [Questions not specified]

---

## D10: Add title field and memory_type enum — 2026-03-04

**Context**: Schema completeness review

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Add title (short scannable name for UI). Add memory_type enum: guidance (positive, ranks higher) and anti_pattern (negative, ranks lower).

**Rationale**: Title improves Web UI scanning. memory_type enables ranking differentiation — positive guidance should surface before warnings about what not to do.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 1, Story 3, Story 7

**Related Questions**: [Questions not specified]

---

## D11: Quality gate: AI-only, humans bypass — 2026-03-04

**Context**: Whether LLM quality gate applies to all memory sources

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Quality gate applies only to AI-created memories. Human-created memories bypass the gate. Web UI shows possible duplicates as user types (heavily debounced).

**Rationale**: Humans have already curated their input. AI needs filtering for noise. Duplicate suggestions in UI prevent duplicates without blocking.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 2, Story 7

**Related Questions**: Q2

---

## D12: Async embedding generation — 2026-03-04

**Context**: Whether to generate Voyage AI embeddings sync or async at write time

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Async. Fire and forget from agent's perspective. Memory is stored immediately, embedding generated in background.

**Rationale**: Agents shouldn't block on embedding generation. Memory becomes vector-searchable shortly after creation but metadata-searchable immediately.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 2

**Related Questions**: [Questions not specified]

---

## D13: LLM-driven duplicate detection with four outcomes — 2026-03-04

**Context**: How to handle semantically similar existing memories at write time

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Ingest LLM performs similarity search before storing. Four outcomes: discard incoming (true duplicate), replace with incoming (better version), update existing from incoming (merge new details), or merge into new combined memory.

**Rationale**: Simple threshold-based dedup is too crude. LLM can make nuanced decisions about whether memories complement, supersede, or duplicate each other.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 2

**Related Questions**: [Questions not specified]

---

## D14: Quality gate uses cheap model via OpenRouter — 2026-03-04

**Context**: Which LLM to use for quality gate and ingest processing

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Use a small/cheap model via OpenRouter for quality gate and duplicate detection. Not the same model the agent uses.

**Rationale**: Quality gate is a filtering task, not a creative task. Cheaper model keeps per-write cost low. OpenRouter provides model flexibility.

**Implications**:
OpenRouter API dependency. Need to choose specific model (e.g., Haiku, Gemma). Quality gate prompt must work well with smaller models.

**Stories Affected**: Story 2

**Related Questions**: [Questions not specified]

---

## D15: Multi-hook retrieval integration model — 2026-03-04

**Context**: How and when memories are surfaced to agents

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Five hook integration points plus agent-initiated MCP calls: SessionStart (project-level critical memories), UserPromptSubmit (blocking quick check + async deep search with mid-run injection via AsyncIterable<SDKUserMessage>), PreToolUse (check writes against anti-patterns), PostToolUse (check what was written), Stop (final critical error check). Agent Teams monitor pattern for proactive surfacing.

**Rationale**: Different hook points provide different context levels and urgency. Layered approach ensures memories surface at the right moment without blocking the agent unnecessarily.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 3, Story 5, Story 6

**Related Questions**: Q1

---

## D16: Two-tier result budgeting — 2026-03-04

**Context**: How many memories to return and how to handle overflow

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Return top 5 full memories. For additional high-relevancy matches, return summary + relevancy score + cache key for next 10-20. Consuming agent decides which to expand.

**Rationale**: Respects context window budget while not discarding potentially relevant memories. Agent has agency to pull more detail on demand.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 3

**Related Questions**: Q1

---

## D17: Agent Teams monitor pattern (experimental) — 2026-03-04

**Context**: How to proactively surface memories during multi-agent work

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: FixOnce monitor agent watches other agents via Agent Teams peer-to-peer messaging. Can proactively SendMessage relevant memories. File watcher tool or PostToolUse hooks on builder agents notify monitor of changes.

**Rationale**: Agent Teams enable proactive memory surfacing without polling. Monitor has its own context window so doesn't burden implementing agents.

**Implications**:
Agent Teams is experimental. File watcher mechanism needed. PostToolUse hook on Write/Edit as notification trigger.

**Stories Affected**: Story 3

**Related Questions**: [Questions not specified]

---

## D18: Version predicates: jsonb with version arrays, OR within AND across — 2026-03-04

**Context**: How to structure version_predicates for efficient querying

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: jsonb object where keys are component names from the Midnight support matrix (12 components) and values are arrays of version strings. OR logic within a component, AND logic across components. Missing key = no constraint. Query uses JSONB ? operator with GIN index.

**Rationale**: Array-of-versions is simpler than semver ranges given the small number of versions. JSONB ? operator is GIN-indexable for efficient queries. Matches the support matrix structure directly.

**Implications**:
Environment detection must resolve to specific version strings (not ranges). 12 possible component keys defined by support matrix.

**Stories Affected**: Story 4, Story 1, Story 3

**Related Questions**: Q3,Q6

---

## D19: Feedback model replaces disable/flag — 2026-03-04

**Context**: How agents report on memory quality and how bad memories get surfaced to humans

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: fixonce_feedback tool replaces fixonce_disable_memory and fixonce_flag_memory. Agents provide text feedback + tags (helpful, not_helpful, damaging, accurate, somewhat_accurate, somewhat_inaccurate, inaccurate, outdated) + suggested action (keep, remove, fix). Memories with any remove/fix feedback are flagged for immediate human review in Web UI. Memories with feedback are de-ranked but still surfaced with a warning.

**Rationale**: Richer signal than binary flag. Multiple agents can provide independent feedback. Escalation to human review is natural. Preserves memory for review rather than silently disabling.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 5, Story 7

**Related Questions**: [Questions not specified]

---

## D20: Configurable query pipeline and verbosity levels — 2026-03-04

**Context**: How flexible should fixonce_query be

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: fixonce_query accepts: pipeline options (rewrite=bool, type=simple|vector|hybrid, rerank=bool), filters (tags, language, project, dates, etc.), context/query string, max_results (default 5) OR max_tokens (approx token budget), verbosity (small|medium|large). Small=vital info only, medium=adds tags/dates/created_by, large=everything.

**Rationale**: Agents need control over pipeline cost vs quality tradeoff. Token budget option respects context window. Verbosity presets simpler than field selection for agent reasoning.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 5

**Related Questions**: [Questions not specified]

---

## D21: Merge fixonce_list_memories into fixonce_query — 2026-03-04

**Context**: Whether to have separate list and query tools

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: Single fixonce_query tool. With rewrite=false, type=simple, rerank=false it acts as a filtered list. No separate list tool needed.

**Rationale**: Avoids agent confusion about which tool to use. Same underlying functionality with different defaults.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 5

**Related Questions**: [Questions not specified]

---

## D22: CLI design: pipe support, human-readable default, separate servers — 2026-03-04

**Context**: CLI interface design decisions

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: CLI supports piping for create (stdin). Default output is human-readable with --json flag for machine consumption. fixonce serve (MCP) and fixonce web (Web UI) are separate commands/processes. Rename fixonce env to fixonce detect to avoid confusion with environment variable configuration.

**Rationale**: Pipe support enables hook scripting. Human-readable default serves developer UX. Separate servers allow independent deployment. 'detect' is clearer than 'env' for version detection.

**Implications**:
[Implications not provided]

**Stories Affected**: Story 6

**Related Questions**: [Questions not specified]

---

## D23: Web UI: React + Vite, 6 views, bulk ops deferred — 2026-03-04

**Context**: Web UI tech stack and page structure

**Question**: [Question not provided]

**Options Considered**:
[Options not provided]

**Decision**: React + Vite for local web app. Six views: Dashboard (with flagged memories prominent), Memory Query (GUI for same query options as CLI/MCP), Memory Detail (view/edit), Create Memory (with live duplicate suggestions), Recent Feedback (filterable good/bad), Recent Activity (realtime stream of queries/updates/creates, filterable). Bulk operations deferred to post-v1.

**Rationale**: React + Vite is lightweight and fast for local dev. Dashboard-first flagged memories ensures urgent items are seen immediately. Activity stream provides observability into what agents are doing.

**Implications**:
Need realtime updates for activity stream (WebSocket or SSE from fixonce web server). Activity logging must be added to all operations.

**Stories Affected**: Story 7

**Related Questions**: [Questions not specified]

---
