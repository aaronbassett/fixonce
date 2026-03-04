# FixOnce Vision

## The Idea

FixOnce is a shared memory layer for LLM-powered coding agents. When an agent gets corrected, discovers a gotcha, or receives explicit guidance, that lesson is stored and automatically surfaced to any agent on the team facing a similar situation. Fix something once, never fix it again.

## Problem Space

LLM coding agents operate in isolated sessions. Every new session starts from zero — unaware of mistakes made yesterday, corrections given last week, or hard-won lessons from last month. This creates a frustrating cycle:

- **Repeated corrections**: Human reviewers leave the same PR feedback over and over because agents don't retain lessons across sessions.
- **Knowledge silos**: One agent session's breakthrough doesn't help the next session, even on the same project.
- **Version-sensitive gotchas**: In ecosystems like Midnight Network, tight coupling between compiler versions, language versions, and runtime creates a combinatorial explosion of "gotchas" that agents can't track without persistent memory.

The first target ecosystem is **Midnight Network**, where strict compatibility requirements between Compact language versions, the compiler, and on-chain runtime make version-aware memory especially valuable. What's an error in Compact 0.18 might be perfectly valid in 0.21 — and vice versa.

## Core Value Proposition

An agent-first experiential knowledge base that grows organically from corrections and discoveries, surfaces the right lessons at the right moment through intelligent retrieval, and improves over time through reinforcement — turning every mistake into institutional memory.

## Key Features (v1 Scope)

### Three Interfaces
- **MCP Server**: Primary agent interface for reading and writing memories programmatically.
- **CLI**: For scripting and automation — can be called from hooks (e.g., Claude Code hooks) to store or retrieve memories.
- **Web UI**: Local web application for human management — review, edit, flag, disable, and curate memories.

### Intelligent Memory Creation
- Memories are created from three sources: human corrections, agent self-discovery, and explicit instructions.
- **LLM quality gate** at write time evaluates proposed memories before storage — rejects trivial observations, overly specific notes, or low-value entries.
- Memories carry **provenance metadata**: source type, version context, and environment details.

### Version-Scoped Metadata
- Memories carry explicit version predicates (e.g., `compact >= 0.18 AND compact < 0.21 AND compiler ~0.3.x`).
- When the environment changes (SDK upgrade, compiler update), memory relevance shifts automatically.
- The ecosystem's support matrix serves as a contextual reference for memory applicability.

### Hybrid Retrieval Pipeline
Three-stage retrieval ensures agents receive only the most relevant memories:
1. **Query rewriting**: A dedicated LLM reformulates the agent's current context into effective search queries.
2. **Hybrid search**: Structured metadata for coarse filtering (language, version, tags) combined with vector similarity for semantic matching.
3. **Reranking**: An LLM pass consolidates results, resolves contradictions between conflicting memories, and surfaces only the most relevant subset.

### Memory Composition and Clustering
Related memories are surfaced together as coherent clusters. Individual memories like "validate inputs at API boundaries," "use zod for validation," and "this project uses tRPC" compose into richer guidance than any single memory provides alone.

### Contradiction Detection and Resolution
When memories conflict (e.g., an older memory recommends `Uint<64>` while a newer one recommends `Uint<128>`), the reranking agent detects the contradiction and surfaces only the most applicable memory with context.

### Reinforcement Scoring and Lifecycle
- Memories have confidence scores that increase when memories are surfaced and followed, decrease when surfaced but ignored.
- Memories below a threshold score are flagged in the web UI for human review — the system highlights its own weeds for the gardener.
- Humans can immediately disable or correct any memory through the web UI (kill switch), while gradual decay handles slow drift.

### Team-Scoped Sharing
Memories are shared across all agents within a team or organization. One agent's lesson immediately benefits every agent on the team.

## Deferred Features

- **Exportable memory packs**: Curated collections of memories that can be exported and imported into other FixOnce instances — enabling the Midnight DevRel team to share institutional knowledge with the wider developer community without allowing direct public edits.
- **Automatic feedback loops**: Tracking whether surfaced memories led to successful outcomes (code passed review, tests passed) to close the reinforcement loop automatically rather than relying on indirect signals.

## Out of Scope / Anti-Goals

- **Not a rules engine**: FixOnce does not replace CLAUDE.md files, linter configs, or `.editorconfig`. It is complementary — handling experiential, contextual knowledge that is too nuanced for static rule files. Memories may "graduate" into static rules over time, but FixOnce is the proving ground, not the rulebook.
- **Not a documentation system**: FixOnce is not a wiki, knowledge base, or documentation platform. It stores actionable agent memories, not reference material.
- **Not a logging or analytics tool**: FixOnce is not for tracking or measuring agent behavior. It exists to improve future behavior, not report on past behavior.
- **Not a general-purpose vector database**: FixOnce is not a generic RAG system. It is purpose-built for the specific pattern of agents learning from corrections and surfacing those lessons contextually.

## Open Questions

- **Context window budget**: Surfaced memories consume agent context window tokens. How do we balance comprehensive retrieval with not eating the agent's working memory? The reranker helps, but there may need to be explicit token budgets or compression strategies.
- **Quality gate calibration**: The write-time LLM quality gate needs to distinguish genuine lessons from trivial observations. Too strict and valuable memories get rejected; too loose and noise accumulates. How is this tuned over time?
- **Memory schema design**: What structured fields does a memory need beyond content? Version predicates, tags, source type, confidence score, creation date, last-surfaced date, reinforcement count — finding the right schema is critical.
- **Cross-project applicability**: Some memories are project-specific ("our API uses camelCase"), others are ecosystem-wide ("Compact doesn't support string concatenation"). How does FixOnce handle memories at different levels of generality?
- **Retrieval latency**: The three-stage retrieval pipeline (query rewriting, hybrid search, reranking) involves multiple LLM calls. How fast does this need to be, and what's acceptable latency for an agent waiting for memories before starting work?

## Inspirations and Analogies

- **Immune system (biology)**: Like antibodies that bind to specific antigens, memories pattern-match on specific code contexts. The first encounter is painful; every subsequent encounter triggers an immediate, targeted response.
- **Mise en place (cooking)**: Like a chef's station setup before service, FixOnce proactively assembles the relevant memories before the agent starts working — a pre-task briefing rather than a manual to search through.
- **Memory decay and reinforcement (neuroscience)**: Memories that aren't reinforced naturally lose confidence, while consistently useful memories strengthen — creating a self-curating quality signal.
- **Oral tradition (anthropology)**: FixOnce is an oral tradition for agents — experiential knowledge passed between sessions and across team members, refined through use, with the future possibility of sharing curated collections with the broader community.
- **Spaced repetition (learning science)**: Memories surface at the moment of relevance, not as a bulk dump — like a mentor who speaks up at exactly the right time rather than handing you a manual.
- **Negative space (art)**: Anti-pattern memories ("we tried X and it was a disaster") are as valuable as positive guidance — the agent knows which paths are mined, not just which path to take.
- **Chord progressions (music)**: Individual memories compose into richer guidance when clustered — like notes forming chords, the combination creates meaning that no single memory carries alone.
