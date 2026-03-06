import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { getMemory } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerGetMemoryTool(server: McpServer): void {
  server.tool(
    "fixonce_get_memory",
    "Retrieve a single memory by its UUID. Returns the full memory content at the requested verbosity level. Use this when you already have a memory ID and want to read its details.",
    {
      id: z.string().uuid().describe("UUID of the memory to retrieve"),
      verbosity: z
        .enum(["small", "medium", "large"])
        .optional()
        .describe(
          "Detail level: 'small' for core fields, 'medium' adds metadata, 'large' adds feedback summary (default: 'large')",
        ),
    },
    async (args) => {
      try {
        const result = await getMemory(args);
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
