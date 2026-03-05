import type { Memory } from "@fixonce/shared";
import { hybridSearch, ftsSearch } from "@fixonce/storage";
import { generateEmbedding } from "@fixonce/storage";
import { llmCallJSON } from "../llm.js";

export type DedupOutcome = "new" | "discard" | "replace" | "update" | "merge";

interface DedupResult {
  outcome: DedupOutcome;
  reason: string;
  target_memory_id?: string;
  merged_content?: string;
  merged_title?: string;
  merged_summary?: string;
}

const SYSTEM_PROMPT = `You are a duplicate detection system for a memory store used by LLM coding agents.

Compare the NEW memory against EXISTING memories. Determine the best outcome.

Return JSON:
{
  "outcome": "new" | "discard" | "replace" | "update" | "merge",
  "reason": "explanation",
  "target_memory_id": "id of existing memory (for discard/replace/update/merge)",
  "merged_content": "combined content (only for merge)",
  "merged_title": "combined title (only for merge)",
  "merged_summary": "combined summary (only for merge)"
}

Outcomes:
- "new": No similar memories found. Store as new.
- "discard": Semantically identical to an existing memory. Drop the incoming.
- "replace": Incoming is a better/more accurate version. Replace existing.
- "update": Incoming has additional details. Update existing with new info.
- "merge": Complementary memories. Create new combined memory.`;

export async function detectDuplicates(
  title: string,
  content: string,
  summary: string,
  language: string,
): Promise<DedupResult> {
  // Search for similar memories
  let candidates: Memory[] = [];

  try {
    const embedding = await generateEmbedding(`${title} ${summary} ${content}`, "query");
    candidates = await hybridSearch({
      query_text: `${title} ${summary}`,
      query_embedding: embedding,
      match_count: 5,
      language,
    });
  } catch {
    // If embedding fails, fall back to FTS
    try {
      candidates = await ftsSearch({
        query_text: `${title} ${summary}`,
        match_count: 5,
        language,
      });
    } catch {
      // If both fail, treat as new
      return { outcome: "new", reason: "Could not search for duplicates" };
    }
  }

  if (candidates.length === 0) {
    return { outcome: "new", reason: "No similar memories found" };
  }

  const existingStr = candidates
    .map((m) => `[ID: ${m.id}]\nTitle: ${m.title}\nSummary: ${m.summary}\nContent: ${m.content}\n`)
    .join("\n---\n");

  const userMessage = `NEW MEMORY:\nTitle: ${title}\nSummary: ${summary}\nContent: ${content}\n\nEXISTING MEMORIES:\n${existingStr}`;

  return llmCallJSON<DedupResult>("duplicate_detection", SYSTEM_PROMPT, userMessage);
}
