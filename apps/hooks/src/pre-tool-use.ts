#!/usr/bin/env node
/**
 * PreToolUse hook: check proposed writes/edits against anti-pattern memories.
 * Can block the tool use if an anti-pattern is detected.
 */

import { queryMemories } from "@fixonce/pipeline";

interface PreToolUseInput {
  readonly tool_name?: string;
  readonly tool_input?: {
    readonly content?: string;
    readonly new_string?: string;
  };
}

const CHECKED_TOOLS = new Set(["Write", "Edit"]);
const MIN_CONTENT_LENGTH = 20;
const QUERY_SLICE_LENGTH = 500;
const RELEVANCY_THRESHOLD = 0.7;

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const input: PreToolUseInput = JSON.parse(Buffer.concat(chunks).toString());

  const toolName = input.tool_name ?? "";
  if (!CHECKED_TOOLS.has(toolName)) return;

  const content =
    input.tool_input?.content ?? input.tool_input?.new_string ?? "";
  if (!content || content.length < MIN_CONTENT_LENGTH) return;

  // Search for anti-patterns matching this content
  const result = await queryMemories({
    query: content.slice(0, QUERY_SLICE_LENGTH),
    type: "simple",
    rewrite: false,
    rerank: false,
    max_results: 3,
    verbosity: "small",
    memory_type: "anti_pattern",
  });

  const matches = result.results.filter(
    (m) => m.relevancy_score > RELEVANCY_THRESHOLD,
  );

  if (matches.length > 0) {
    const reasons = matches.map(
      (m) => `[anti_pattern] ${m.title}: ${m.summary}`,
    );
    console.log(
      JSON.stringify({
        decision: "block",
        reason: `FixOnce detected potential anti-patterns:\n${reasons.join("\n")}`,
      }),
    );
  }
}

main().catch((err: unknown) => {
  console.error("PreToolUse hook error:", err);
  process.exit(0);
});
