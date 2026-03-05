import type { Command } from "commander";
import type { Verbosity } from "@fixonce/shared";
import { expandCacheKey } from "@fixonce/pipeline";
import { formatExpandResult, formatJson } from "../formatters/index.js";
import { handleError } from "../utils.js";

export function registerExpandCommand(program: Command): void {
  program
    .command("expand <cache_key>")
    .description("Expand a cache key to view the full memory")
    .option("--verbosity <level>", "Verbosity: small, medium, or large")
    .action(async (cacheKey: string, opts) => {
      try {
        const result = await expandCacheKey({
          cache_key: cacheKey,
          verbosity: opts.verbosity as Verbosity | undefined,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatExpandResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
