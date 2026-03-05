#!/usr/bin/env node
/**
 * UserPromptSubmit hook: quick memory lookup based on user prompt.
 * Returns relevant memories as additional context.
 */

import { queryMemories } from "@fixonce/pipeline";

interface UserPromptSubmitInput {
  readonly prompt?: string;
  readonly query?: string;
}

const MIN_PROMPT_LENGTH = 10;

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const input: UserPromptSubmitInput = JSON.parse(
    Buffer.concat(chunks).toString(),
  );
  const userPrompt = input.prompt ?? input.query ?? "";

  if (!userPrompt || userPrompt.length < MIN_PROMPT_LENGTH) return;

  // Quick search: simple mode, no rewrite/rerank for speed
  const result = await queryMemories({
    query: userPrompt,
    type: "simple",
    rewrite: false,
    rerank: false,
    max_results: 3,
    verbosity: "small",
  });

  if (result.results.length > 0) {
    const lines = ["Relevant FixOnce memories:"];
    for (const m of result.results) {
      const truncatedContent =
        m.content.length > 200 ? `${m.content.slice(0, 200)}...` : m.content;
      lines.push(`- [${m.memory_type}] ${m.title}`);
      lines.push(`  ${truncatedContent}`);
    }
    console.log(JSON.stringify({ additionalContext: lines.join("\n") }));
  }
}

main().catch((err: unknown) => {
  console.error("UserPromptSubmit hook error:", err);
  process.exit(0);
});
