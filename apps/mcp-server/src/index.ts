#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { registerCreateMemoryTool } from "./tools/create-memory.js";
import { registerQueryTool } from "./tools/query.js";
import { registerExpandTool } from "./tools/expand.js";
import { registerGetMemoryTool } from "./tools/get-memory.js";
import { registerUpdateMemoryTool } from "./tools/update-memory.js";
import { registerFeedbackTool } from "./tools/feedback.js";
import { registerDetectEnvironmentTool } from "./tools/detect-environment.js";

const server = new McpServer({
  name: "fixonce",
  version: "0.1.0",
});

registerCreateMemoryTool(server);
registerQueryTool(server);
registerExpandTool(server);
registerGetMemoryTool(server);
registerUpdateMemoryTool(server);
registerFeedbackTool(server);
registerDetectEnvironmentTool(server);

const transport = new StdioServerTransport();
await server.connect(transport);
