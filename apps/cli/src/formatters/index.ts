import type {
  CreateMemoryResult,
  QueryMemoriesResult,
  MemorySmall,
  MemoryMedium,
  MemoryLarge,
  UpdateMemoryResult,
  SubmitFeedbackResult,
  DetectEnvironmentResult,
  ExpandCacheKeyResult,
  OverflowEntry,
} from "@fixonce/shared";

export function formatJson(data: unknown): string {
  return JSON.stringify(data, null, 2);
}

export function formatCreateResult(result: CreateMemoryResult): string {
  const lines: string[] = [];
  lines.push(`Status: ${result.status}`);

  if (result.memory) {
    lines.push(`Memory ID: ${result.memory.id}`);
    lines.push(`Title: ${result.memory.title}`);
    lines.push(`Created at: ${result.memory.created_at}`);
  }

  if (result.dedup_outcome) {
    lines.push(`Dedup outcome: ${result.dedup_outcome}`);
  }

  if (result.affected_memory_ids && result.affected_memory_ids.length > 0) {
    lines.push(`Affected memories: ${result.affected_memory_ids.join(", ")}`);
  }

  if (result.reason) {
    lines.push(`Reason: ${result.reason}`);
  }

  if (result.existing_memory_id) {
    lines.push(`Existing memory: ${result.existing_memory_id}`);
  }

  return lines.join("\n");
}

export function formatQueryResult(result: QueryMemoriesResult): string {
  const lines: string[] = [];
  lines.push(`Found ${result.total_found} result(s)`);
  lines.push(`Pipeline: search=${result.pipeline.search_type}, rewrite=${result.pipeline.rewrite_used}, rerank=${result.pipeline.rerank_used}`);
  lines.push("");

  for (const memory of result.results) {
    lines.push(formatMemory(memory));
    lines.push("");
  }

  if (result.overflow.length > 0) {
    lines.push("--- Overflow (use `fixonce expand <cache_key>` to view) ---");
    for (const entry of result.overflow) {
      lines.push(formatOverflowEntry(entry));
    }
  }

  return lines.join("\n").trimEnd();
}

function formatOverflowEntry(entry: OverflowEntry): string {
  return `  [${entry.cache_key}] ${entry.title} (relevancy: ${entry.relevancy_score.toFixed(2)})`;
}

export function formatMemory(memory: MemorySmall | MemoryMedium | MemoryLarge): string {
  const lines: string[] = [];
  lines.push(`ID: ${memory.id}`);
  lines.push(`Title: ${memory.title}`);
  lines.push(`Type: ${memory.memory_type}`);
  lines.push(`Relevancy: ${memory.relevancy_score.toFixed(2)}`);
  lines.push(`Summary: ${memory.summary}`);

  if (hasField(memory, "tags")) {
    const med = memory as MemoryMedium;
    lines.push(`Tags: ${med.tags.join(", ")}`);
    lines.push(`Language: ${med.language}`);
    lines.push(`Source: ${med.source_type} by ${med.created_by}`);
    lines.push(`Created: ${med.created_at}`);
    lines.push(`Updated: ${med.updated_at}`);
    if (med.version_predicates) {
      lines.push(`Versions: ${JSON.stringify(med.version_predicates)}`);
    }
  }

  if (hasField(memory, "confidence")) {
    const lg = memory as MemoryLarge;
    lines.push(`Confidence: ${lg.confidence}`);
    lines.push(`Surfaced: ${lg.surfaced_count} time(s)`);
    if (lg.last_surfaced_at) {
      lines.push(`Last surfaced: ${lg.last_surfaced_at}`);
    }
    if (lg.source_url) {
      lines.push(`Source URL: ${lg.source_url}`);
    }
    if (lg.project_name) {
      lines.push(`Project: ${lg.project_name}`);
    }
    if (lg.project_repo_url) {
      lines.push(`Repo: ${lg.project_repo_url}`);
    }
    if (lg.project_workspace_path) {
      lines.push(`Workspace: ${lg.project_workspace_path}`);
    }
    lines.push(`Feedback: ${lg.feedback_summary.total_count} total`);
    if (lg.feedback_summary.flagged_actions.length > 0) {
      lines.push(`Flagged actions: ${lg.feedback_summary.flagged_actions.join(", ")}`);
    }
  }

  lines.push(`Content:\n${memory.content}`);

  return lines.join("\n");
}

export function formatUpdateResult(result: UpdateMemoryResult): string {
  const lines: string[] = [];
  lines.push(`Updated memory: ${result.memory.id}`);
  lines.push(`Title: ${result.memory.title}`);
  lines.push(`Updated at: ${result.memory.updated_at}`);
  if (result.embedding_regenerating) {
    lines.push("Embedding is being regenerated.");
  }
  return lines.join("\n");
}

export function formatFeedbackResult(result: SubmitFeedbackResult): string {
  const lines: string[] = [];
  lines.push(`Feedback ID: ${result.feedback.id}`);
  lines.push(`Memory ID: ${result.feedback.memory_id}`);
  lines.push(`Created at: ${result.feedback.created_at}`);
  if (result.memory_flagged) {
    lines.push("Memory has been flagged for review.");
  }
  return lines.join("\n");
}

export function formatDetectResult(result: DetectEnvironmentResult): string {
  const lines: string[] = [];
  const entries = Object.entries(result.detected_versions);

  if (entries.length > 0) {
    lines.push("Detected versions:");
    for (const [component, version] of entries) {
      const source = result.scan_sources[component as keyof typeof result.scan_sources];
      lines.push(`  ${component}: ${version}${source ? ` (from ${source})` : ""}`);
    }
  } else {
    lines.push("No versions detected.");
  }

  if (result.undetected_components.length > 0) {
    lines.push(`\nUndetected: ${result.undetected_components.join(", ")}`);
  }

  return lines.join("\n");
}

export function formatExpandResult(result: ExpandCacheKeyResult): string {
  return formatMemory(result.memory);
}

function hasField(obj: unknown, field: string): boolean {
  return typeof obj === "object" && obj !== null && field in obj;
}
