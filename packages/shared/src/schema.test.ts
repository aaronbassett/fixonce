import { describe, expect, it } from "vitest";
import { randomUUID } from "node:crypto";
import {
  MemoryTypeSchema,
  SourceTypeSchema,
  CreatedBySchema,
  CreatedByInputSchema,
  FeedbackTagSchema,
  SuggestedActionSchema,
  OperationTypeSchema,
  SearchTypeSchema,
  VerbositySchema,
  ComponentKeySchema,
  VersionPredicatesSchema,
  DetectedVersionsSchema,
  CreateMemoryInputSchema,
  QueryMemoriesInputSchema,
  ExpandCacheKeyInputSchema,
  GetMemoryInputSchema,
  UpdateMemoryInputSchema,
  SubmitFeedbackInputSchema,
  DetectEnvironmentInputSchema,
} from "./schema.js";

// ---------------------------------------------------------------------------
// Enum schemas
// ---------------------------------------------------------------------------

describe("MemoryTypeSchema", () => {
  it.each(["guidance", "anti_pattern"])("accepts '%s'", (v) => {
    expect(MemoryTypeSchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => MemoryTypeSchema.parse("unknown")).toThrow();
  });
});

describe("SourceTypeSchema", () => {
  it.each(["correction", "discovery", "instruction"])("accepts '%s'", (v) => {
    expect(SourceTypeSchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => SourceTypeSchema.parse("manual")).toThrow();
  });
});

describe("CreatedBySchema", () => {
  it.each(["ai", "human", "human_modified"])("accepts '%s'", (v) => {
    expect(CreatedBySchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => CreatedBySchema.parse("bot")).toThrow();
  });
});

describe("CreatedByInputSchema", () => {
  it.each(["ai", "human"])("accepts '%s'", (v) => {
    expect(CreatedByInputSchema.parse(v)).toBe(v);
  });

  it("rejects 'human_modified' (not valid for input)", () => {
    expect(() => CreatedByInputSchema.parse("human_modified")).toThrow();
  });
});

describe("FeedbackTagSchema", () => {
  const validTags = [
    "helpful",
    "not_helpful",
    "damaging",
    "accurate",
    "somewhat_accurate",
    "somewhat_inaccurate",
    "inaccurate",
    "outdated",
  ];

  it.each(validTags)("accepts '%s'", (v) => {
    expect(FeedbackTagSchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => FeedbackTagSchema.parse("spam")).toThrow();
  });
});

describe("SuggestedActionSchema", () => {
  it.each(["keep", "remove", "fix"])("accepts '%s'", (v) => {
    expect(SuggestedActionSchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => SuggestedActionSchema.parse("delete")).toThrow();
  });
});

describe("OperationTypeSchema", () => {
  it.each(["query", "create", "update", "feedback", "detect"])(
    "accepts '%s'",
    (v) => {
      expect(OperationTypeSchema.parse(v)).toBe(v);
    },
  );

  it("rejects invalid value", () => {
    expect(() => OperationTypeSchema.parse("remove")).toThrow();
  });
});

describe("SearchTypeSchema", () => {
  it.each(["simple", "vector", "hybrid"])("accepts '%s'", (v) => {
    expect(SearchTypeSchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => SearchTypeSchema.parse("fuzzy")).toThrow();
  });
});

describe("VerbositySchema", () => {
  it.each(["small", "medium", "large"])("accepts '%s'", (v) => {
    expect(VerbositySchema.parse(v)).toBe(v);
  });

  it("rejects invalid value", () => {
    expect(() => VerbositySchema.parse("verbose")).toThrow();
  });
});

