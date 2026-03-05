import type {
  QueryMemoriesInput,
  QueryMemoriesResult,
  Memory,
  OverflowEntry,
} from "@fixonce/shared";
import { searchError, embeddingError } from "@fixonce/shared";
import {
  hybridSearch,
  vectorSearch,
  ftsSearch,
  generateEmbedding,
  filterByVersionPredicates,
  incrementSurfacedCount,
} from "@fixonce/storage";
import { rewriteQuery } from "./query-rewriter.js";
import { rerankResults } from "./reranker.js";
import type { RankedMemory } from "./reranker.js";
import { generateCacheKey } from "./cache.js";
import { projectByVerbosity } from "../projections.js";

async function searchByType(
  query: string,
  type: "hybrid" | "vector" | "simple",
  options: {
    language?: string;
    memory_type?: string;
    tags?: string[];
    created_after?: string;
    updated_after?: string;
  },
): Promise<Memory[]> {
  const searchOpts = {
    match_count: 20,
    language: options.language,
    memory_type: options.memory_type,
    tags: options.tags,
    created_after: options.created_after,
    updated_after: options.updated_after,
  };

  if (type === "simple") {
    return ftsSearch({ ...searchOpts, query_text: query });
  }

  let embedding: number[];
  try {
    embedding = await generateEmbedding(query, "query");
  } catch (err) {
    throw embeddingError(
      `Failed to generate query embedding: ${err instanceof Error ? err.message : String(err)}`,
      "Try searching with type='simple' to avoid embedding generation.",
    );
  }

  if (type === "vector") {
    return vectorSearch({ ...searchOpts, query_embedding: embedding });
  }

  return hybridSearch({
    ...searchOpts,
    query_text: query,
    query_embedding: embedding,
  });
}

export async function executeReadPipeline(
  input: QueryMemoriesInput,
): Promise<QueryMemoriesResult> {
  const rewriteUsed = input.rewrite !== false;
  const rerankUsed = input.rerank !== false;
  const searchType = input.type ?? "hybrid";
  const maxResults = input.max_results ?? 5;
  const verbosity = input.verbosity ?? "small";

  // Stage 1: Rewrite
  let searchQuery = input.query;
  if (rewriteUsed) {
    searchQuery = await rewriteQuery(input.query);
  }

  // Stage 2: Search
  let candidates: Memory[];
  try {
    candidates = await searchByType(searchQuery, searchType, {
      language: input.language,
      memory_type: input.memory_type,
      tags: input.tags,
      created_after: input.created_after,
      updated_after: input.updated_after,
    });
  } catch (err) {
    if (err instanceof Error && "stage" in err) throw err;
    throw searchError(
      `Search failed: ${err instanceof Error ? err.message : String(err)}`,
      "Check that the database is reachable and try again.",
    );
  }

  // Stage 3: Version filter
  if (input.version_predicates && Object.keys(input.version_predicates).length > 0) {
    candidates = filterByVersionPredicates(candidates, input.version_predicates);
  }

  const totalFound = candidates.length;

  // Stage 4: Rerank
  let ranked: RankedMemory[];
  if (rerankUsed && candidates.length > 0) {
    ranked = await rerankResults(searchQuery, candidates, candidates.length);
  } else {
    ranked = candidates.map((memory, index) => ({
      memory,
      relevancy_score: Math.max(0, 1 - index * 0.05),
    }));
  }

  // Stage 5: Budget — split into top results and overflow
  const topRanked = ranked.slice(0, maxResults);
  const overflowRanked = ranked.slice(maxResults);

  // Stage 6: Project verbosity on top results
  const results = await Promise.all(
    topRanked.map((r) => projectByVerbosity(r.memory, r.relevancy_score, verbosity)),
  );

  // Build overflow entries with cache keys
  const overflow: OverflowEntry[] = overflowRanked.map((r) => ({
    id: r.memory.id,
    title: r.memory.title,
    summary: r.memory.summary,
    relevancy_score: r.relevancy_score,
    cache_key: generateCacheKey(r.memory.id),
  }));

  // Stage 7: Track surfaced counts (fire-and-forget)
  const surfacedIds = topRanked.map((r) => r.memory.id);
  incrementSurfacedCount(surfacedIds).catch((err) => {
    console.error("Failed to increment surfaced counts:", err);
  });

  return {
    results,
    overflow,
    total_found: totalFound,
    pipeline: {
      rewrite_used: rewriteUsed,
      search_type: searchType,
      rerank_used: rerankUsed,
    },
  };
}
