# FixOnce v2 Vision

## The Idea

A shared memory layer for LLM coding agents that captures corrections, gotchas, best practices, and discoveries from coding sessions and surfaces them contextually. Memories are alive — they decay, reinforce, compete, and self-correct over time. Every mistake becomes institutional memory; every session makes the system smarter.

## Problem Space

LLM coding agents operate in isolated sessions with no persistent memory. Every new session starts from zero, unaware of corrections and lessons from previous sessions. This creates a costly cycle of:

- **Repeated mistakes** — agents hit the same Compact compiler errors, SDK version incompatibilities, and anti-patterns session after session
- **Siloed knowledge** — one developer's hard-won discovery stays locked in their session history
- **Version-sensitive errors** — breaking changes in the Midnight ecosystem create waves of stale advice that agents can't distinguish from current best practices
- **No compounding** — knowledge doesn't get better with use; it doesn't get worse with age; it just sits there

FixOnce v1 proved the concept as a TypeScript monorepo with a CLI, MCP server, web dashboard, and Claude Code hooks. The architecture hit a wall: client-side API key storage doesn't scale to teams, the monorepo structure limited deployment flexibility, and the lack of proper auth blocked the path from internal tool to ecosystem resource.

## Core Value Proposition

**A memory system that converges on what actually works.** Not a static knowledge base — a living system where memories compete, reinforce, decay, and replace each other. Bad information fades out. Good information gets stronger. Contradictions resolve themselves through use. Over time, the system knows more than any individual developer or agent session.

## Key Features (v1 Scope)

### Backend
- **Supabase** for storage, serverless edge functions, and authentication
- **GitHub OAuth** with org/team-based access restrictions — not every GitHub user gets in
- **Row Level Security (RLS)** — database is never directly queryable; all operations go through edge functions
- **Server-side encrypted secrets** — API keys (VoyageAI, etc.) stored encrypted in the database, decrypted on-demand via authenticated edge function requests; the CLI never writes secrets to disk
- **Multi-tenant architecture** from day one, even with a single initial tenant

