import type { Command } from "commander";
import type { MemoryType, SearchType, Verbosity } from "@fixonce/shared";
import { queryMemories } from "@fixonce/pipeline";
import { formatQueryResult, formatJson } from "../formatters/index.js";
import { handleError } from "../utils.js";

export function registerQueryCommand(program: Command): void {
  program
    .command("query <query>")
    .description("Query memories by natural language")
    .option("--no-rewrite", "Disable query rewriting")
    .option("--type <type>", "Search type: simple, vector, or hybrid")
    .option("--no-rerank", "Disable reranking")
    .option("--tags <tags>", "Comma-separated tag filter")
    .option("--language <lang>", "Language filter")
    .option("--project-name <name>", "Project name filter")
    .option(
      "--memory-type <type>",
      "Memory type filter: guidance or anti_pattern",
    )
    .option("--created-after <date>", "Created after ISO date")
    .option("--updated-after <date>", "Updated after ISO date")
    .option("--max-results <n>", "Maximum results", parseInt)
    .option("--max-tokens <n>", "Maximum tokens", parseInt)
    .option("--verbosity <level>", "Verbosity: small, medium, or large")
    .action(async (query: string, opts) => {
      try {
        const result = await queryMemories({
          query,
          rewrite: opts.rewrite as boolean,
          type: opts.type as SearchType | undefined,
          rerank: opts.rerank as boolean,
          tags: opts.tags
            ? (opts.tags as string).split(",").map((t: string) => t.trim())
            : undefined,
          language: opts.language as string | undefined,
          project_name: opts.projectName as string | undefined,
          memory_type: opts.memoryType as MemoryType | undefined,
          created_after: opts.createdAfter as string | undefined,
          updated_after: opts.updatedAfter as string | undefined,
          max_results: opts.maxResults as number | undefined,
          max_tokens: opts.maxTokens as number | undefined,
          verbosity: opts.verbosity as Verbosity | undefined,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatQueryResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
