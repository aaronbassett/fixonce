import type { Command } from "commander";
import { createMemory } from "@fixonce/pipeline";
import { formatCreateResult, formatJson } from "../formatters/index.js";
import { readStdin, handleError } from "../utils.js";

export function registerCreateCommand(program: Command): void {
  program
    .command("create")
    .description("Create a new memory")
    .requiredOption("--title <title>", "Memory title")
    .option("--content <content>", "Memory content (or pipe via stdin)")
    .requiredOption("--summary <summary>", "Brief summary")
    .requiredOption("--memory-type <type>", "guidance or anti_pattern")
    .requiredOption("--source-type <type>", "correction, discovery, or instruction")
    .requiredOption("--language <lang>", "Language context")
    .option("--tags <tags>", "Comma-separated tags")
    .option("--source-url <url>", "Source URL")
    .option("--version <json>", "Version predicates JSON")
    .option("--project-name <name>", "Project name")
    .option("--project-repo-url <url>", "Project repo URL")
    .option("--confidence <n>", "Confidence 0-1", parseFloat)
    .action(async (opts) => {
      try {
        let content: string | undefined = opts.content as string | undefined;
        if (!content && !process.stdin.isTTY) {
          content = await readStdin();
        }
        if (!content) {
          console.error("Error: --content is required or pipe content via stdin");
          return process.exit(1);
        }

        const result = await createMemory({
          title: opts.title as string,
          content,
          summary: opts.summary as string,
          memory_type: opts.memoryType as string as "guidance" | "anti_pattern",
          source_type: opts.sourceType as string as "correction" | "discovery" | "instruction",
          created_by: "human",
          language: opts.language as string,
          tags: opts.tags ? (opts.tags as string).split(",").map((t: string) => t.trim()) : undefined,
          source_url: opts.sourceUrl as string | undefined,
          version_predicates: opts.version ? JSON.parse(opts.version as string) : undefined,
          project_name: opts.projectName as string | undefined,
          project_repo_url: opts.projectRepoUrl as string | undefined,
          confidence: opts.confidence as number | undefined,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatCreateResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
