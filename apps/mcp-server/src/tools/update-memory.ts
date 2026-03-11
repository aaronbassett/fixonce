import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { updateMemory } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerUpdateMemoryTool(server: McpServer): void {
  server.tool(
    "fixonce_update_memory",
    "Update an existing memory's fields. Only the provided fields are changed; omitted fields remain unchanged. If content or summary changes, the embedding is regenerated asynchronously in the background.",
    {
      id: z.string().uuid().describe("UUID of the memory to update"),
      title: z.string().min(1).max(500).optional().describe("Updated title"),
      content: z
        .string()
        .min(1)
        .max(51200)
        .optional()
        .describe("Updated content"),
      summary: z
        .string()
        .min(1)
        .max(1000)
        .optional()
        .describe("Updated summary"),
      memory_type: z
        .enum(["guidance", "anti_pattern"])
        .optional()
        .describe("Updated memory type"),
      source_type: z
        .enum(["correction", "discovery", "instruction"])
        .optional()
        .describe("Updated source type"),
      source_url: z
        .string()
        .url()
        .nullable()
        .optional()
        .describe("Updated source URL, or null to clear"),
      tags: z
        .array(z.string().max(100))
        .max(20)
        .optional()
        .describe("Replacement tag list (replaces all existing tags)"),
      language: z
        .string()
        .min(1)
        .optional()
        .describe("Updated language context"),
      version_predicates: z
        .record(z.array(z.string()))
        .nullable()
        .optional()
        .describe("Updated version constraints, or null to clear"),
      project_name: z
        .string()
        .nullable()
        .optional()
        .describe("Updated project name, or null to clear"),
      project_repo_url: z
        .string()
        .nullable()
        .optional()
        .describe("Updated project repo URL, or null to clear"),
      project_workspace_path: z
        .string()
        .nullable()
        .optional()
        .describe("Updated workspace path, or null to clear"),
      confidence: z
        .number()
        .min(0)
        .max(1)
        .optional()
        .describe("Updated confidence score"),
      enabled: z
        .boolean()
        .optional()
        .describe(
          "Enable or disable this memory from appearing in query results",
        ),
    },
    async (args) => {
      try {
        const result = await updateMemory(args);
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
