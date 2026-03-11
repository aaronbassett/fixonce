import { describe, it, expect, vi } from "vitest";
import type { Memory } from "@fixonce/shared";

vi.mock("@fixonce/storage", () => ({
  listFeedbackByMemoryId: vi.fn().mockResolvedValue([]),
}));

import { projectSmall, projectMedium } from "./projections.js";

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "mem-001",
    title: "Test Memory",
    content: "Some content here",
    summary: "A short summary",
    memory_type: "howto",
    source_type: "cli",
    created_by: "human",
    source_url: "https://example.com",
    tags: ["typescript", "testing"],
    language: "en",
    version_predicates: null,
    project_name: "fixonce",
    project_repo_url: "https://github.com/org/fixonce",
    project_workspace_path: "/workspace",
    confidence: 0.95,
    surfaced_count: 3,
    last_surfaced_at: "2026-01-15T00:00:00Z",
    enabled: true,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-10T00:00:00Z",
    embedding: null,
    ...overrides,
  };
}

const SMALL_KEYS = [
  "id",
  "title",
  "content",
  "summary",
  "memory_type",
  "relevancy_score",
] as const;

const MEDIUM_EXTRA_KEYS = [
  "tags",
  "language",
  "version_predicates",
  "created_by",
  "source_type",
  "created_at",
  "updated_at",
] as const;

const LARGE_ONLY_KEYS = [
  "source_url",
  "project_name",
  "project_repo_url",
  "project_workspace_path",
  "confidence",
  "surfaced_count",
  "last_surfaced_at",
  "feedback_summary",
] as const;

describe("projectSmall", () => {
  it("returns only small projection fields", () => {
    const memory = makeMemory();
    const result = projectSmall(memory, 0.87);

    expect(result).toEqual({
      id: "mem-001",
      title: "Test Memory",
      content: "Some content here",
      summary: "A short summary",
      memory_type: "howto",
      relevancy_score: 0.87,
    });
  });

  it("contains exactly the expected keys", () => {
    const result = projectSmall(makeMemory(), 0.5);
    expect(Object.keys(result).sort()).toEqual([...SMALL_KEYS].sort());
  });

  it("does not include medium or large fields", () => {
    const result = projectSmall(makeMemory(), 0.5);
    for (const key of [...MEDIUM_EXTRA_KEYS, ...LARGE_ONLY_KEYS]) {
      expect(result).not.toHaveProperty(key);
    }
  });

  it("uses the provided relevancy score, not a memory field", () => {
    const result = projectSmall(makeMemory(), 0.42);
    expect(result.relevancy_score).toBe(0.42);
  });
});

describe("projectMedium", () => {
  it("returns small fields plus medium-specific fields", () => {
    const memory = makeMemory();
    const result = projectMedium(memory, 0.73);

    expect(result).toEqual({
      id: "mem-001",
      title: "Test Memory",
      content: "Some content here",
      summary: "A short summary",
      memory_type: "howto",
      relevancy_score: 0.73,
      tags: ["typescript", "testing"],
      language: "en",
      version_predicates: null,
      created_by: "human",
      source_type: "cli",
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-10T00:00:00Z",
    });
  });

  it("contains exactly small + medium keys", () => {
    const result = projectMedium(makeMemory(), 0.5);
    const expectedKeys = [...SMALL_KEYS, ...MEDIUM_EXTRA_KEYS];
    expect(Object.keys(result).sort()).toEqual([...expectedKeys].sort());
  });

  it("does not include large-only fields", () => {
    const result = projectMedium(makeMemory(), 0.5);
    for (const key of LARGE_ONLY_KEYS) {
      expect(result).not.toHaveProperty(key);
    }
  });

  it("preserves array and nullable fields from memory", () => {
    const memory = makeMemory({
      tags: [],
      version_predicates: { node: ">=18" },
    });
    const result = projectMedium(memory, 0.6);

    expect(result.tags).toEqual([]);
    expect(result.version_predicates).toEqual({ node: ">=18" });
  });
});
