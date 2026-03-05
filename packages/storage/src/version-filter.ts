import type { DetectedVersions } from "@fixonce/shared";

/**
 * Build a Supabase filter string for version predicate matching.
 *
 * Rules:
 * - OR within a component (any listed version matches)
 * - AND across components (all constrained components must match)
 * - Missing key = no constraint on that component
 * - null version_predicates = matches all environments
 *
 * Returns filter conditions to apply to a Supabase query.
 * Since Supabase JS client has limited JSONB support, we build
 * raw filter strings for use with .or() / .filter().
 */
export function buildVersionFilter(versions: DetectedVersions): string {
  const components = Object.entries(versions);
  if (components.length === 0) return "";

  const conditions = components.map(([key, value]) => {
    return (
      `version_predicates.is.null,` +
      `not.version_predicates.cd.{"${key}"},` +
      `version_predicates->${key}.cs.["${value}"]`
    );
  });

  return conditions.join(",");
}

/**
 * Apply version filtering to a list of memories.
 * Used as a post-filter when Supabase query builder cannot express
 * the full JSONB predicate.
 */
export function filterByVersionPredicates<
  T extends { version_predicates: Record<string, string[]> | null },
>(memories: T[], detectedVersions: DetectedVersions): T[] {
  if (Object.keys(detectedVersions).length === 0) return memories;

  return memories.filter((memory) => {
    if (!memory.version_predicates) return true;

    return Object.entries(detectedVersions).every(
      ([key, detectedVersion]) => {
        const allowedVersions =
          memory.version_predicates?.[
            key as keyof typeof memory.version_predicates
          ];
        if (!allowedVersions) return true;
        if (typeof detectedVersion !== "string") return true;
        return allowedVersions.includes(detectedVersion);
      },
    );
  });
}
