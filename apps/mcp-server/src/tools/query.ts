import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { queryMemories } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerQueryTool(server: McpServer): void {
  server.tool(
    "fixonce_query",
    "Search FixOnce memories using natural language. Supports hybrid search (text + vector), optional query rewriting for better recall, and reranking for precision. Returns matched memories with relevancy scores, plus overflow entries with cache keys for token-efficient expansion.",
    {
      query: z
        .string()
        .min(1)
        .describe("Natural language search query describing what you need"),
      rewrite: z
        .boolean()
        .optional()
        .describe(
          "Enable LLM query rewriting for better recall (default: true)",
        ),
      type: z
        .enum(["simple", "vector", "hybrid"])
        .optional()
        .describe(
          "Search strategy: 'simple' for text match, 'vector' for semantic, 'hybrid' for both (default: 'hybrid')",
        ),
      rerank: z
        .boolean()
        .optional()
        .describe(
          "Enable LLM reranking of results for better precision (default: true)",
        ),
      tags: z
        .array(z.string())
        .optional()
        .describe("Filter results to memories with any of these tags"),
      language: z
        .string()
        .optional()
        .describe("Filter results to a specific language or technology"),
      project_name: z
        .string()
        .optional()
        .describe("Filter results to a specific project"),
      memory_type: z
        .enum(["guidance", "anti_pattern"])
        .optional()
        .describe("Filter by memory type"),
      created_after: z
        .string()
        .optional()
        .describe(
          "ISO 8601 datetime to filter memories created after this date",
        ),
      updated_after: z
        .string()
        .optional()
        .describe(
          "ISO 8601 datetime to filter memories updated after this date",
        ),
      max_results: z
        .number()
        .int()
        .min(1)
        .max(50)
        .optional()
        .describe("Maximum number of results to return (default: 5)"),
      max_tokens: z
        .number()
        .int()
        .positive()
        .optional()
        .describe(
          "Approximate token budget for results; excess goes to overflow",
        ),
      verbosity: z
        .enum(["small", "medium", "large"])
        .optional()
        .describe(
          "Detail level of returned memories: 'small' for summaries, 'medium' adds metadata, 'large' adds feedback (default: 'small')",
        ),
      version_predicates: z
        .record(z.string())
        .optional()
        .describe(
          'Detected component versions to filter compatible memories, e.g. { "compact_compiler": "0.15.0" }',
        ),
    },
    async (args) => {
      try {
        const result = await queryMemories(args);
        return {
          content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
        };
      } catch (err) {
        if (err instanceof FixOnceError) {
          return {
            content: [
              {
                type: "text",
                text: JSON.stringify({ error: err.toJSON() }, null, 2),
              },
            ],
            isError: true,
          };
        }
        throw err;
      }
    },
  );
}
