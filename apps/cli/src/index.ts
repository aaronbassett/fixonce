#!/usr/bin/env node
import { Command } from "commander";
import { registerCreateCommand } from "./commands/create.js";
import { registerQueryCommand } from "./commands/query.js";
import { registerExpandCommand } from "./commands/expand.js";
import { registerGetCommand } from "./commands/get.js";
import { registerUpdateCommand } from "./commands/update.js";
import { registerFeedbackCommand } from "./commands/feedback.js";
import { registerDetectCommand } from "./commands/detect.js";
import { registerServeCommand } from "./commands/serve.js";
import { registerWebCommand } from "./commands/web.js";

const program = new Command()
  .name("fixonce")
  .description("FixOnce memory management CLI")
  .version("0.1.0")
  .option("--json", "Output as JSON");

registerCreateCommand(program);
registerQueryCommand(program);
registerExpandCommand(program);
registerGetCommand(program);
registerUpdateCommand(program);
registerFeedbackCommand(program);
registerDetectCommand(program);
registerServeCommand(program);
registerWebCommand(program);

program.parse();
