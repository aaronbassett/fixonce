import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { submitFeedback } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerFeedbackTool(server: McpServer): void {
  server.tool(
    "fixonce_feedback",
    "Submit feedback on a memory to help improve quality over time. Feedback tags indicate whether a memory was helpful, accurate, outdated, or damaging. Use 'suggested_action' to flag a memory for removal or correction. Memories flagged with 'remove', 'fix', or 'damaging' are surfaced for human review.",
    {
      memory_id: z.string().uuid().describe("UUID of the memory to provide feedback on"),
      text: z.string().nullable().optional().describe("Free-form feedback text explaining the issue or observation"),
      tags: z.array(
        z.enum([
          "helpful", "not_helpful", "damaging", "accurate",
          "somewhat_accurate", "somewhat_inaccurate", "inaccurate", "outdated",
        ]),
      ).optional().describe("Feedback classification tags"),
      suggested_action: z.enum(["keep", "remove", "fix"]).nullable().optional().describe("Suggested action: 'keep' if fine, 'remove' if wrong, 'fix' if needs correction"),
    },
    async (args) => {
      try {
        const result = await submitFeedback(args);
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
