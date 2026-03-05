import type { Command } from "commander";
import { detectEnvironment } from "@fixonce/pipeline";
import { formatDetectResult, formatJson } from "../formatters/index.js";
import { handleError } from "../utils.js";

export function registerDetectCommand(program: Command): void {
  program
    .command("detect [path]")
    .description("Detect environment versions from project files")
    .action(async (path: string | undefined) => {
      try {
        const result = await detectEnvironment({
          project_path: path ?? ".",
        });

        const isJson = program.opts()["json"] as boolean | undefined;
        console.log(isJson ? formatJson(result) : formatDetectResult(result));
      } catch (err) {
        handleError(err, Boolean(program.opts()["json"]));
      }
    });
}
