import { describe, expect, it } from "vitest";
import { filterByVersionPredicates } from "./version-filter.js";
import type { DetectedVersions } from "@fixonce/shared";

interface TestMemory {
  id: string;
  version_predicates: Record<string, string[]> | null;
}

function mem(
  id: string,
  predicates: Record<string, string[]> | null,
): TestMemory {
  return { id, version_predicates: predicates };
}

describe("filterByVersionPredicates", () => {
  it("returns all memories when detectedVersions is empty", () => {
    const memories = [
      mem("a", { react: ["18"] }),
      mem("b", null),
      mem("c", { node: ["20", "22"] }),
    ];
    const result = filterByVersionPredicates(memories, {});
    expect(result).toEqual(memories);
  });

  it("includes memories with null version_predicates (universal)", () => {
    const memories = [
      mem("a", null),
      mem("b", { react: ["18"] }),
    ];
    const detected: DetectedVersions = { react: "19" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[0]]);
  });

  it("matches when detected version is in allowed list (OR within component)", () => {
    const memories = [
      mem("a", { react: ["17", "18", "19"] }),
      mem("b", { react: ["18"] }),
    ];
    const detected: DetectedVersions = { react: "18" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual(memories);
  });

  it("excludes when detected version is not in allowed list", () => {
    const memories = [
      mem("a", { react: ["17"] }),
      mem("b", { react: ["18", "19"] }),
    ];
    const detected: DetectedVersions = { react: "18" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[1]]);
  });

  it("requires all constrained components to match (AND across components)", () => {
    const memories = [
      mem("a", { react: ["18"], node: ["20"] }),
      mem("b", { react: ["18"], node: ["22"] }),
    ];
    const detected: DetectedVersions = { react: "18", node: "20" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[0]]);
  });

  it("missing key in predicates means no constraint on that component", () => {
    const memories = [
      mem("a", { react: ["18"] }),
      mem("b", { react: ["18"], node: ["20"] }),
    ];
    const detected: DetectedVersions = { react: "18", node: "22" };
    const result = filterByVersionPredicates(memories, detected);
    // "a" has no node constraint so it passes; "b" requires node=20 so it fails
    expect(result).toEqual([memories[0]]);
  });
});
