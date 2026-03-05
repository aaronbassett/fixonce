#!/usr/bin/env node
/**
 * Stop hook: final check for critical patterns before session ends.
 */

import { queryMemories } from "@fixonce/pipeline";

interface StopInput {
  readonly stop_reason?: string;
}

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const input: StopInput = JSON.parse(Buffer.concat(chunks).toString());

  const stopReason = input.stop_reason ?? "";
  if (stopReason === "user") return; // User explicitly stopped, don't interfere

  // Check for any critical guidance that should be surfaced before session ends
  const result = await queryMemories({
    query: "critical error common mistake deployment",
    type: "simple",
    rewrite: false,
    rerank: false,
    max_results: 2,
    verbosity: "small",
  });

  if (result.results.length > 0) {
    const lines = ["FixOnce reminders before finishing:"];
    for (const m of result.results) {
      lines.push(`- ${m.title}`);
    }
    console.log(JSON.stringify({ additionalContext: lines.join("\n") }));
  }
}

main().catch((err: unknown) => {
  console.error("Stop hook error:", err);
  process.exit(0);
});
