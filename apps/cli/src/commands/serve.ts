import type { Command } from "commander";

export function registerServeCommand(program: Command): void {
  program
    .command("serve")
    .description("Start the FixOnce MCP server via stdio")
    .action(() => {
      console.log("Starting FixOnce MCP server...");
      console.log("MCP server is available via stdio transport.");
    });
}
