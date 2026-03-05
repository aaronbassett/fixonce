import type { Memory } from "@fixonce/shared";
import { rerankError } from "@fixonce/shared";
import { llmCallJSON } from "../llm.js";

export interface RankedMemory {
  memory: Memory;
  relevancy_score: number;
}

interface RerankResponse {
  rankings: Array<{
    id: string;
    score: number;
  }>;
}

const SYSTEM_PROMPT = `You are a search result reranker. Given a user query and a list of candidate memory entries, score each candidate for relevancy.

Rules:
- Score each candidate from 0.0 (irrelevant) to 1.0 (perfect match)
- Remove exact or near-duplicate entries (keep the higher quality one)
- De-rank entries that have negative feedback indicators (tags like "not_helpful", "damaging", or "inaccurate")
- Consider title, content, and summary when scoring
- Return a JSON object with a "rankings" array containing objects with "id" (string) and "score" (number)
- Sort by score descending
- Only include candidates with score > 0.1

Respond with ONLY valid JSON, no explanation.`;

function buildUserMessage(query: string, candidates: Memory[]): string {
  const candidateSummaries = candidates.map((c) => ({
    id: c.id,
    title: c.title,
    summary: c.summary,
    content_preview: c.content.slice(0, 300),
    tags: c.tags,
    memory_type: c.memory_type,
  }));

  return JSON.stringify({
    query,
    candidates: candidateSummaries,
  });
}

export async function rerankResults(
  query: string,
  candidates: Memory[],
  maxResults: number,
): Promise<RankedMemory[]> {
  if (candidates.length === 0) return [];

  try {
    const response = await llmCallJSON<RerankResponse>(
      "reranking",
      SYSTEM_PROMPT,
      buildUserMessage(query, candidates),
    );

    const candidateMap = new Map(candidates.map((c) => [c.id, c]));
    const ranked: RankedMemory[] = [];

    for (const entry of response.rankings) {
      const memory = candidateMap.get(entry.id);
      if (memory && entry.score > 0.1) {
        ranked.push({
          memory,
          relevancy_score: Math.min(1, Math.max(0, entry.score)),
        });
      }
    }

    ranked.sort((a, b) => b.relevancy_score - a.relevancy_score);
    return ranked.slice(0, maxResults);
  } catch (err) {
    throw rerankError(
      `Reranking failed: ${err instanceof Error ? err.message : String(err)}`,
      "Try searching with rerank=false to skip this step.",
    );
  }
}