describe("ComponentKeySchema", () => {
  const validKeys = [
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

  it.each(validKeys)("accepts '%s'", (v) => {
    expect(ComponentKeySchema.parse(v)).toBe(v);
  });

  it("rejects invalid key", () => {
    expect(() => ComponentKeySchema.parse("unknown_component")).toThrow();
  });
});

// ---------------------------------------------------------------------------
// VersionPredicatesSchema
// ---------------------------------------------------------------------------

describe("VersionPredicatesSchema", () => {
  it("accepts valid predicates", () => {
    const input = { node: [">=1.0.0", "<2.0.0"], ledger: ["^3.0.0"] };
    expect(VersionPredicatesSchema.parse(input)).toEqual(input);
  });

  it("accepts undefined", () => {
    expect(VersionPredicatesSchema.parse(undefined)).toBeUndefined();
  });

  it("accepts null", () => {
    expect(VersionPredicatesSchema.parse(null)).toBeNull();
  });

  it("rejects invalid component keys", () => {
    expect(() =>
      VersionPredicatesSchema.parse({ bad_key: [">=1.0.0"] }),
    ).toThrow();
  });

  it("rejects non-string array values", () => {
    expect(() => VersionPredicatesSchema.parse({ node: [123] })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// DetectedVersionsSchema
// ---------------------------------------------------------------------------

describe("DetectedVersionsSchema", () => {
  it("accepts valid detected versions", () => {
    const input = { node: "1.2.3", ledger: "4.5.6" };
    expect(DetectedVersionsSchema.parse(input)).toEqual(input);
  });

  it("accepts undefined", () => {
    expect(DetectedVersionsSchema.parse(undefined)).toBeUndefined();
  });

  it("rejects invalid component keys", () => {
    expect(() =>
      DetectedVersionsSchema.parse({ invalid_key: "1.0.0" }),
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// CreateMemoryInputSchema
// ---------------------------------------------------------------------------

const validCreateInput = {
  title: "Test memory",
  content: "Some useful content",
  summary: "A short summary",
  memory_type: "guidance",
  source_type: "correction",
  created_by: "ai",
  language: "typescript",
  version_predicates: null,
};

describe("CreateMemoryInputSchema", () => {
  it("accepts valid input with defaults applied", () => {
    const result = CreateMemoryInputSchema.parse(validCreateInput);
    expect(result.tags).toEqual([]);
    expect(result.confidence).toBe(0.5);
  });

  it("accepts full input", () => {
    const full = {
      ...validCreateInput,
      tags: ["a", "b"],
      source_url: "https://example.com",
      version_predicates: { node: [">=1.0.0"] },
      project_name: "my-project",
      project_repo_url: "https://github.com/test/repo",
      project_workspace_path: "/home/user/project",
      confidence: 0.9,
    };
    const result = CreateMemoryInputSchema.parse(full);
    expect(result.confidence).toBe(0.9);
    expect(result.tags).toEqual(["a", "b"]);
  });

  it("rejects empty title", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validCreateInput, title: "" }),
    ).toThrow();
  });

  it("rejects title exceeding max length", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({
        ...validCreateInput,
        title: "x".repeat(501),
      }),
    ).toThrow();
  });

  it("rejects empty content", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validCreateInput, content: "" }),
    ).toThrow();
  });

  it("rejects invalid memory_type", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({
        ...validCreateInput,
        memory_type: "invalid",
      }),
    ).toThrow();
  });

  it("rejects invalid source_url", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({
        ...validCreateInput,
        source_url: "not-a-url",
      }),
    ).toThrow();
  });

  it("rejects confidence out of range", () => {
    expect(() =>
      CreateMemoryInputSchema.parse({
        ...validCreateInput,
        confidence: 1.5,
      }),
    ).toThrow();
    expect(() =>
      CreateMemoryInputSchema.parse({
        ...validCreateInput,
        confidence: -0.1,
      }),
    ).toThrow();
  });

  it("rejects more than 20 tags", () => {
    const tags = Array.from({ length: 21 }, (_, i) => `tag${i}`);
    expect(() =>
      CreateMemoryInputSchema.parse({ ...validCreateInput, tags }),
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// QueryMemoriesInputSchema
// ---------------------------------------------------------------------------

describe("QueryMemoriesInputSchema", () => {
  it("accepts minimal input and applies defaults", () => {
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

  it("rejects max_results below 1", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 0 }),
    ).toThrow();
  });

  it("rejects max_results above 50", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 51 }),
    ).toThrow();
  });

  it("rejects non-integer max_results", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({ query: "test", max_results: 2.5 }),
    ).toThrow();
  });

  it("accepts valid datetime strings", () => {
    const result = QueryMemoriesInputSchema.parse({
      query: "test",
      created_after: "2024-01-01T00:00:00Z",
    });
    expect(result.created_after).toBe("2024-01-01T00:00:00Z");
  });

  it("rejects invalid datetime strings", () => {
    expect(() =>
      QueryMemoriesInputSchema.parse({
        query: "test",
        created_after: "not-a-date",
      }),
    ).toThrow();
  });

  it("accepts version_predicates", () => {
    const result = QueryMemoriesInputSchema.parse({
      query: "test",
      version_predicates: { node: "1.0.0" },
    });
    expect(result.version_predicates).toEqual({ node: "1.0.0" });
  });
});

