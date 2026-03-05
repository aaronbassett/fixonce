import type {
  CreateMemoryInput,
  CreateMemoryResult,
  QueryMemoriesInput,
  QueryMemoriesResult,
  ExpandCacheKeyInput,
  ExpandCacheKeyResult,
  GetMemoryInput,
  GetMemoryResult,
  UpdateMemoryInput,
  UpdateMemoryResult,
  SubmitFeedbackInput,
  SubmitFeedbackResult,
  DetectEnvironmentInput,
  DetectEnvironmentResult,
} from "@fixonce/shared";
import {
  CreateMemoryInputSchema,
  QueryMemoriesInputSchema,
  ExpandCacheKeyInputSchema,
  GetMemoryInputSchema,
  UpdateMemoryInputSchema,
  SubmitFeedbackInputSchema,
  DetectEnvironmentInputSchema,
} from "@fixonce/shared";
import { FixOnceError } from "@fixonce/shared";
import { logActivity } from "@fixonce/activity";
import {
  getMemoryById,
  updateMemory as storageUpdateMemory,
  generateEmbedding,
  createFeedback,
} from "@fixonce/storage";
import { executeWritePipeline } from "./write/index.js";
import { executeReadPipeline } from "./read/index.js";
import { lookupCacheKey } from "./read/cache.js";
import { detectEnvironment as scanEnvironment } from "./environment.js";
import { projectByVerbosity } from "./projections.js";

export async function createMemory(rawInput: CreateMemoryInput): Promise<CreateMemoryResult> {
  const input = CreateMemoryInputSchema.parse(rawInput);
  const result = await executeWritePipeline(input);

  await logActivity("create", {
    status: result.status,
    memory_id: result.memory?.id,
    dedup_outcome: result.dedup_outcome,
  }, result.memory?.id);

  return result;
}

export async function queryMemories(rawInput: QueryMemoriesInput): Promise<QueryMemoriesResult> {
  const input = QueryMemoriesInputSchema.parse(rawInput);
  const result = await executeReadPipeline(input);

  await logActivity("query", {
    query: input.query,
    search_type: result.pipeline.search_type,
    rewrite_used: result.pipeline.rewrite_used,
    rerank_used: result.pipeline.rerank_used,
    total_found: result.total_found,
    results_returned: result.results.length,
  });

  return result;
}

export async function expandCacheKey(rawInput: ExpandCacheKeyInput): Promise<ExpandCacheKeyResult> {
  const input = ExpandCacheKeyInputSchema.parse(rawInput);
  const verbosity = input.verbosity ?? "small";

  const memoryId = lookupCacheKey(input.cache_key);
  if (!memoryId) {
    throw new FixOnceError({
      stage: "cache",
      reason: `Cache key "${input.cache_key}" not found or expired.`,
      suggestion: "Re-run the query to get fresh cache keys.",
    });
  }

  const memory = await getMemoryById(memoryId);
  if (!memory) {
    throw new FixOnceError({
      stage: "storage",
      reason: `Memory "${memoryId}" referenced by cache key no longer exists.`,
      suggestion: "The memory may have been deleted. Re-run the query.",
    });
  }

  const projected = await projectByVerbosity(memory, 0, verbosity);

  await logActivity("query", {
    cache_key: input.cache_key,
    memory_id: memoryId,
    verbosity,
  }, memoryId);

  return { memory: projected };
}

export async function getMemory(rawInput: GetMemoryInput): Promise<GetMemoryResult> {
  const input = GetMemoryInputSchema.parse(rawInput);
  const verbosity = input.verbosity ?? "large";

  const memory = await getMemoryById(input.id);
  if (!memory) {
    throw new FixOnceError({
      stage: "storage",
      reason: `Memory "${input.id}" not found.`,
      suggestion: "Check the memory ID and try again.",
    });
  }

  const projected = await projectByVerbosity(memory, 0, verbosity);

  return { memory: projected };
}

export async function updateMemory(rawInput: UpdateMemoryInput): Promise<UpdateMemoryResult> {
  const input = UpdateMemoryInputSchema.parse(rawInput);
  const { id, ...updates } = input;

  const existing = await getMemoryById(id);
  if (!existing) {
    throw new FixOnceError({
      stage: "storage",
      reason: `Memory "${id}" not found.`,
      suggestion: "Check the memory ID and try again.",
    });
  }

  const contentChanged =
    (updates.content !== undefined && updates.content !== existing.content) ||
    (updates.summary !== undefined && updates.summary !== existing.summary);

  const updated = await storageUpdateMemory(id, updates);

  let embeddingRegenerating = false;
  if (contentChanged) {
    embeddingRegenerating = true;
    const embeddingSource = `${updated.title} ${updated.summary} ${updated.content}`;
    generateEmbedding(embeddingSource, "document")
      .then(async (embedding) => {
        await storageUpdateMemory(id, { embedding });
      })
      .catch((err) => {
        console.error(`Failed to regenerate embedding for memory ${id}:`, err);
      });
  }

  await logActivity("update", {
    memory_id: id,
    fields_updated: Object.keys(updates),
    embedding_regenerating: embeddingRegenerating,
  }, id);

  return {
    memory: { id: updated.id, title: updated.title, updated_at: updated.updated_at },
    embedding_regenerating: embeddingRegenerating,
  };
}

export async function submitFeedback(rawInput: SubmitFeedbackInput): Promise<SubmitFeedbackResult> {
  const input = SubmitFeedbackInputSchema.parse(rawInput);

  const memory = await getMemoryById(input.memory_id);
  if (!memory) {
    throw new FixOnceError({
      stage: "storage",
      reason: `Memory "${input.memory_id}" not found.`,
      suggestion: "Check the memory ID and try again.",
    });
  }

  const feedback = await createFeedback({
    memory_id: input.memory_id,
    text: input.text ?? null,
    tags: input.tags ?? [],
    suggested_action: input.suggested_action ?? null,
  });

  const memoryFlagged =
    input.suggested_action === "remove" ||
    input.suggested_action === "fix" ||
    (input.tags ?? []).includes("damaging");

  await logActivity("feedback", {
    memory_id: input.memory_id,
    feedback_id: feedback.id,
    tags: input.tags,
    suggested_action: input.suggested_action,
    memory_flagged: memoryFlagged,
  }, input.memory_id);

  return {
    feedback: {
      id: feedback.id,
      memory_id: feedback.memory_id,
      created_at: feedback.created_at,
    },
    memory_flagged: memoryFlagged,
  };
}

export async function detectEnvironment(
  rawInput: DetectEnvironmentInput,
): Promise<DetectEnvironmentResult> {
  const input = DetectEnvironmentInputSchema.parse(rawInput);
  const result = await scanEnvironment(input);

  await logActivity("detect", {
    detected_count: Object.keys(result.detected_versions).length,
    undetected_count: result.undetected_components.length,
    scan_sources: Object.keys(result.scan_sources),
  });

  return result;
}
