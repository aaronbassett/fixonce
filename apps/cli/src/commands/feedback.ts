import type { Command } from "commander";
import type { FeedbackTag, SuggestedAction } from "@fixonce/shared";
import { submitFeedback } from "@fixonce/pipeline";
import { formatFeedbackResult, formatJson } from "../formatters/index.js";
import { handleError } from "../utils.js";

export function registerFeedbackCommand(program: Command): void {
  program
    .command("feedback <memory_id>")
    .description("Submit feedback for a memory")
    .option("--text <text>", "Feedback text")
    .option("--tags <tags>", "Comma-separated feedback tags")
    .option("--action <action>", "Suggested action: keep, remove, or fix")
    .action(async (memoryId: string, opts) => {
      try {
        const result = await submitFeedback({
          memory_id: memoryId,
          text: opts.text as string | undefined,
          tags: opts.tags
            ? ((opts.tags as string)
                .split(",")
                .map((t: string) => t.trim()) as FeedbackTag[])
            : undefined,
          suggested_action: opts.action as SuggestedAction | undefined,
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatFeedbackResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
