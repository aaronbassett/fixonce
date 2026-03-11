import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Memory } from "@fixonce/shared";

const mockFtsSearch = vi.fn();
const mockHybridSearch = vi.fn();
const mockVectorSearch = vi.fn();
const mockGenerateEmbedding = vi.fn();
const mockFilterByVersionPredicates = vi.fn();
const mockIncrementSurfacedCount = vi.fn().mockResolvedValue(undefined);
const mockListFeedbackByMemoryId = vi.fn().mockResolvedValue([]);

vi.mock("@fixonce/storage", () => ({
  ftsSearch: (...args: unknown[]) => mockFtsSearch(...args),
  hybridSearch: (...args: unknown[]) => mockHybridSearch(...args),
  vectorSearch: (...args: unknown[]) => mockVectorSearch(...args),
  generateEmbedding: (...args: unknown[]) => mockGenerateEmbedding(...args),
  filterByVersionPredicates: (...args: unknown[]) =>
    mockFilterByVersionPredicates(...args),
  incrementSurfacedCount: (...args: unknown[]) =>
    mockIncrementSurfacedCount(...args),
  listFeedbackByMemoryId: (...args: unknown[]) =>
    mockListFeedbackByMemoryId(...args),
}));

import { executeReadPipeline } from "./index.js";

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "mem-001",
    title: "Test Memory",
    content: "Some content here",
    summary: "A short summary",
    memory_type: "guidance",
    source_type: "discovery",
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

describe("executeReadPipeline", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("project_name filtering", () => {
    it("passes project_name to ftsSearch", async () => {
      const memories = [makeMemory({ project_name: "my-project" })];
      mockFtsSearch.mockResolvedValue(memories);

      await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        project_name: "my-project",
      });

      expect(mockFtsSearch).toHaveBeenCalledOnce();
      const opts = mockFtsSearch.mock.calls[0]?.[0] as Record<string, unknown>;
      expect(opts.project_name).toBe("my-project");
    });

    it("passes project_name to hybridSearch", async () => {
      const memories = [makeMemory({ project_name: "my-project" })];
      mockGenerateEmbedding.mockResolvedValue([0.1, 0.2]);
      mockHybridSearch.mockResolvedValue(memories);

      await executeReadPipeline({
        query: "test query",
        type: "hybrid",
        rewrite: false,
        rerank: false,
        project_name: "my-project",
      });

      expect(mockHybridSearch).toHaveBeenCalledOnce();
      const opts = mockHybridSearch.mock.calls[0]?.[0] as Record<
        string,
        unknown
      >;
      expect(opts.project_name).toBe("my-project");
    });

    it("passes project_name to vectorSearch", async () => {
      const memories = [makeMemory({ project_name: "my-project" })];
      mockGenerateEmbedding.mockResolvedValue([0.1, 0.2]);
      mockVectorSearch.mockResolvedValue(memories);

      await executeReadPipeline({
        query: "test query",
        type: "vector",
        rewrite: false,
        rerank: false,
        project_name: "my-project",
      });

      expect(mockVectorSearch).toHaveBeenCalledOnce();
      const opts = mockVectorSearch.mock.calls[0]?.[0] as Record<
        string,
        unknown
      >;
      expect(opts.project_name).toBe("my-project");
    });

    it("does not include project_name when not provided", async () => {
      mockFtsSearch.mockResolvedValue([]);

      await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
      });

      const opts = mockFtsSearch.mock.calls[0]?.[0] as Record<string, unknown>;
      expect(opts.project_name).toBeUndefined();
    });
  });

  describe("max_tokens budget", () => {
    it("limits results when max_tokens is exceeded", async () => {
      const memories = [
        makeMemory({ id: "mem-1", content: "A".repeat(400) }),
        makeMemory({ id: "mem-2", content: "B".repeat(400) }),
        makeMemory({ id: "mem-3", content: "C".repeat(400) }),
      ];
      mockFtsSearch.mockResolvedValue(memories);

      // Calculate a realistic budget that fits only the first memory
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { embedding, ...firstWithoutEmbedding } = memories[0];
      const singleEstimate = Math.ceil(
        JSON.stringify(firstWithoutEmbedding).length / 4,
      );

      const result = await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        max_results: 10,
        max_tokens: singleEstimate + 1,
      });

      expect(result.results.length).toBeLessThan(memories.length);
      expect(result.overflow.length).toBeGreaterThan(0);
    });

    it("returns all results when max_tokens is large enough", async () => {
      const memories = [
        makeMemory({ id: "mem-1" }),
        makeMemory({ id: "mem-2" }),
      ];
      mockFtsSearch.mockResolvedValue(memories);

      const result = await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        max_results: 10,
        max_tokens: 999999,
      });

      expect(result.results.length).toBe(2);
      expect(result.overflow.length).toBe(0);
    });

    it("uses the more restrictive of max_results and max_tokens", async () => {
      const memories = [
        makeMemory({ id: "mem-1" }),
        makeMemory({ id: "mem-2" }),
        makeMemory({ id: "mem-3" }),
      ];
      mockFtsSearch.mockResolvedValue(memories);

      const result = await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        max_results: 1,
        max_tokens: 999999,
      });

      expect(result.results.length).toBe(1);
    });

    it("pushes excess results to overflow when max_tokens is tight", async () => {
      const memories = [
        makeMemory({ id: "mem-1", content: "short" }),
        makeMemory({ id: "mem-2", content: "X".repeat(2000) }),
        makeMemory({ id: "mem-3", content: "Y".repeat(2000) }),
      ];
      mockFtsSearch.mockResolvedValue(memories);

      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { embedding, ...firstWithoutEmbedding } = memories[0];
      const singleTokenEstimate = Math.ceil(
        JSON.stringify(firstWithoutEmbedding).length / 4,
      );

      const result = await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        max_results: 10,
        max_tokens: singleTokenEstimate + 1,
      });

      expect(result.results.length).toBe(1);
      expect(result.overflow.length).toBe(2);
    });

    it("does not constrain results when max_tokens is not set", async () => {
      const memories = [
        makeMemory({ id: "mem-1" }),
        makeMemory({ id: "mem-2" }),
        makeMemory({ id: "mem-3" }),
      ];
      mockFtsSearch.mockResolvedValue(memories);

      const result = await executeReadPipeline({
        query: "test query",
        type: "simple",
        rewrite: false,
        rerank: false,
        max_results: 10,
      });

      expect(result.results.length).toBe(3);
    });
  });
});
