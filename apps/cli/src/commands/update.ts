import type { Command } from "commander";
import type { MemoryType, SourceType } from "@fixonce/shared";
import { updateMemory } from "@fixonce/pipeline";
import { formatUpdateResult, formatJson } from "../formatters/index.js";
import { readStdin, handleError } from "../utils.js";

export function registerUpdateCommand(program: Command): void {
  program
    .command("update <id>")
    .description("Update an existing memory")
    .option("--title <title>", "New title")
    .option("--content <content>", "New content (or pipe via stdin)")
    .option("--summary <summary>", "New summary")
    .option("--memory-type <type>", "guidance or anti_pattern")
    .option("--source-type <type>", "correction, discovery, or instruction")
    .option("--tags <tags>", "Comma-separated tags")
    .option("--language <lang>", "Language context")
    .option("--source-url <url>", "Source URL")
    .option("--version <json>", "Version predicates JSON")
    .option("--project-name <name>", "Project name")
    .option("--project-repo-url <url>", "Project repo URL")
    .option("--confidence <n>", "Confidence 0-1", parseFloat)
    .option("--enabled <bool>", "Enable or disable memory")
    .action(async (id: string, opts) => {
      try {
        let content: string | undefined = opts.content as string | undefined;
        if (!content && !process.stdin.isTTY) {
          content = await readStdin();
        }

        const result = await updateMemory({
          id,
          title: opts.title as string | undefined,
          content,
          summary: opts.summary as string | undefined,
          memory_type: opts.memoryType as MemoryType | undefined,
          source_type: opts.sourceType as SourceType | undefined,
          tags: opts.tags ? (opts.tags as string).split(",").map((t: string) => t.trim()) : undefined,
          language: opts.language as string | undefined,
          source_url: opts.sourceUrl as string | undefined,
          version_predicates: opts.version ? JSON.parse(opts.version as string) : undefined,
          project_name: opts.projectName as string | undefined,
          project_repo_url: opts.projectRepoUrl as string | undefined,
          confidence: opts.confidence as number | undefined,
          enabled: opts.enabled !== undefined ? opts.enabled === "true" : undefined,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatUpdateResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
