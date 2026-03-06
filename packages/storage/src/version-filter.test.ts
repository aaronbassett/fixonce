import { describe, it, expect } from "vitest";
import { filterByVersionPredicates } from "./version-filter.js";

interface TestMemory {
  id: string;
  version_predicates: Record<string, string[]> | null;
}

function makeMemory(
  id: string,
  predicates: Record<string, string[]> | null,
): TestMemory {
  return { id, version_predicates: predicates };
}

describe("filterByVersionPredicates", () => {
  it("returns all memories when detectedVersions is empty", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0"] }),
      makeMemory("2", { compact_compiler: ["0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {});
    expect(result).toHaveLength(2);
  });

  it("includes memories with null version_predicates (universal)", () => {
    const memories = [
      makeMemory("1", null),
      makeMemory("2", { compact_compiler: ["0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.14.0",
    });
    expect(result).toHaveLength(1);
    expect(result[0].id).toBe("1");
  });

  it("matches when detected version is in allowed list (OR within component)", () => {
    const memories = [
      makeMemory("1", { compact_compiler: ["0.14.0", "0.15.0"] }),
    ];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.15.0",
    });
    expect(result).toHaveLength(1);
  });

  it("excludes when detected version is not in allowed list", () => {
    const memories = [makeMemory("1", { compact_compiler: ["0.14.0"] })];
    const result = filterByVersionPredicates(memories, {
      compact_compiler: "0.16.0",
    });
    expect(result).toHaveLength(0);
  });

  it("requires all constrained components to match (AND across components)", () => {
    const memories = [
      makeMemory("1", {
        compact_compiler: ["0.14.0"],
        wallet_sdk: ["1.0.0"],
      }),
    ];
    expect(
      filterByVersionPredicates(memories, {
        compact_compiler: "0.14.0",
        wallet_sdk: "1.0.0",
      }),
    ).toHaveLength(1);
    expect(
      filterByVersionPredicates(memories, {
        compact_compiler: "0.14.0",
        wallet_sdk: "2.0.0",
      }),
    ).toHaveLength(0);
  });

  it("missing key in predicates means no constraint on that component", () => {
    const memories = [makeMemory("1", { compact_compiler: ["0.14.0"] })];
    const result = filterByVersionPredicates(memories, {
      wallet_sdk: "1.0.0",
    });
    expect(result).toHaveLength(1);
  });
});
