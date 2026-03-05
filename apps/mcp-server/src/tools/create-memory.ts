import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { createMemory } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerCreateMemoryTool(server: McpServer): void {
  server.tool(
    "fixonce_create_memory",
    "Submit a new memory to FixOnce. AI-created memories pass through a quality gate and duplicate detection pipeline before storage. Human-created memories are stored immediately. Returns the created memory ID and status, or a rejection reason if the memory did not pass quality checks.",
    {
      title: z.string().min(1).max(500).describe("Short descriptive title for the memory"),
      content: z.string().min(1).max(51200).describe("Full memory content with details, code examples, and context"),
      summary: z.string().min(1).max(1000).describe("Brief one-line summary of what this memory captures"),
      memory_type: z.enum(["guidance", "anti_pattern"]).describe("Type of memory: 'guidance' for recommended practices, 'anti_pattern' for things to avoid"),
      source_type: z.enum(["correction", "discovery", "instruction"]).describe("How the memory was discovered: 'correction' from fixing mistakes, 'discovery' from exploration, 'instruction' from explicit teaching"),
      created_by: z.enum(["ai", "human"]).describe("Who created this memory: 'ai' triggers quality gate, 'human' skips it"),
      language: z.string().min(1).describe("Programming language or technology context (e.g. 'typescript', 'rust', 'compact')"),
      tags: z.array(z.string().max(100)).max(20).optional().describe("Tags for categorization and filtering"),
      source_url: z.string().url().nullable().optional().describe("URL where this knowledge was discovered"),
      version_predicates: z.record(z.array(z.string())).nullable().optional().describe("Version constraints per component, e.g. { \"compact_compiler\": [\">=0.15.0\"] }"),
      project_name: z.string().nullable().optional().describe("Project name this memory is scoped to"),
      project_repo_url: z.string().nullable().optional().describe("Project repository URL"),
      project_workspace_path: z.string().nullable().optional().describe("Workspace path on disk"),
      confidence: z.number().min(0).max(1).optional().describe("Confidence score from 0 to 1, default 0.5"),
    },
    async (args) => {
      try {
        const result = await createMemory(args);
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        if (err instanceof FixOnceError) {
          return {
            content: [{ type: "text", text: JSON.stringify({ error: err.toJSON() }, null, 2) }],
            isError: true,
          };
        }
        throw err;
      }
    },
  );
}
