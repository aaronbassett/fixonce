import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { detectEnvironment } from "@fixonce/pipeline";
import { FixOnceError } from "@fixonce/shared";

export function registerDetectEnvironmentTool(server: McpServer): void {
  server.tool(
    "fixonce_detect_environment",
    "Scan a project directory to detect installed component versions (compilers, SDKs, runtimes). Returns detected versions, the files they were found in, and a list of components that could not be detected. Use the detected versions as version_predicates in fixonce_query to filter memories to your environment.",
    {
      project_path: z
        .string()
        .optional()
        .describe(
          "Absolute path to the project directory to scan. Defaults to the current working directory if omitted.",
        ),
    },
    async (args) => {
      try {
        const result = await detectEnvironment(args);
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
