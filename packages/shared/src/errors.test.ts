import { describe, it, expect } from "vitest";
import {
  FixOnceError,
  validationError,
  storageError,
  qualityGateError,
  duplicateDetectionError,
  searchError,
  rewriteError,
  rerankError,
  embeddingError,
} from "./errors.js";

describe("FixOnceError", () => {
  it("is an instance of Error", () => {
    const err = new FixOnceError({
      stage: "test",
      reason: "something broke",
      suggestion: "fix it",
    });
    expect(err).toBeInstanceOf(Error);
    expect(err.name).toBe("FixOnceError");
  });

  it("exposes stage, message, and suggestion", () => {
    const err = new FixOnceError({
      stage: "validation",
      reason: "bad input",
      suggestion: "check your input",
    });
    expect(err.stage).toBe("validation");
    expect(err.message).toBe("bad input");
    expect(err.suggestion).toBe("check your input");
  });

  it("serializes to JSON with toJSON()", () => {
    const err = new FixOnceError({
      stage: "storage",
      reason: "write failed",
      suggestion: "retry",
    });
    expect(err.toJSON()).toEqual({
      stage: "storage",
      reason: "write failed",
      suggestion: "retry",
    });
  });
});

describe("error factory functions", () => {
  const factories = [
    { fn: validationError, stage: "validation" },
    { fn: storageError, stage: "storage" },
    { fn: qualityGateError, stage: "quality_gate" },
    { fn: duplicateDetectionError, stage: "duplicate_detection" },
    { fn: searchError, stage: "search" },
    { fn: rewriteError, stage: "rewrite" },
    { fn: rerankError, stage: "rerank" },
    { fn: embeddingError, stage: "embedding" },
  ] as const;

  for (const { fn, stage } of factories) {
    it(`${fn.name}() creates error with stage "${stage}"`, () => {
      const err = fn("reason", "suggestion");
      expect(err).toBeInstanceOf(FixOnceError);
      expect(err.stage).toBe(stage);
      expect(err.message).toBe("reason");
      expect(err.suggestion).toBe("suggestion");
    });
  }
});
