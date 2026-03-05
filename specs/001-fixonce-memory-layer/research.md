# Research: Technical Unknowns

Resolution of all technical unknowns identified in spec-analysis.md, with decisions, rationale, and implementation notes.

---

## 1. Claude Code Hooks SDK (TypeScript)

### Decision

Use `@anthropic-ai/claude-agent-sdk` with programmatic TypeScript hooks. All five required hook events (SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop) are supported in the TypeScript SDK.

### Key Findings

**Package**: `@anthropic-ai/claude-agent-sdk` (npm)

**Available Hook Events (TypeScript SDK)**:

| Hook Event | Supported | Matcher Target | Blocking? |
|---|---|---|---|
| `SessionStart` | Yes (TS only) | N/A | Yes |
| `UserPromptSubmit` | Yes | N/A | Yes |
| `PreToolUse` | Yes | Tool name (regex) | Yes (can allow/deny/ask) |
| `PostToolUse` | Yes | Tool name (regex) | Yes (can add context) |
| `Stop` | Yes | N/A | Yes |
| `SubagentStart` | Yes | N/A | Yes |
| `SubagentStop` | Yes | N/A | Yes |
| `Notification` | Yes | Notification type | No (side effect) |

**Hook Registration Pattern**:

```typescript
import { query, HookCallback, PreToolUseHookInput } from "@anthropic-ai/claude-agent-sdk";

const myHook: HookCallback = async (input, toolUseID, { signal }) => {
  // input contains: hook_event_name, session_id, cwd, + event-specific fields
  // Return {} to allow, or hookSpecificOutput to modify behavior
  return {};
};

for await (const message of query({
  prompt: "...",
  options: {
    hooks: {
      PreToolUse: [{ matcher: "Write|Edit", hooks: [myHook] }],
      PostToolUse: [{ hooks: [anotherHook] }],
      SessionStart: [{ hooks: [sessionStartHook] }],
      UserPromptSubmit: [{ hooks: [promptHook] }],
      Stop: [{ hooks: [stopHook] }],
    }
  }
})) {
  // process messages
}
```

**Hook Callback Outputs**:

- `{}` — allow operation without changes
- `{ hookSpecificOutput: { permissionDecision: "deny", permissionDecisionReason: "..." } }` — block (PreToolUse)
- `{ hookSpecificOutput: { updatedInput: {...}, permissionDecision: "allow" } }` — modify input (PreToolUse)
- `{ hookSpecificOutput: { additionalContext: "..." } }` — append context (PostToolUse, UserPromptSubmit)
- `{ systemMessage: "..." }` — inject a system message into the conversation
- `{ async: true }` — non-blocking side effect (logging, webhooks)

**Mid-Run Message Injection (Story 3, Scenario 3) — CONFIRMED AVAILABLE**:

The `query()` function returns a `Query` object with a `streamInput(stream: AsyncIterable<SDKUserMessage>)` method. This enables mid-run injection of messages into an active session.

The `prompt` parameter itself also accepts `AsyncIterable<SDKUserMessage>` for streaming input mode.

This confirms the spec's design for `UserPromptSubmit` doing a blocking quick check + async deep search with results injected mid-run. The pattern is:

1. `UserPromptSubmit` hook fires, returns quick results synchronously via `additionalContext`
2. In the background, start deep search
3. Use `query.streamInput()` to inject deep search results as they become available

### Alternatives Considered

- **Shell command hooks** (`.claude/settings.json`): Simpler but less flexible. Cannot access TypeScript types, cannot use `streamInput()` for mid-run injection. Only suitable for simple pre/post scripts.
- **V2 SDK interface** (`send()` / `stream()` patterns): Available as preview. May simplify multi-turn patterns but is not yet stable. Stick with `query()` for v1.

### Implementation Notes

