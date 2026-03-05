#!/usr/bin/env node
/**
 * SessionStart hook: detect environment and surface critical project memories.
 * Blocking -- should complete within 1-2 seconds.
 *
 * Claude Code hooks receive JSON on stdin with session context.
 * Output goes to stdout as additional context for the session.
 */

import { detectEnvironment, queryMemories } from "@fixonce/pipeline";

interface SessionStartInput {
  readonly cwd?: string;
}

async function main(): Promise<void> {
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) {
    chunks.push(chunk as Buffer);
  }
  const input: SessionStartInput = JSON.parse(Buffer.concat(chunks).toString());

  const env = await detectEnvironment({
    project_path: input.cwd ?? process.cwd(),
  });

  // Quick search for critical project memories -- no rewrite/rerank for speed
  const memories = await queryMemories({
    query: "critical project setup configuration",
    type: "simple",
    rewrite: false,
    rerank: false,
    max_results: 3,
    verbosity: "small",
    version_predicates: env.detected_versions,
  });

  const context: string[] = [];

  if (Object.keys(env.detected_versions).length > 0) {
    context.push(
      `Detected versions: ${JSON.stringify(env.detected_versions)}`,
    );
  }

  if (memories.results.length > 0) {
    context.push("Critical memories:");
    for (const m of memories.results) {
      context.push(`- [${m.memory_type}] ${m.title}: ${m.summary}`);
    }
  }

  if (context.length > 0) {
    console.log(JSON.stringify({ additionalContext: context.join("\n") }));
  }
}

main().catch((err: unknown) => {
  console.error("SessionStart hook error:", err);
  process.exit(0);
});
