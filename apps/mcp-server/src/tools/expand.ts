import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { expandCacheKey } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerExpandTool(server: McpServer): void {
  server.tool(
    "fixonce_expand",
    "Expand a cache key from a previous query's overflow section into the full memory content. Cache keys are short-lived references returned by fixonce_query when results exceed the token budget. Use this to retrieve individual overflow memories on demand.",
    {
      cache_key: z
        .string()
        .min(1)
        .describe(
          "Cache key from the overflow section of a fixonce_query result",
        ),
      verbosity: z
        .enum(["small", "medium", "large"])
        .optional()
        .describe("Detail level of returned memory (default: 'small')"),
    },
    async (args) => {
      try {
        const result = await expandCacheKey(args);
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
