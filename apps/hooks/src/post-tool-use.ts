#!/usr/bin/env node
/**
 * PostToolUse hook: check content that was just written for anti-pattern matches.
 * Adds warnings as additional context.
 */

import { queryMemories } from "@fixonce/pipeline";

interface PostToolUseInput {
  readonly tool_name?: string;
  readonly tool_input?: {
    readonly content?: string;
    readonly new_string?: string;
  };
}

const CHECKED_TOOLS = new Set(["Write", "Edit"]);
const MIN_CONTENT_LENGTH = 20;
const QUERY_SLICE_LENGTH = 500;
const RELEVANCY_THRESHOLD = 0.5;
const CONTENT_PREVIEW_LENGTH = 150;

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const input: PostToolUseInput = JSON.parse(Buffer.concat(chunks).toString());

  const toolName = input.tool_name ?? "";
  if (!CHECKED_TOOLS.has(toolName)) return;

  const content =
    input.tool_input?.content ?? input.tool_input?.new_string ?? "";
  if (!content || content.length < MIN_CONTENT_LENGTH) return;

  const result = await queryMemories({
    query: content.slice(0, QUERY_SLICE_LENGTH),
    type: "simple",
    rewrite: false,
    rerank: false,
    max_results: 2,
    verbosity: "small",
    memory_type: "anti_pattern",
  });

  const warnings = result.results.filter(
    (m) => m.relevancy_score > RELEVANCY_THRESHOLD,
  );

  if (warnings.length > 0) {
    const lines = [
      "FixOnce detected potential issues in the code just written:",
    ];
    for (const m of warnings) {
      lines.push(`- ${m.title}: ${m.content.slice(0, CONTENT_PREVIEW_LENGTH)}`);
    }
    console.log(JSON.stringify({ additionalContext: lines.join("\n") }));
  }
}

main().catch((err: unknown) => {
  console.error("PostToolUse hook error:", err);
  process.exit(0);
});
