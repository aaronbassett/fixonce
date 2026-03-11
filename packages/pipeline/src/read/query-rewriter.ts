import { rewriteError } from "@fixonce/shared";
import { llmCall } from "../llm.js";

const SYSTEM_PROMPT = `You are a search query optimizer. Your job is to convert a user's conversational question or context into an optimized keyword-focused search query for a technical knowledge base.

Rules:
- Expand common abbreviations (e.g. "TS" -> "TypeScript", "env" -> "environment")
- Extract the core technical concepts and terms
- Remove filler words and conversational phrases
- Preserve version numbers and specific identifiers
- Return ONLY the rewritten query text, nothing else — no explanation, no quotes, no prefixes`;

export async function rewriteQuery(query: string): Promise<string> {
  try {
    const rewritten = await llmCall("query_rewriting", SYSTEM_PROMPT, query);
    return rewritten.trim();
  } catch (err) {
    throw rewriteError(
      `Query rewriting failed: ${err instanceof Error ? err.message : String(err)}`,
      "Try searching with rewrite=false to skip this step.",
    );
  }
}
