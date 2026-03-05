import type { CreateMemoryInput, CreateMemoryResult, Memory } from "@fixonce/shared";
import { duplicateDetectionError, storageError } from "@fixonce/shared";
import { createMemory as storeMemory, updateMemory, getMemoryById } from "@fixonce/storage";
import { generateEmbedding } from "@fixonce/storage";
import { evaluateQuality } from "./quality-gate.js";
import { detectDuplicates } from "./duplicate-detection.js";

async function triggerAsyncEmbedding(memoryId: string, content: string): Promise<void> {
  // Fire-and-forget embedding generation
  generateEmbedding(content, "document")
    .then(async (embedding) => {
      await updateMemory(memoryId, { embedding });
    })
    .catch((err) => {
      console.error(`Failed to generate embedding for memory ${memoryId}:`, err);
    });
}

export async function executeWritePipeline(input: CreateMemoryInput): Promise<CreateMemoryResult> {
  // Human path: store immediately
  if (input.created_by === "human") {
    const memory = await storeMemory({
      title: input.title,
      content: input.content,
      summary: input.summary,
      memory_type: input.memory_type,
      source_type: input.source_type,
      created_by: input.created_by,
      source_url: input.source_url ?? null,
      tags: input.tags ?? [],
      language: input.language,
      version_predicates: input.version_predicates ?? null,
      project_name: input.project_name ?? null,
      project_repo_url: input.project_repo_url ?? null,
      project_workspace_path: input.project_workspace_path ?? null,
      confidence: input.confidence ?? 0.5,
    });

    triggerAsyncEmbedding(memory.id, `${memory.title} ${memory.summary} ${memory.content}`);

    return {
      status: "created",
      memory: { id: memory.id, title: memory.title, created_at: memory.created_at },
      dedup_outcome: "new",
    };
  }

  // AI path: quality gate -> dedup -> store/reject
  const qualityResult = await evaluateQuality(input.title, input.content, input.summary);

  if (qualityResult.decision === "reject") {
    return {
      status: "rejected",
      reason: qualityResult.reason,
    };
  }

  // Duplicate detection
  const dedupResult = await detectDuplicates(input.title, input.content, input.summary, input.language);

  switch (dedupResult.outcome) {
    case "new": {
      const memory = await storeMemory({
        title: input.title,
        content: input.content,
        summary: input.summary,
        memory_type: input.memory_type,
        source_type: input.source_type,
        created_by: input.created_by,
        source_url: input.source_url ?? null,
        tags: input.tags ?? [],
        language: input.language,
        version_predicates: input.version_predicates ?? null,
        project_name: input.project_name ?? null,
        project_repo_url: input.project_repo_url ?? null,
        project_workspace_path: input.project_workspace_path ?? null,
        confidence: input.confidence ?? 0.5,
      });

      triggerAsyncEmbedding(memory.id, `${memory.title} ${memory.summary} ${memory.content}`);

      return {
        status: "created",
        memory: { id: memory.id, title: memory.title, created_at: memory.created_at },
        dedup_outcome: "new",
      };
    }

    case "discard": {
      return {
        status: "discarded",
        reason: dedupResult.reason,
        existing_memory_id: dedupResult.target_memory_id,
        dedup_outcome: "discard",
      };
    }

    case "replace": {
      if (!dedupResult.target_memory_id) {
        throw duplicateDetectionError("Replace outcome requires target_memory_id", "This indicates a malformed LLM dedup response. Retry the operation.");
      }
      const existing = await getMemoryById(dedupResult.target_memory_id);
      if (!existing) throw storageError(`Target memory ${dedupResult.target_memory_id} not found for replace`, "The target memory may have been deleted. Retry to re-evaluate.");
      const updated = await updateMemory(dedupResult.target_memory_id, {
        title: input.title,
        content: input.content,
        summary: input.summary,
        embedding: null,
      });

      triggerAsyncEmbedding(updated.id, `${updated.title} ${updated.summary} ${updated.content}`);

      return {
        status: "replaced",
        memory: { id: updated.id, title: updated.title, created_at: existing.created_at },
        dedup_outcome: "replace",
        affected_memory_ids: [dedupResult.target_memory_id],
      };
    }

    case "update": {
      if (!dedupResult.target_memory_id) {
        throw duplicateDetectionError("Update outcome requires target_memory_id", "This indicates a malformed LLM dedup response. Retry the operation.");
      }
      const existing = await getMemoryById(dedupResult.target_memory_id);
      if (!existing) throw storageError(`Target memory ${dedupResult.target_memory_id} not found for update`, "The target memory may have been deleted. Retry to re-evaluate.");

      const updatedContent = `${existing.content}\n\n---\n\n${input.content}`;
      const updated = await updateMemory(dedupResult.target_memory_id, {
        content: updatedContent,
        summary: input.summary || existing.summary,
        embedding: null,
      });

      triggerAsyncEmbedding(updated.id, `${updated.title} ${updated.summary} ${updated.content}`);

      return {
        status: "updated",
        memory: { id: updated.id, title: updated.title, created_at: existing.created_at },
        dedup_outcome: "update",
        affected_memory_ids: [dedupResult.target_memory_id],
      };
    }

    case "merge": {
      if (!dedupResult.target_memory_id) {
        throw duplicateDetectionError("Merge outcome requires target_memory_id", "This indicates a malformed LLM dedup response. Retry the operation.");
      }

      // Disable original memory
      await updateMemory(dedupResult.target_memory_id, { enabled: false });

      // Create new merged memory
      const merged = await storeMemory({
        title: dedupResult.merged_title || input.title,
        content: dedupResult.merged_content || input.content,
        summary: dedupResult.merged_summary || input.summary,
        memory_type: input.memory_type,
        source_type: input.source_type,
        created_by: input.created_by,
        source_url: input.source_url ?? null,
        tags: input.tags ?? [],
        language: input.language,
        version_predicates: input.version_predicates ?? null,
        project_name: input.project_name ?? null,
        project_repo_url: input.project_repo_url ?? null,
        project_workspace_path: input.project_workspace_path ?? null,
        confidence: input.confidence ?? 0.5,
      });

      triggerAsyncEmbedding(merged.id, `${merged.title} ${merged.summary} ${merged.content}`);

      return {
        status: "merged",
        memory: { id: merged.id, title: merged.title, created_at: merged.created_at },
        dedup_outcome: "merge",
        affected_memory_ids: [dedupResult.target_memory_id],
      };
    }

    default: {
      const _exhaustive: never = dedupResult.outcome;
      throw new Error(`Unhandled dedup outcome: ${String(_exhaustive)}`);
    }
  }
}