// ---------------------------------------------------------------------------
// ExpandCacheKeyInputSchema
// ---------------------------------------------------------------------------

describe("ExpandCacheKeyInputSchema", () => {
  it("accepts valid input with default verbosity", () => {
    const result = ExpandCacheKeyInputSchema.parse({ cache_key: "abc123" });
    expect(result.verbosity).toBe("small");
  });

  it("rejects empty cache_key", () => {
    expect(() => ExpandCacheKeyInputSchema.parse({ cache_key: "" })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// GetMemoryInputSchema
// ---------------------------------------------------------------------------

describe("GetMemoryInputSchema", () => {
  it("accepts valid UUID and defaults verbosity to 'large'", () => {
    const id = randomUUID();
    const result = GetMemoryInputSchema.parse({ id });
    expect(result.id).toBe(id);
    expect(result.verbosity).toBe("large");
  });

  it("allows overriding verbosity", () => {
    const result = GetMemoryInputSchema.parse({
      id: randomUUID(),
      verbosity: "small",
    });
    expect(result.verbosity).toBe("small");
  });

  it("rejects invalid UUID", () => {
    expect(() => GetMemoryInputSchema.parse({ id: "not-a-uuid" })).toThrow();
  });
});

// ---------------------------------------------------------------------------
// UpdateMemoryInputSchema
// ---------------------------------------------------------------------------

describe("UpdateMemoryInputSchema", () => {
  it("accepts id-only with no updates", () => {
    const id = randomUUID();
    const result = UpdateMemoryInputSchema.parse({
      id,
      version_predicates: null,
    });
    expect(result.id).toBe(id);
  });

  it("allows partial updates", () => {
    const result = UpdateMemoryInputSchema.parse({
      id: randomUUID(),
      title: "Updated title",
      confidence: 0.8,
      enabled: false,
      version_predicates: null,
    });
    expect(result.title).toBe("Updated title");
    expect(result.confidence).toBe(0.8);
    expect(result.enabled).toBe(false);
  });

  it("rejects invalid UUID", () => {
    expect(() =>
      UpdateMemoryInputSchema.parse({
        id: "bad",
        version_predicates: null,
      }),
    ).toThrow();
  });

  it("rejects empty title when provided", () => {
    expect(() =>
      UpdateMemoryInputSchema.parse({
        id: randomUUID(),
        title: "",
        version_predicates: null,
      }),
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// SubmitFeedbackInputSchema
// ---------------------------------------------------------------------------

describe("SubmitFeedbackInputSchema", () => {
  it("accepts valid input with defaults", () => {
    const memoryId = randomUUID();
    const result = SubmitFeedbackInputSchema.parse({ memory_id: memoryId });
    expect(result.memory_id).toBe(memoryId);
    expect(result.tags).toEqual([]);
  });

  it("accepts full input", () => {
    const result = SubmitFeedbackInputSchema.parse({
      memory_id: randomUUID(),
      text: "Very useful",
      tags: ["helpful", "accurate"],
      suggested_action: "keep",
    });
    expect(result.tags).toEqual(["helpful", "accurate"]);
    expect(result.suggested_action).toBe("keep");
  });

  it("rejects invalid UUID for memory_id", () => {
    expect(() =>
      SubmitFeedbackInputSchema.parse({ memory_id: "not-uuid" }),
    ).toThrow();
  });

  it("rejects invalid feedback tags", () => {
    expect(() =>
      SubmitFeedbackInputSchema.parse({
        memory_id: randomUUID(),
        tags: ["invalid_tag"],
      }),
    ).toThrow();
  });

  it("rejects invalid suggested_action", () => {
    expect(() =>
      SubmitFeedbackInputSchema.parse({
        memory_id: randomUUID(),
        suggested_action: "destroy",
      }),
    ).toThrow();
  });
});

// ---------------------------------------------------------------------------
// DetectEnvironmentInputSchema
// ---------------------------------------------------------------------------

describe("DetectEnvironmentInputSchema", () => {
  it("accepts empty object", () => {
    const result = DetectEnvironmentInputSchema.parse({});
    expect(result.project_path).toBeUndefined();
  });

  it("accepts project_path", () => {
    const result = DetectEnvironmentInputSchema.parse({
      project_path: "/some/path",
    });
    expect(result.project_path).toBe("/some/path");
  });
});
