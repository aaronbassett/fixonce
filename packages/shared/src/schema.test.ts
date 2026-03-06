import { describe, it, expect } from "vitest";
import {
  MemoryTypeSchema,
  SourceTypeSchema,
  CreatedByInputSchema,
  FeedbackTagSchema,
  SuggestedActionSchema,
  ComponentKeySchema,
  VersionPredicatesSchema,
  CreateMemoryInputSchema,
  QueryMemoriesInputSchema,
  SubmitFeedbackInputSchema,
  GetMemoryInputSchema,
  UpdateMemoryInputSchema,
} from "./schema.js";

describe("enum schemas", () => {
  it("MemoryTypeSchema accepts valid values", () => {
    expect(MemoryTypeSchema.parse("guidance")).toBe("guidance");
    expect(MemoryTypeSchema.parse("anti_pattern")).toBe("anti_pattern");
  });

  it("MemoryTypeSchema rejects invalid values", () => {
    expect(() => MemoryTypeSchema.parse("invalid")).toThrow();
  });

  it("SourceTypeSchema accepts valid values", () => {
    for (const v of ["correction", "discovery", "instruction"]) {
      expect(SourceTypeSchema.parse(v)).toBe(v);
    }
  });

  it("CreatedByInputSchema accepts ai and human only", () => {
    expect(CreatedByInputSchema.parse("ai")).toBe("ai");
    expect(CreatedByInputSchema.parse("human")).toBe("human");
    expect(() => CreatedByInputSchema.parse("human_modified")).toThrow();
  });

  it("FeedbackTagSchema accepts all 8 tags", () => {
    const tags = [
      "helpful",
      "not_helpful",
      "damaging",
      "accurate",
      "somewhat_accurate",
      "somewhat_inaccurate",
      "inaccurate",
      "outdated",
    ];
    for (const tag of tags) {
      expect(FeedbackTagSchema.parse(tag)).toBe(tag);
    }
  });

  it("SuggestedActionSchema accepts keep, remove, fix", () => {
    for (const v of ["keep", "remove", "fix"]) {
      expect(SuggestedActionSchema.parse(v)).toBe(v);
    }
  });

  it("ComponentKeySchema accepts all 12 component keys", () => {
    const keys = [
      "network",
      "node",
      "compact_compiler",
      "compact_runtime",
      "compact_js",
      "onchain_runtime",
      "ledger",
      "wallet_sdk",
      "midnight_js",
      "dapp_connector_api",
      "midnight_indexer",
      "proof_server",
    ];
    for (const k of keys) {
      expect(ComponentKeySchema.parse(k)).toBe(k);
    }
  });
});

describe("VersionPredicatesSchema", () => {
  it("accepts valid version predicates", () => {
    const result = VersionPredicatesSchema.parse({
      compact_compiler: ["0.14.0", "0.15.0"],
    });
    expect(result).toEqual({ compact_compiler: ["0.14.0", "0.15.0"] });
  });

  it("accepts null and undefined", () => {
    expect(VersionPredicatesSchema.parse(null)).toBeNull();
    expect(VersionPredicatesSchema.parse(undefined)).toBeUndefined();
  });

  it("rejects invalid component keys", () => {
    expect(() =>
      VersionPredicatesSchema.parse({ invalid_key: ["1.0.0"] }),
    ).toThrow();
  });
});

describe("CreateMemoryInputSchema", () => {
  const validInput = {
    title: "Test Memory",
    content: "This is the content of the memory.",
    summary: "A short summary.",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "human",
    language: "typescript",
    version_predicates: null,
  };

  it("parses valid input with defaults applied", () => {
    const result = CreateMemoryInputSchema.parse(validInput);
    expect(result.tags).toEqual([]);
    expect(result.confidence).toBe(0.5);
  });

  it("rejects empty title", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, title: "" }),
    ).toThrow();
  });

  it("rejects title over 500 chars", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, title: "x".repeat(501) }),
    ).toThrow();
  });

  it("rejects confidence out of range", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, confidence: 1.5 }),
    ).toThrow();
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, confidence: -0.1 }),
    ).toThrow();
  });

  it("accepts valid optional fields", () => {
    const result = CreateMemoryInputSchema.parse({
      ...validInput,
      tags: ["typescript", "react"],
      source_url: "https://example.com",
      confidence: 0.9,
      project_name: "fixonce",
    });
    expect(result.tags).toEqual(["typescript", "react"]);
    expect(result.source_url).toBe("https://example.com");
    expect(result.confidence).toBe(0.9);
  });

  it("rejects invalid source_url", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validInput, source_url: "not-a-url" }),
    ).toThrow();
  });
});

describe("QueryMemoriesInputSchema", () => {
  it("applies defaults", () => {
    const result = QueryMemoriesInputSchema.parse({ query: "test" });
    expect(result.rewrite).toBe(true);
    expect(result.type).toBe("hybrid");
    expect(result.rerank).toBe(true);
    expect(result.max_results).toBe(5);
    expect(result.verbosity).toBe("small");
  });

  it("rejects empty query", () => {
    expect(() => QueryMemoriesInputSchema.parse({ query: "" })).toThrow();
  });

  it("rejects max_results out of range", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 0 }),
    ).toThrow();
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 51 }),
    ).toThrow();
  });
});

describe("SubmitFeedbackInputSchema", () => {
  it("requires valid UUID for memory_id", () => {
    expect(() =>
      SubmitFeedbackInputSchema.parse({ memory_id: "not-a-uuid" }),
    ).toThrow();
  });

  it("accepts minimal valid input", () => {
    const result = SubmitFeedbackInputSchema.parse({
      memory_id: "550e8400-e29b-41d4-a716-446655440000",
    });
    expect(result.tags).toEqual([]);
  });
});

describe("GetMemoryInputSchema", () => {
  it("defaults verbosity to large", () => {
    const result = GetMemoryInputSchema.parse({
      id: "550e8400-e29b-41d4-a716-446655440000",
    });
    expect(result.verbosity).toBe("large");
  });
});

describe("UpdateMemoryInputSchema", () => {
  it("allows partial updates", () => {
    const result = UpdateMemoryInputSchema.parse({
      id: "550e8400-e29b-41d4-a716-446655440000",
      title: "Updated Title",
    });
    expect(result.title).toBe("Updated Title");
    expect(result.content).toBeUndefined();
  });
});
