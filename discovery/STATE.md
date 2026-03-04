# Discovery State: fixonce-memory-layer

**Updated**: 2026-03-04 13:30 UTC
**Iteration**: 9
**Phase**: Complete

---

## Problem Understanding

### Problem Statement
LLM coding agents operate in isolated sessions with no persistent memory. Every new session starts from zero — unaware of corrections, gotchas, and lessons from previous sessions. This creates a costly cycle of repeated mistakes, siloed knowledge, and version-sensitive errors that agents can't track. FixOnce is a shared memory layer that captures corrections and discoveries, surfaces them contextually to any agent on the team, and improves over time through reinforcement — turning every mistake into institutional memory.

### Personas
| Persona | Description | Primary Goals |
|---------|-------------|---------------|
| Agent (LLM coding agent) | An LLM-powered coding agent (e.g., Claude Code) working on a codebase | Receive relevant memories before/during work; contribute new memories from corrections and discoveries |
| Human Developer | A developer who works alongside agents and reviews their output | Correct agent mistakes (creating memories); curate and manage the memory store; configure memory scoping |
| DevRel / Ecosystem Maintainer | Maintains ecosystem tooling (e.g., Midnight Network DevRel team) | Capture institutional knowledge about ecosystem gotchas; share curated knowledge with community (deferred) |

### Current State vs. Desired State
**Today (without feature)**: Agents start every session from scratch. Human reviewers leave the same PR feedback repeatedly. Version-specific gotchas (e.g., Midnight's Compact language changes between versions) must be manually communicated each time. Knowledge lives in individual heads or scattered docs.

**Tomorrow (with feature)**: An agent starting work on a Midnight project automatically receives relevant memories: "Compact 0.18 doesn't support X," "This project uses camelCase," "We tried approach Y and it failed." Corrections compound across sessions and team members. Humans curate through a web UI, and low-value memories decay naturally.

### Constraints
- First target ecosystem: Midnight Network (Compact language, compiler, runtime version coupling)
- Must not replace static rule systems (CLAUDE.md, linters, .editorconfig) — complementary
- Must not become a general-purpose vector database or documentation system
- Memory retrieval adds latency to agent startup — must be acceptable
- Surfaced memories consume agent context window tokens — must be budgeted

---

## Story Landscape

### Story Status Overview
| # | Story | Priority | Status | Confidence | Key Decisions |
|---|-------|----------|--------|------------|---------------|
| 1 | Memory Storage & Schema | P1 | ✅ In SPEC | 100% | D1, D5-D10 |
| 2 | Memory Creation (Write Path) | P1 | ✅ In SPEC | 100% | D11-D14 |
| 3 | Memory Retrieval (Read Path) | P1 | ✅ In SPEC | 100% | D15-D17 |
| 4 | Version-Scoped Metadata | P1 | ✅ In SPEC | 100% | D18 |
| 5 | MCP Server Interface | P1 | ✅ In SPEC | 100% | D19-D21 |
| 6 | CLI Interface | P2 | ✅ In SPEC | 100% | D22 |
| 7 | Web UI for Memory Management | P2 | ✅ In SPEC | 100% | D23 |

### Story Dependencies
```
Story 1: Memory Storage & Schema
  └──> Story 2: Memory Creation (Write Path)
  └──> Story 3: Memory Retrieval (Read Path)
  └──> Story 4: Version-Scoped Metadata
         └──> (integrates into Stories 2 & 3)
  └──> Story 5: MCP Server Interface (exposes Stories 2 & 3)
  └──> Story 6: CLI Interface (exposes Stories 2 & 3)
  └──> Story 7: Web UI (exposes Stories 1, 2, 3)
```

### Deferred Proto-Stories (post-v1)
- **Reinforcement Scoring & Lifecycle** — confidence decay, flagging (D4)
- **Contradiction Detection & Resolution** — reranker conflict handling (D4)
- **Memory Composition & Clustering** — coherent guidance clusters (D4)
- **Team-Scoped Sharing** — multi-user/multi-agent sharing (D3)
- **Bulk Operations in Web UI** — multi-select disable/delete/tag (D23)
- **Exportable Memory Packs** — curated collections for community sharing

---

## Completed Stories Summary

| # | Story | Priority | Completed | Key Decisions | Revision Risk |
|---|-------|----------|-----------|---------------|---------------|
| 1 | Memory Storage & Schema | P1 | 2026-03-04 | D1, D5-D10 | Low |
| 2 | Memory Creation (Write Path) | P1 | 2026-03-04 | D11-D14 | Low |
| 3 | Memory Retrieval (Read Path) | P1 | 2026-03-04 | D15-D17 | Medium (Agent Teams experimental) |
| 4 | Version-Scoped Metadata | P1 | 2026-03-04 | D18 | Low |
| 5 | MCP Server Interface | P1 | 2026-03-04 | D19-D21 | Low |
| 6 | CLI Interface | P2 | 2026-03-04 | D22 | Low |
| 7 | Web UI for Memory Management | P2 | 2026-03-04 | D23 | Low |

*Full stories in SPEC.md*

---

## Glossary

- **Memory**: A single unit of experiential knowledge — a correction, gotcha, or guideline — stored and retrievable by agents
- **Quality Gate**: LLM-based evaluation at write time that filters out trivial or low-value memories before storage
- **Feedback**: Agent-provided signal on memory quality (tags + suggested action + text)
- **Version Predicate**: A jsonb object mapping Midnight component keys to arrays of applicable version strings
- **Reranker**: The final LLM pass in retrieval that consolidates, deduplicates, and resolves contradictions among candidate memories
- **Compact**: The smart contract language for the Midnight Network blockchain
- **MCP (Model Context Protocol)**: Protocol for LLM agents to interact with external tools and data sources
- **Agent Teams**: Experimental Claude Code feature for coordinating multiple agent instances with peer-to-peer messaging
- **Activity Log**: Record of all FixOnce operations for the Web UI activity stream