### CLI (Rust)
- **Single binary distribution** — no runtime dependencies, fast startup, easy installation
- **Public-key authentication** — CLI generates a keypair; authenticated users register public keys to their account (inspired by VibeTea's auth model)
- **Full CRUD operations** on memories
- **Dual output formats** — human-readable text and structured JSON/TOON for agent consumption
- **Environment detection** — scan a project for Midnight component versions, SDK versions, compiler versions
- **Context gathering** — command to collect relevant project context for memory creation
- **Session transcript analysis** — parse Claude Code session logs and propose candidate memories (passive harvesting, CLI-initiated)
- **Fault tolerant and highly performant** — Rust handles all non-inference operations at native speed; LLM calls are the known slow path

### Memory Model
- **Rich Midnight-specific metadata** — Compact pragma version, compiler version, midnight-js SDK version, and other ecosystem-specific version fields as contextual dimensions
- **Provenance tracking** — where the memory came from (PR URL, GitHub repo URL, task summary, session ID, agent feedback)
- **Anti-memories** — first-class "do NOT do this" artifacts with their own embeddings, version constraints, and decay behavior; surfaced proactively when an agent appears headed toward a known mistake
- **Hybrid decay** — base temporal decay (unused memories fade on a schedule) accelerated by events (new SDK release invalidates version-pinned memories)
- **Reinforcement** — memories accessed frequently and receiving positive agent feedback grow stronger; memories flagged as harmful or outdated decay faster
- **Memory lineage** — every memory carries its full history: what it replaced, what merged into it, what feedback changed it; lineage is stored as metadata and returned only on explicit request, not in standard query responses
- **Contradiction courts** — when two memories disagree, the system flags the tension; the next agent session encountering both acts as tiebreaker by trying approaches and reporting results; the losing memory decays faster
- **Memory signatures** — pre-computed relevance fingerprints based on correlated files, error patterns, and SDK method calls; when an agent starts a session and runs environment detection, FixOnce pre-computes a session relevance profile and caches a hot set of likely-relevant memories

### Read Pipelines
The CLI supports the full menu of RAG techniques, composable per query:

**Query Techniques:**
- Query Rewriting
- Multi-Query Generation
- Step-Back Queries
- HyDE (Pseudo-Answer Generation)
- Decomposition
- Retrieve-Read-Retrieve
- Query Refinement
- Contradiction Detection

**Result Refinement:**
- Confidence Assessment
- Relevance Reranking
- Trust-Aware Reranking
- Freshness Reranking
- Deduplication
- Coverage Balancing
- Answerability Scoring

**Search Modes:**
- Hybrid Search (full-text + semantic via Reciprocal Rank Fusion)
- Metadata Filtering
- Graph-Assisted Retrieval
- Parent-Child Retrieval
- Field-Aware Retrieval
- Passage Compression

All inference in the pipeline uses Claude via `claude -p --output-format json`. Claude is a hard dependency — no local/offline fallback.

### Write Pipelines
- Quality gating with credential/PII detection
- LLM-powered deduplication with 5+ outcomes (new, discard, replace, update, merge)
- Automatic metadata enrichment
- Anti-memory generation from negative feedback patterns

### Web Dashboard
- System administration
- Memory search and browsing
- Memory creation and editing
- Feedback submission
- Real-time activity stream (SSE)
- Static frontend deployed to Netlify; local dev server for development

### Claude Code Hooks
- Session start: detect environment, surface critical memories
- User prompt: quick-search for relevant memories
- Pre/post tool use: warn (not block) if anti-pattern matched
- Session end: surface final reminders
- All interventions are **advisory only** — the system warns, never blocks

### Embeddings
- VoyageAI voyage-code-3 for code-specialized embeddings
- CLI requests the VoyageAI API key from the encrypted secrets endpoint, generates embeddings locally, then discards the key

## Deferred Features

- **Memory Constellations** — emergent clustering of related memories with synthesized views; memories that "know they belong together" and can return a mini-guide assembled from the cluster rather than individual results
- **MCP server** — direct Claude Code tool integration (v1 had this; v2 defers it to focus on CLI + hooks as the primary interface)
- **Automatic hook-based passive harvesting** — hooks that observe sessions in real-time and propose memories without CLI intervention (v1 has CLI-only transcript analysis)

## Out of Scope / Anti-Goals

- **Hard-blocking agent actions** — FixOnce advises, never controls; warn-only intervention
- **Generic multi-ecosystem schema** — the metadata model is Midnight-specific; if the system expands to other ecosystems, schema migration is a future problem
- **Local/offline LLM fallbacks** — Claude is required; no degraded mode without it
- **Incremental/degraded releases** — the full pipeline ships together; no "fast path" subset released early
- **Pluggable LLM backends** — Claude via `claude -p` is the inference mechanism; no abstraction layer for swapping providers

## Open Questions

- **Decay rate calibration** — what's the right temporal half-life for an unused memory? Should it differ by memory type (gotcha vs. best practice vs. anti-memory)?
- **Contradiction court quorum** — how many agent sessions need to weigh in before a contradiction is considered resolved? One decisive result, or statistical significance over multiple sessions?
- **Signature invalidation** — when a session's pre-computed memory signature becomes stale (e.g., memories are created mid-session that would be relevant), how aggressively should the cache refresh?
- **Harvesting signal-to-noise** — when analyzing session transcripts, what distinguishes a "learned something worth capturing" moment from routine debugging? How do we avoid proposing memories for every minor fix?
- **Multi-tenant isolation** — when FixOnce opens to ecosystem developers, do teams share the full memory store or maintain private pools? Can memories be "published" from a team's private pool to the shared store?
- **Secret rotation** — the encrypted secrets endpoint handles API keys; what's the rotation and revocation model?

## Inspirations & Analogies

- **Paleontology (stratigraphy)** — memory lineage as a fossil record; every memory carries its geological context, what it replaced, what was above and below it in the knowledge timeline
- **Agriculture (seasonal rhythms)** — tying memory lifecycle to release cycles; harvest phases after SDK updates where version-pinned memories get verified or culled
- **Immunology (active/archival)** — the two-tier memory model inspiration; active memories in the hot path, archival memories dormant but ready to reactivate when their pattern matches
- **Music production (frequency signatures)** — memory signatures as harmonic fingerprints; pre-computed relevance profiles that match sessions to memories before any query is made
- **Astronomy (constellations)** — the deferred clustering feature; individual memories that form emergent groups based on co-access and embedding proximity
- **Winemaking (terroir)** — the insight that the same memory means different things in different contexts; context-dependent relevance weighting (explored but not adopted for v1)
