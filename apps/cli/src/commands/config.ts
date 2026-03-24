import { Command } from "commander";
import { mkdirSync, existsSync, writeFileSync } from "node:fs";
import { execSync } from "node:child_process";
import {
  SETTINGS_DIR,
  SETTINGS_PATH,
  SETTINGS_TEMPLATE,
} from "@fixonce/shared";

export function registerConfigCommand(parent: Command): void {
  parent
    .command("config")
    .description("Open the FixOnce settings file in your editor")
    .action(() => {
      mkdirSync(SETTINGS_DIR, { recursive: true });

      if (!existsSync(SETTINGS_PATH)) {
        writeFileSync(
          SETTINGS_PATH,
          JSON.stringify(SETTINGS_TEMPLATE, null, 2) + "\n",
          "utf-8",
        );
      }

      const editor = process.env.EDITOR || process.env.VISUAL;
      if (!editor) {
        console.log(`Settings file created at ${SETTINGS_PATH}`);
        console.log("Open it in your editor and fill in your API keys.");
        return;
      }

      execSync(`${editor} "${SETTINGS_PATH}"`, { stdio: "inherit" });
    });
}
