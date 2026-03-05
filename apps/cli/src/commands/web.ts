import type { Command } from "commander";

export function registerWebCommand(program: Command): void {
  program
    .command("web")
    .description("Start the FixOnce web UI")
    .option("--port <port>", "Port number", "3000")
    .option("--no-open", "Do not open browser automatically")
    .action((opts) => {
      const port = opts.port as string;
      const shouldOpen = opts.open as boolean;
      console.log(`Starting FixOnce web UI on port ${port}...`);
      if (shouldOpen) {
        console.log("Browser will open automatically.");
      }
    });
}
