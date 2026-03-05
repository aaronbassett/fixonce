import type {
  Memory,
  MemorySmall,
  MemoryMedium,
  MemoryLarge,
  FeedbackSummary,
  FeedbackTag,
  SuggestedAction,
  Verbosity,
} from "@fixonce/shared";
import { listFeedbackByMemoryId } from "@fixonce/storage";

export function projectSmall(memory: Memory, relevancyScore: number): MemorySmall {
  return {
    id: memory.id,
    title: memory.title,
    content: memory.content,
    summary: memory.summary,
    memory_type: memory.memory_type,
    relevancy_score: relevancyScore,
  };
}

export function projectMedium(memory: Memory, relevancyScore: number): MemoryMedium {
  return {
    ...projectSmall(memory, relevancyScore),
    tags: memory.tags,
    language: memory.language,
    version_predicates: memory.version_predicates,
    created_by: memory.created_by,
    source_type: memory.source_type,
    created_at: memory.created_at,
    updated_at: memory.updated_at,
  };
}

export async function buildFeedbackSummary(memoryId: string): Promise<FeedbackSummary> {
  const feedback = await listFeedbackByMemoryId(memoryId);
  const tagCounts: Partial<Record<FeedbackTag, number>> = {};
  const flaggedActions: SuggestedAction[] = [];

  for (const fb of feedback) {
    for (const tag of fb.tags) {
      tagCounts[tag] = (tagCounts[tag] ?? 0) + 1;
    }
    if (fb.suggested_action && (fb.suggested_action === "remove" || fb.suggested_action === "fix")) {
      flaggedActions.push(fb.suggested_action);
    }
  }

  return {
    total_count: feedback.length,
    tag_counts: tagCounts,
    flagged_actions: flaggedActions,
  };
}

export async function projectLarge(memory: Memory, relevancyScore: number): Promise<MemoryLarge> {
  const feedbackSummary = await buildFeedbackSummary(memory.id);
  return {
    ...projectMedium(memory, relevancyScore),
    source_url: memory.source_url,
    project_name: memory.project_name,
    project_repo_url: memory.project_repo_url,
    project_workspace_path: memory.project_workspace_path,
    confidence: memory.confidence,
    surfaced_count: memory.surfaced_count,
    last_surfaced_at: memory.last_surfaced_at,
    feedback_summary: feedbackSummary,
  };
}

export async function projectByVerbosity(
  memory: Memory,
  relevancyScore: number,
  verbosity: Verbosity,
): Promise<MemorySmall | MemoryMedium | MemoryLarge> {
  switch (verbosity) {
    case "small":
      return projectSmall(memory, relevancyScore);
    case "medium":
      return projectMedium(memory, relevancyScore);
    case "large":
      return projectLarge(memory, relevancyScore);
  }
}
