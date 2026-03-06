import type { DetectedVersions } from "@fixonce/shared";

/**
 * Apply version filtering to a list of memories.
 * Used as a post-filter because the Supabase JS client cannot express
 * the full JSONB predicate for OR-within/AND-across version matching.
 *
 * Rules:
 * - null version_predicates = matches all environments
 * - Missing key for a component = no constraint on that component
 * - OR within a component (any listed version matches)
 * - AND across components (all constrained components must match)
 */
export function filterByVersionPredicates<
  T extends { version_predicates: Record<string, string[]> | null },
>(memories: T[], detectedVersions: DetectedVersions): T[] {
  if (Object.keys(detectedVersions).length === 0) return memories;

  return memories.filter((memory) => {
    if (!memory.version_predicates) return true;

    return Object.entries(detectedVersions).every(([key, detectedVersion]) => {
      const allowedVersions = memory.version_predicates?.[key];
      if (!allowedVersions) return true;
      if (typeof detectedVersion !== "string") return true;
      return allowedVersions.includes(detectedVersion);
    });
  });
}
