import { describe, it, expect, beforeEach, vi } from "vitest";
import type { Memory, QueryMemoriesResult } from "@fixonce/shared";

vi.mock("@fixonce/storage", () => ({
  getMemoryById: vi.fn(),
  updateMemory: vi.fn(),
  generateEmbedding: vi.fn(),
  createFeedback: vi.fn(),
}));

vi.mock("@fixonce/activity", () => ({
  logActivity: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("./write/index.js", () => ({
  executeWritePipeline: vi.fn(),
}));

vi.mock("./read/index.js", () => ({
  executeReadPipeline: vi.fn(),
}));

import {
  getMemoryById,
  updateMemory as storageUpdateMemory,
  generateEmbedding,
} from "@fixonce/storage";
import { logActivity } from "@fixonce/activity";
import { executeWritePipeline } from "./write/index.js";
import { executeReadPipeline } from "./read/index.js";
import { updateMemory, queryMemories, createMemory } from "./service.js";

const mockGetMemoryById = vi.mocked(getMemoryById);
const mockStorageUpdateMemory = vi.mocked(storageUpdateMemory);
const mockGenerateEmbedding = vi.mocked(generateEmbedding);
const mockLogActivity = vi.mocked(logActivity);
const mockExecuteWritePipeline = vi.mocked(executeWritePipeline);
const mockExecuteReadPipeline = vi.mocked(executeReadPipeline);

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "00000000-0000-4000-8000-000000000001",
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

function makeReadPipelineResult(
  overrides?: Partial<QueryMemoriesResult>,
): QueryMemoriesResult {
  return {
    results: [],
    overflow: [],
    total_found: 0,
    pipeline: {
      rewrite_used: false,
      search_type: "simple",
      rerank_used: false,
    },
    ...overrides,
  };
}

describe("updateMemory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGenerateEmbedding.mockResolvedValue([0.1, 0.2, 0.3]);
  });

  it("triggers embedding regeneration when title changes", async () => {
    const existing = makeMemory({ title: "Old Title" });
    const updated = makeMemory({ title: "New Title" });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      title: "New Title",
    });

    expect(mockGenerateEmbedding).toHaveBeenCalledOnce();
    expect(result.embedding_regenerating).toBe(true);
  });

  it("does NOT trigger embedding regeneration when title is unchanged", async () => {
    const existing = makeMemory({ title: "Same Title" });
    const updated = makeMemory({ title: "Same Title" });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      title: "Same Title",
    });

    expect(mockGenerateEmbedding).not.toHaveBeenCalled();
    expect(result.embedding_regenerating).toBe(false);
  });

  it("triggers embedding regeneration when content changes", async () => {
    const existing = makeMemory({ content: "Old content" });
    const updated = makeMemory({ content: "New content" });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      content: "New content",
    });

    expect(mockGenerateEmbedding).toHaveBeenCalledOnce();
    expect(result.embedding_regenerating).toBe(true);
  });

  it("triggers embedding regeneration when summary changes", async () => {
    const existing = makeMemory({ summary: "Old summary" });
    const updated = makeMemory({ summary: "New summary" });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      summary: "New summary",
    });

    expect(mockGenerateEmbedding).toHaveBeenCalledOnce();
    expect(result.embedding_regenerating).toBe(true);
  });

  it("does NOT trigger embedding regeneration when no content fields change", async () => {
    const existing = makeMemory();
    const updated = makeMemory({ confidence: 0.8 });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      confidence: 0.8,
    });

    expect(mockGenerateEmbedding).not.toHaveBeenCalled();
    expect(result.embedding_regenerating).toBe(false);
  });

  it("throws when memory is not found", async () => {
    mockGetMemoryById.mockResolvedValue(null);

    await expect(
      updateMemory({ id: "00000000-0000-4000-8000-000000000001", title: "X" }),
    ).rejects.toThrow();

    expect(mockStorageUpdateMemory).not.toHaveBeenCalled();
    expect(mockGenerateEmbedding).not.toHaveBeenCalled();
  });

  it("catches generateEmbedding failure and does not propagate", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const existing = makeMemory({ title: "Old Title" });
    const updated = makeMemory({ title: "New Title" });

    mockGetMemoryById.mockResolvedValue(existing);
    mockStorageUpdateMemory.mockResolvedValue(updated);
    mockGenerateEmbedding.mockRejectedValue(new Error("Embedding API down"));

    const result = await updateMemory({
      id: "00000000-0000-4000-8000-000000000001",
      title: "New Title",
    });

    expect(result.embedding_regenerating).toBe(true);

    // Wait for the fire-and-forget promise to settle
    await vi.waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        expect.stringContaining("Failed to regenerate embedding"),
        expect.any(Error),
      );
    });

    errorSpy.mockRestore();
  });
});

