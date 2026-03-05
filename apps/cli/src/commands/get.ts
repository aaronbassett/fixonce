import type { Command } from "commander";
import type { Verbosity } from "@fixonce/shared";
import { getMemory } from "@fixonce/pipeline";
import { formatMemory, formatJson } from "../formatters/index.js";
import { handleError } from "../utils.js";

export function registerGetCommand(program: Command): void {
  program
    .command("get <id>")
    .description("Get a single memory by ID")
    .option("--verbosity <level>", "Verbosity: small, medium, or large", "large")
    .action(async (id: string, opts) => {
      try {
        const result = await getMemory({
          id,
          verbosity: opts.verbosity as Verbosity,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatMemory(result.memory));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