- `SessionStart` is TypeScript-only (not available in Python SDK)
- Matchers only filter by tool name, not file paths. Path filtering must be done inside the callback.
- Multiple hooks execute in array order. Use separate matchers for different responsibilities.
- Hook timeout defaults to 60 seconds; configurable per matcher.
- The `signal` (AbortSignal) in the context should be passed to any fetch calls for cancellation support.

### Sources

- [Claude Agent SDK Hooks Guide](https://platform.claude.com/docs/en/agent-sdk/hooks)
- [Claude Agent SDK TypeScript Reference](https://platform.claude.com/docs/en/agent-sdk/typescript)
- [Claude Code Hooks Guide](https://code.claude.com/docs/en/hooks-guide)

---

## 2. MCP Server Implementation (TypeScript)

### Decision

Use `@modelcontextprotocol/sdk` with `McpServer` class and Zod v4 for tool schema validation. Transport via stdio for local use (launched by `fixonce serve`).

### Key Findings

**Package**: `@modelcontextprotocol/sdk` (npm), peer dependency on `zod` (v4)

**Server Creation Pattern**:

```typescript
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const server = new McpServer({
  name: "fixonce",
  version: "1.0.0"
});

// Register a tool with Zod schema
server.registerTool(
  "fixonce_query",
  {
    title: "Query Memories",
    description: "Search fixonce memory store",
    inputSchema: z.object({
      query: z.string(),
      type: z.enum(["simple", "vector", "hybrid"]).default("hybrid"),
      max_results: z.number().default(5),
      // ... more params
    }),
  },
  async (args) => {
    // Tool handler implementation
    return {
      content: [{ type: "text", text: JSON.stringify(results) }]
    };
  }
);

// Connect transport
const transport = new StdioServerTransport();
await server.connect(transport);
```

**Transport Options**:

| Transport | Use Case | Notes |
|---|---|---|
| stdio | Local process (Claude Desktop, CLI) | Best for `fixonce serve` |
| Streamable HTTP | Remote/networked servers | Supports SSE for server-to-client notifications |

**Alternative: SDK In-Process MCP Server**

The Claude Agent SDK also provides `createSdkMcpServer()` for in-process MCP servers that run in the same process as the SDK application:

```typescript
import { createSdkMcpServer, tool } from "@anthropic-ai/claude-agent-sdk";

const fixonceTool = tool(
  "fixonce_query",
  "Search fixonce memory store",
  { query: z.string() },
  async (args) => ({ content: [{ type: "text", text: "..." }] })
);

const mcpServer = createSdkMcpServer({
  name: "fixonce",
  tools: [fixonceTool]
});
```

This is useful for integration testing or when fixonce is embedded directly in an SDK application rather than run as a separate process.

### Alternatives Considered

- **fastmcp**: Higher-level wrapper over the MCP SDK. Adds convenience but is a third-party dependency. Not needed given the official SDK is straightforward.
- **Custom JSON-RPC**: Building our own protocol. Unnecessary since MCP is the standard.

### Implementation Notes

- All 7 MCP tools from the spec map directly to `server.registerTool()` calls
- Zod schemas provide automatic validation and type inference
- The MCP SDK handles JSON-RPC framing, error formatting, and protocol negotiation
- Tool handlers receive validated/typed arguments
- `CallToolResult` return type uses `content` array with `{ type: "text", text: string }` entries

### Sources

- [MCP TypeScript SDK - Server Docs](https://github.com/modelcontextprotocol/typescript-sdk/blob/main/docs/server.md)
- [@modelcontextprotocol/sdk on npm](https://www.npmjs.com/package/@modelcontextprotocol/sdk)

---

## 3. Supabase pgvector Setup

### Decision

Use Supabase Postgres with pgvector extension for unified storage. Hybrid search implemented as a single SQL RPC function using Reciprocal Rank Fusion (RRF) to merge tsvector FTS and pgvector semantic results.

### Key Findings

**Enabling pgvector**: pgvector is available as a Supabase extension. Enable via dashboard or SQL:

```sql
create extension if not exists vector;
```

**Table Schema** (memories table, relevant columns):

```sql
-- tsvector column: auto-generated from multiple fields
fts tsvector generated always as (
  to_tsvector('english',
    coalesce(title, '') || ' ' ||
    coalesce(content, '') || ' ' ||
    coalesce(summary, '') || ' ' ||
    coalesce(array_to_string(tags, ' '), '')
  )
) stored;

-- Vector column
embedding vector(1024)
```

**Hybrid Search Function (RRF)**:

```sql
create or replace function hybrid_search(
  query_text text,
  query_embedding vector(1024),
  match_count int,
  full_text_weight float default 1.0,
  semantic_weight float default 1.0,
  rrf_k int default 50
)
returns setof memories
language sql
as $$
with full_text as (
  select id,
    row_number() over(
      order by ts_rank_cd(fts, websearch_to_tsquery(query_text)) desc
    ) as rank_ix
  from memories
  where fts @@ websearch_to_tsquery(query_text)
    and enabled = true
  limit least(match_count, 30) * 2
),
semantic as (
  select id,
    row_number() over(
      order by embedding <=> query_embedding
    ) as rank_ix
  from memories
  where enabled = true
  limit least(match_count, 30) * 2
)
select memories.*
from full_text
full outer join semantic on full_text.id = semantic.id
join memories on coalesce(full_text.id, semantic.id) = memories.id
order by
  coalesce(1.0 / (rrf_k + full_text.rank_ix), 0.0) * full_text_weight +
  coalesce(1.0 / (rrf_k + semantic.rank_ix), 0.0) * semantic_weight
  desc
limit least(match_count, 30)
$$;
```

**Index Setup**:

```sql
-- GIN index for tsvector full-text search
create index idx_memories_fts on memories using gin(fts);

-- HNSW index for vector similarity (cosine distance)
create index idx_memories_embedding on memories
  using hnsw (embedding vector_cosine_ops);

-- GIN index for JSONB version_predicates (? operator)
create index idx_memories_version_predicates on memories
  using gin(version_predicates);

-- GIN index for tags array
create index idx_memories_tags on memories using gin(tags);

-- Partial index for enabled filter
create index idx_memories_enabled on memories (enabled) where enabled = true;
```

**Distance Operators**:

| Operator | Distance | Index Ops Class |
|---|---|---|
| `<=>` | Cosine distance | `vector_cosine_ops` |
| `<#>` | Inner product (negative) | `vector_ip_ops` |
| `<->` | L2 distance | `vector_l2_ops` |

Use `<=>` (cosine) since we don't know if Voyage AI embeddings are normalized.

**Supabase Client (TypeScript)**:

```typescript
import { createClient } from "@supabase/supabase-js";

const supabase = createClient(
  process.env.SUPABASE_URL!,
  process.env.SUPABASE_ANON_KEY!
);

// Call hybrid search RPC
const { data, error } = await supabase.rpc("hybrid_search", {
  query_text: "how to deploy contract",
  query_embedding: embeddingVector,
  match_count: 15
});
```

### Alternatives Considered

- **SQLite + Chroma** (original D1): Two separate stores requiring sync. Rejected by D8.
- **Pinecone**: Managed vector DB but no FTS, no relational data. Would still need a separate DB.
- **Neon Postgres + pgvector**: Similar to Supabase but without the client library ecosystem and dashboard.

### Implementation Notes

- HNSW index is preferred over IVFFlat for small-to-medium datasets (no training step needed)
- The `enabled = true` filter should be in both CTEs of the hybrid search function
- Additional metadata filters (language, tags, version_predicates) can be added as WHERE clauses in the CTEs
- tsvector `generated always as stored` means the column auto-updates when source fields change
- Supabase free tier: 500MB database, 2GB bandwidth, sufficient for MVP
- Use `@supabase/supabase-js` v2+ for TypeScript

### Sources

- [Supabase Hybrid Search Guide](https://supabase.com/docs/guides/ai/hybrid-search)
- [Supabase pgvector Extension](https://supabase.com/docs/guides/database/extensions/pgvector)
- [Supabase HNSW Indexes](https://supabase.com/docs/guides/ai/vector-indexes/hnsw-indexes)

---

## 4. Voyage AI API

### Decision

Use `voyage-code-3` via the REST API at `https://api.voyageai.com/v1/embeddings` with 1024 dimensions (the default). Use the official `voyageai` npm package for TypeScript integration.

### Key Findings

**Model**: `voyage-code-3`
- Context length: 32,000 tokens
- Output dimensions: 256, 512, **1024 (default)**, 2048 (Matryoshka learning)
- Optimized for code retrieval
- Outperforms OpenAI text-embedding-3-large by ~14% on code retrieval benchmarks
- Supports output types: float, int8, uint8, binary, ubinary

**API Endpoint**: `POST https://api.voyageai.com/v1/embeddings`

**Authentication**: Bearer token via `Authorization: Bearer $VOYAGE_API_KEY`

**Request Format**:

```json
{
  "input": ["text to embed", "another text"],
  "model": "voyage-code-3",
  "input_type": "document",
  "output_dimension": 1024
}
```

`input_type` values:
- `"document"` — when storing memories (write path)
- `"query"` — when searching (read path)

**TypeScript SDK**:

```typescript
import { VoyageAIClient } from "voyageai";

const client = new VoyageAIClient({ apiKey: process.env.VOYAGE_API_KEY });

const result = await client.embed({
  input: ["memory content here"],
  model: "voyage-code-3",
  inputType: "document",
  outputDimension: 1024,
});

const embedding = result.data[0].embedding; // number[1024]
```

**Pricing**: Based on token usage (check current rates at voyageai.com/pricing). Significantly cheaper than OpenAI embeddings.

### Alternatives Considered

- **OpenAI text-embedding-3-large**: General-purpose, not code-optimized. Lower retrieval quality on code benchmarks.
- **Cohere embed-v3**: Good general embeddings but not code-specific.
- **Local models (e.g., CodeSage)**: No API cost but requires GPU infrastructure. Not suitable for a hosted SaaS approach.

### Implementation Notes

- Use `input_type: "document"` at write time and `input_type: "query"` at search time for optimal retrieval
- 1024 is the default dimension, so `output_dimension` can be omitted
- Batch up to ~128 texts per API call for efficiency
- Embedding generation should be async (don't block memory writes)
- Store as `vector(1024)` in Supabase
- Environment variable: `VOYAGE_API_KEY`

### Sources

- [Voyage AI Text Embeddings Docs](https://docs.voyageai.com/docs/embeddings)
- [voyage-code-3 Blog Post](https://blog.voyageai.com/2024/12/04/voyage-code-3/)
- [Voyage AI TypeScript SDK (npm: voyageai)](https://www.npmjs.com/package/voyageai)

---

## 5. OpenRouter API

### Decision

Use OpenRouter as the LLM provider for all cheap-model tasks (quality gate, dedup, query rewriting, reranking). Use `google/gemma-3-4b-it` as the default cheap model, with `anthropic/claude-3.5-haiku` as a higher-quality fallback for tasks where quality matters more. OpenAI SDK-compatible API.

### Key Findings

**API Endpoint**: `POST https://openrouter.ai/api/v1/chat/completions`

**Authentication**: `Authorization: Bearer $OPENROUTER_API_KEY`

**Request Format** (OpenAI-compatible):

```typescript
import OpenAI from "openai";

const openrouter = new OpenAI({
  baseURL: "https://openrouter.ai/api/v1",
  apiKey: process.env.OPENROUTER_API_KEY,
});

const response = await openrouter.chat.completions.create({
  model: "google/gemma-3-4b-it",
  messages: [
    { role: "system", content: "You are a quality evaluator..." },
    { role: "user", content: memoryContent }
  ],
});
```

**Recommended Models by Task**:

| Task | Primary Model | Fallback | Notes |
|---|---|---|---|
| Quality gate | `google/gemma-3-4b-it` | `anthropic/claude-3.5-haiku` | Binary accept/reject with reason |
| Duplicate detection | `anthropic/claude-3.5-haiku` | `google/gemma-3-12b-it` | 4-outcome nuanced decision |
| Query rewriting | `google/gemma-3-4b-it` | — | Simple reformulation |
| Reranking | `google/gemma-3-4b-it` | `anthropic/claude-3.5-haiku` | Score and sort |

**Pricing** (approximate, as of early 2026):
- Free models available (rate-limited: ~20 req/min, 200/day) — not suitable for production
- Gemma 3 4B: very cheap (fractions of a cent per request)
- Claude 3.5 Haiku: ~$0.25/M input, $1.25/M output via OpenRouter
- OpenRouter adds no markup on model pricing; charges 5.5% on credit purchases

**Free Tier Note**: Free models exist but have strict rate limits. For a tool used by developers, even the cheapest paid models are more reliable.

### Alternatives Considered

- **Direct Anthropic API**: Would lock us to Anthropic models only. OpenRouter provides model flexibility.
- **Direct Google AI API**: Cheaper for Gemma but no fallback flexibility.
- **Ollama (local)**: Zero cost but requires local GPU, adds setup complexity, inconsistent quality.
- **LiteLLM**: Similar gateway but OpenRouter is simpler (no self-hosting).

### Implementation Notes

- Use the `openai` npm package pointed at OpenRouter's base URL — zero additional dependencies
- Model names follow `provider/model-name` format
- Store model choice per task in config so users can override
- Add `X-Title: fixonce` header for OpenRouter analytics
- Environment variable: `OPENROUTER_API_KEY`
- Consider caching LLM responses for identical inputs (quality gate, dedup) to reduce cost
- All LLM calls should have timeouts (5-10s) and graceful fallbacks

### Sources

- [OpenRouter Quickstart](https://openrouter.ai/docs/quickstart)
- [OpenRouter Models](https://openrouter.ai/models)
- [OpenRouter API Reference](https://openrouter.ai/docs/api/reference/overview)

---

## 6. Project Structure

### Decision

Use **pnpm workspaces + Turborepo** with the following monorepo structure:

```
fixonce/
├── package.json              # Root: pnpm workspace + turborepo config
├── pnpm-workspace.yaml
├── turbo.json
├── tsconfig.base.json        # Shared TypeScript config
│
├── packages/
│   ├── shared/               # Shared types, schemas, constants
│   │   ├── src/
│   │   │   ├── schema.ts     # Memory, Feedback, ActivityLog types + Zod schemas
│   │   │   ├── enums.ts      # All enum types
│   │   │   └── index.ts
│   │   ├── package.json      # @fixonce/shared
│   │   └── tsconfig.json
│   │
│   ├── storage/              # Storage layer (Supabase client, migrations, embeddings)
│   │   ├── src/
│   │   │   ├── client.ts     # Supabase client wrapper
│   │   │   ├── embeddings.ts # Voyage AI integration
│   │   │   ├── search.ts     # Hybrid search functions
│   │   │   └── index.ts
│   │   ├── migrations/       # SQL migration files
│   │   ├── package.json      # @fixonce/storage
│   │   └── tsconfig.json
│   │
│   ├── pipeline/             # Write path + Read path logic
│   │   ├── src/
│   │   │   ├── write.ts      # Quality gate, dedup, store
│   │   │   ├── read.ts       # Query rewriting, search, rerank
│   │   │   ├── llm.ts        # OpenRouter client wrapper
│   │   │   └── index.ts
│   │   ├── package.json      # @fixonce/pipeline
│   │   └── tsconfig.json
│   │
│   └── activity/             # Activity logging (cross-cutting)
│       ├── src/
│       │   └── index.ts
│       ├── package.json      # @fixonce/activity
│       └── tsconfig.json
│
├── apps/
│   ├── mcp-server/           # MCP server (Story 5)
│   │   ├── src/
│   │   │   └── index.ts      # Tool registrations, stdio transport
│   │   ├── package.json      # @fixonce/mcp-server
│   │   └── tsconfig.json
│   │
│   ├── cli/                  # CLI (Story 6)
│   │   ├── src/
│   │   │   └── index.ts      # Command definitions
│   │   ├── package.json      # fixonce (bin entry)
│   │   └── tsconfig.json
│   │
│   └── web/                  # Web UI (Story 7)
│       ├── src/
│       │   ├── App.tsx
│       │   └── main.tsx
│       ├── package.json      # @fixonce/web
│       ├── vite.config.ts
│       └── tsconfig.json
│
└── .claude/                  # Claude Code configuration
    └── settings.json
```

### Rationale

- **pnpm**: Fastest package manager, strict dependency resolution (prevents phantom deps), native workspace support, disk-efficient via content-addressable store.
- **Turborepo**: Task orchestration with caching. `turbo run build` builds packages in dependency order. Cached rebuilds are near-instant (~0.2s vs ~30s).
- **Separate `packages/` and `apps/`**: Standard monorepo convention. Packages are libraries consumed by apps. Apps are entry points (executables, servers).
- **`@fixonce/shared`**: Single source of truth for types and schemas used by all packages and apps.
- **`@fixonce/storage`**: Encapsulates all Supabase and Voyage AI interactions. No other package imports Supabase directly.
- **`@fixonce/pipeline`**: Contains the business logic (write path, read path) independent of transport (MCP, CLI, Web).

### Alternatives Considered

- **npm workspaces**: Slower than pnpm, no content-addressable store, less strict dependency resolution. Works but pnpm is better.
- **Yarn workspaces + Lerna**: Lerna is deprecated in favor of Turborepo. Yarn Berry has PnP compatibility issues.
- **Nx**: More powerful but heavier. Turborepo is simpler for a project this size.
- **Single package**: Would work for MVP but makes it harder to separate concerns and independently test packages.

### Implementation Notes

- `pnpm-workspace.yaml` defines `packages/*` and `apps/*` as workspace members
- `turbo.json` defines task pipelines: `build` depends on `^build` (dependencies first)
- TypeScript project references in `tsconfig.json` for cross-package type checking
- The CLI app (`apps/cli`) publishes as the `fixonce` npm package with a `bin` entry
- Use `tsup` or `unbuild` for building packages (fast, zero-config for library builds)

### Sources

- [pnpm Workspaces](https://pnpm.io/workspaces)
- [Turborepo Getting Started](https://turbo.build/repo/docs)
- [Modern TypeScript Monorepo Example](https://github.com/bakeruk/modern-typescript-monorepo-example)

---

## 7. WebSocket vs SSE for Realtime

### Decision

Use **Server-Sent Events (SSE)** for the Web UI activity stream.

### Rationale

The activity stream in Story 7 is strictly **one-way server-to-client**: the server pushes activity log events (query, create, update, feedback, detect operations) to the Web UI. The client never needs to send data back over this channel (it uses standard HTTP requests for actions like create, update, feedback).

SSE is the clear winner for this pattern:

| Factor | SSE | WebSocket |
|---|---|---|
| Direction | Server-to-client only | Bidirectional |
| Complexity | Minimal (EventSource API) | Requires upgrade handshake, ping/pong |
| Reconnection | Built-in automatic reconnect | Must implement manually |
| Protocol | HTTP (works through all proxies) | Requires protocol upgrade |
| Browser API | `EventSource` — 5 lines of code | `WebSocket` — more setup code |
| Server implementation | Simple HTTP response with `text/event-stream` | Requires ws library |

**Client Implementation**:

```typescript
const events = new EventSource("/api/activity/stream");

events.onmessage = (event) => {
  const activity = JSON.parse(event.data);
  // Update React state
};

events.onerror = () => {
  // EventSource auto-reconnects; just log
};
```

**Server Implementation** (in the Web UI backend, not the MCP server):

```typescript
app.get("/api/activity/stream", (req, res) => {
  res.writeHead(200, {
    "Content-Type": "text/event-stream",
    "Cache-Control": "no-cache",
    Connection: "keep-alive",
  });

  const unsubscribe = activityLog.subscribe((event) => {
    res.write(`data: ${JSON.stringify(event)}\n\n`);
  });

  req.on("close", unsubscribe);
});
```

### Alternatives Considered

- **WebSocket**: More capable but unnecessary for one-way streaming. Adds complexity (ws library, upgrade handling, reconnection logic) with no benefit for this use case.
- **Polling**: Simpler but introduces latency and wasted requests. Not suitable for a "realtime" feel.
- **WebTransport**: Cutting-edge but poor browser support and overkill for this use case.

### Implementation Notes

- SSE has a browser limit of ~6 connections per domain (HTTP/1.1). Since this is a local dev tool with one tab, this is not a concern.
- `EventSource` automatically remembers `Last-Event-ID` and sends it on reconnect, enabling resume-from-last-event.
- Use `id:` field in SSE events mapped to `activity_log.id` for resume support.
- The SSE endpoint lives in the Web UI backend (`apps/web`), not in the MCP server.

### Sources

- [MDN: Using Server-Sent Events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events)
- [SSE vs WebSockets Comparison (Ably)](https://ably.com/blog/websockets-vs-sse)
- [SSE vs WebSockets (SoftwareMill)](https://softwaremill.com/sse-vs-websockets-comparing-real-time-communication-protocols/)

---

## Summary of Decisions

| # | Topic | Decision | Confidence |
|---|---|---|---|
| 1 | Claude Code Hooks | `@anthropic-ai/claude-agent-sdk` programmatic TS hooks. `streamInput()` confirmed for mid-run injection. | High |
| 2 | MCP Server | `@modelcontextprotocol/sdk` with `McpServer` + Zod v4 + stdio transport | High |
| 3 | Supabase pgvector | Hybrid search via RRF SQL function. HNSW index for vectors, GIN for FTS + JSONB. | High |
| 4 | Voyage AI | `voyage-code-3` at 1024 dims (default). REST API + `voyageai` npm package. | High |
| 5 | OpenRouter | OpenAI-compatible API. Gemma 3 4B for cheap tasks, Haiku for nuanced tasks. | High |
| 6 | Monorepo | pnpm workspaces + Turborepo. `packages/` (shared, storage, pipeline, activity) + `apps/` (mcp-server, cli, web). | High |
| 7 | Realtime | SSE for one-way activity stream. `EventSource` on client, simple HTTP stream on server. | High |

## Environment Variables Required

| Variable | Service | Used By |
|---|---|---|
| `SUPABASE_URL` | Supabase | `@fixonce/storage` |
| `SUPABASE_ANON_KEY` | Supabase | `@fixonce/storage` |
| `VOYAGE_API_KEY` | Voyage AI | `@fixonce/storage` (embeddings) |
| `OPENROUTER_API_KEY` | OpenRouter | `@fixonce/pipeline` (LLM calls) |

## Key npm Dependencies

| Package | Version | Used By | Purpose |
|---|---|---|---|
| `@anthropic-ai/claude-agent-sdk` | latest | hooks integration | Agent SDK for hook registration |
| `@modelcontextprotocol/sdk` | latest | `apps/mcp-server` | MCP server implementation |
| `zod` | v4 | all packages | Schema validation (MCP peer dep) |
| `@supabase/supabase-js` | v2+ | `@fixonce/storage` | Supabase client |
| `voyageai` | latest | `@fixonce/storage` | Embedding generation |
| `openai` | latest | `@fixonce/pipeline` | OpenRouter API client (OpenAI-compatible) |
| `react` | v19 | `apps/web` | Web UI framework |
| `vite` | latest | `apps/web` | Web UI build tool |
