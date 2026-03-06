import { describe, it, expect, vi } from "vitest";
import type { Memory } from "@fixonce/shared";

vi.mock("@fixonce/storage", () => ({
  listFeedbackByMemoryId: vi.fn(),
}));

import { projectSmall, projectMedium } from "./projections.js";

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "550e8400-e29b-41d4-a716-446655440000",
    title: "Test Memory",
    content: "Full content here.",
    summary: "Short summary.",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "human",
    source_url: "https://example.com",
    tags: ["typescript"],
    language: "typescript",
    version_predicates: null,
    project_name: "fixonce",
    project_repo_url: "https://github.com/devrel-ai/fixonce",
    project_workspace_path: "/workspace",
    confidence: 0.8,
    surfaced_count: 5,
    last_surfaced_at: "2025-01-01T00:00:00Z",
    enabled: true,
    created_at: "2024-01-01T00:00:00Z",
    updated_at: "2024-06-01T00:00:00Z",
    embedding: null,
    ...overrides,
  };
}

describe("projectSmall", () => {
  it("returns only the small projection fields", () => {
    const memory = makeMemory();
    const result = projectSmall(memory, 0.95);

    expect(result).toEqual({
      id: memory.id,
      title: memory.title,
      content: memory.content,
      summary: memory.summary,
      memory_type: memory.memory_type,
      relevancy_score: 0.95,
    });
  });

  it("does not include medium/large fields", () => {
    const result = projectSmall(makeMemory(), 0.5);
    expect(result).not.toHaveProperty("tags");
    expect(result).not.toHaveProperty("language");
    expect(result).not.toHaveProperty("source_url");
    expect(result).not.toHaveProperty("confidence");
  });
});

describe("projectMedium", () => {
  it("includes small fields plus medium-specific fields", () => {
    const memory = makeMemory();
    const result = projectMedium(memory, 0.85);

    expect(result.id).toBe(memory.id);
    expect(result.relevancy_score).toBe(0.85);
    expect(result.tags).toEqual(["typescript"]);
    expect(result.language).toBe("typescript");
    expect(result.version_predicates).toBeNull();
    expect(result.created_by).toBe("human");
    expect(result.source_type).toBe("discovery");
    expect(result.created_at).toBe("2024-01-01T00:00:00Z");
    expect(result.updated_at).toBe("2024-06-01T00:00:00Z");
  });

  it("does not include large-only fields", () => {
    const result = projectMedium(makeMemory(), 0.5);
    expect(result).not.toHaveProperty("source_url");
    expect(result).not.toHaveProperty("confidence");
    expect(result).not.toHaveProperty("surfaced_count");
    expect(result).not.toHaveProperty("feedback_summary");
  });
});
