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
      mem("a", { compact_compiler: ["0.14.0"] }),
      mem("b", null),
      mem("c", { wallet_sdk: ["1.0.0", "2.0.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {});
    expect(result).toEqual(memories);
  });

  it("includes memories with null version_predicates (universal)", () => {
    const memories = [
      mem("a", null),
      mem("b", { compact_compiler: ["0.14.0"] }),
    ];
    const detected: DetectedVersions = { compact_compiler: "0.15.0" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[0]]);
  });

  it("matches when detected version is in allowed list (OR within component)", () => {
    const memories = [
      mem("a", { compact_compiler: ["0.13.0", "0.14.0", "0.15.0"] }),
      mem("b", { compact_compiler: ["0.14.0"] }),
    ];
    const detected: DetectedVersions = { compact_compiler: "0.14.0" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual(memories);
  });

  it("excludes when detected version is not in allowed list", () => {
    const memories = [
      mem("a", { compact_compiler: ["0.13.0"] }),
      mem("b", { compact_compiler: ["0.14.0", "0.15.0"] }),
    ];
    const detected: DetectedVersions = { compact_compiler: "0.14.0" };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[1]]);
  });

  it("requires all constrained components to match (AND across components)", () => {
    const memories = [
      mem("a", { compact_compiler: ["0.14.0"], wallet_sdk: ["1.0.0"] }),
      mem("b", { compact_compiler: ["0.14.0"], wallet_sdk: ["2.0.0"] }),
    ];
    const detected: DetectedVersions = {
      compact_compiler: "0.14.0",
      wallet_sdk: "1.0.0",
    };
    const result = filterByVersionPredicates(memories, detected);
    expect(result).toEqual([memories[0]]);
  });

  it("missing key in predicates means no constraint on that component", () => {
    const memories = [
      mem("a", { compact_compiler: ["0.14.0"] }),
      mem("b", { compact_compiler: ["0.14.0"], wallet_sdk: ["1.0.0"] }),
    ];
    const detected: DetectedVersions = {
      compact_compiler: "0.14.0",
      wallet_sdk: "2.0.0",
    };
    const result = filterByVersionPredicates(memories, detected);
    // "a" has no wallet_sdk constraint so it passes; "b" requires wallet_sdk=1.0.0 so it fails
    expect(result).toEqual([memories[0]]);
  });
});