describe("queryMemories", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("validates input via schema and passes project_name and max_tokens to the read pipeline", async () => {
    const pipelineResult = makeReadPipelineResult({
      results: [
        {
          id: "mem-1",
          title: "Result",
          content: "Result content",
          summary: "Summary",
          memory_type: "guidance",
          relevancy_score: 0.9,
        },
      ],
      total_found: 1,
    });
    mockExecuteReadPipeline.mockResolvedValue(pipelineResult);

    await queryMemories({
      query: "test query",
      project_name: "my-project",
      max_tokens: 5000,
    });

    expect(mockExecuteReadPipeline).toHaveBeenCalledOnce();
    const passedInput = mockExecuteReadPipeline.mock.calls[0]?.[0];
    expect(passedInput).toMatchObject({
      query: "test query",
      project_name: "my-project",
      max_tokens: 5000,
    });
  });

  it("logs activity with correct pipeline metadata", async () => {
    const pipelineResult = makeReadPipelineResult({
      results: [
        {
          id: "mem-1",
          title: "Result",
          content: "Result content",
          summary: "Summary",
          memory_type: "guidance",
          relevancy_score: 0.9,
        },
        {
          id: "mem-2",
          title: "Result 2",
          content: "Result 2 content",
          summary: "Summary 2",
          memory_type: "guidance",
          relevancy_score: 0.8,
        },
      ],
      total_found: 5,
      pipeline: {
        search_type: "hybrid",
        rewrite_used: true,
        rerank_used: true,
      },
    });
    mockExecuteReadPipeline.mockResolvedValue(pipelineResult);

    await queryMemories({ query: "find something" });

    expect(mockLogActivity).toHaveBeenCalledOnce();
    expect(mockLogActivity).toHaveBeenCalledWith("query", {
      query: "find something",
      search_type: "hybrid",
      rewrite_used: true,
      rerank_used: true,
      total_found: 5,
      results_returned: 2,
    });
  });

  it("throws validation error for invalid input", async () => {
    const err = await queryMemories({ query: "" } as never).catch(
      (e: unknown) => e,
    );
    expect(err).toBeDefined();
    expect((err as Error).name).toBe("ZodError");

    expect(mockExecuteReadPipeline).not.toHaveBeenCalled();
  });
});

describe("createMemory", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("validates input, calls write pipeline, then logs activity", async () => {
    mockExecuteWritePipeline.mockResolvedValue({
      status: "created",
      memory: {
        id: "mem-new",
        title: "New Memory",
        created_at: "2026-03-11T00:00:00Z",
      },
      dedup_outcome: "new",
    });

    const result = await createMemory({
      title: "New Memory",
      content: "Content here",
      summary: "Summary here",
      memory_type: "guidance",
      source_type: "discovery",
      created_by: "ai",
      language: "en",
    });

    expect(result.status).toBe("created");
    expect(result.memory?.id).toBe("mem-new");

    expect(mockExecuteWritePipeline).toHaveBeenCalledOnce();
    const pipelineInput = mockExecuteWritePipeline.mock.calls[0]?.[0];
    expect(pipelineInput).toMatchObject({
      title: "New Memory",
      content: "Content here",
      created_by: "ai",
    });

    expect(mockLogActivity).toHaveBeenCalledOnce();
    expect(mockLogActivity).toHaveBeenCalledWith(
      "create",
      {
        status: "created",
        memory_id: "mem-new",
        dedup_outcome: "new",
      },
      "mem-new",
    );
  });

  it("passes created_by: human to the write pipeline unchanged", async () => {
    mockExecuteWritePipeline.mockResolvedValue({
      status: "created",
      memory: {
        id: "mem-human",
        title: "Human Memory",
        created_at: "2026-03-11T00:00:00Z",
      },
      dedup_outcome: "new",
    });

    const result = await createMemory({
      title: "Human Memory",
      content: "Content by human",
      summary: "Human summary",
      memory_type: "guidance",
      source_type: "instruction",
      created_by: "human",
      language: "en",
    });

    expect(result.status).toBe("created");

    const pipelineInput = mockExecuteWritePipeline.mock.calls[0]?.[0];
    expect(pipelineInput).toMatchObject({
      created_by: "human",
    });
  });

  it("propagates write pipeline failure and does not log activity", async () => {
    mockExecuteWritePipeline.mockRejectedValue(
      new Error("Storage write failed"),
    );

    await expect(
      createMemory({
        title: "New Memory",
        content: "Content here",
        summary: "Summary here",
        memory_type: "guidance",
        source_type: "discovery",
        created_by: "ai",
        language: "en",
      }),
    ).rejects.toThrow("Storage write failed");

    expect(mockLogActivity).not.toHaveBeenCalled();
  });

  it("throws validation error for invalid input", async () => {
    const err = await createMemory({
      title: "",
      content: "Content",
      summary: "Summary",
      memory_type: "guidance",
      source_type: "discovery",
      created_by: "ai",
      language: "en",
    }).catch((e: unknown) => e);
    expect(err).toBeDefined();
    expect((err as Error).name).toBe("ZodError");

    expect(mockExecuteWritePipeline).not.toHaveBeenCalled();
  });
});
