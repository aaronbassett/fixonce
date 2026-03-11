import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Memory, CreateMemoryInput } from "@fixonce/shared";

vi.mock("@fixonce/storage", () => ({
  createMemory: vi.fn(),
  updateMemory: vi.fn(),
  getMemoryById: vi.fn(),
  generateEmbedding: vi.fn().mockResolvedValue([0.1, 0.2, 0.3]),
}));

vi.mock("./quality-gate.js", () => ({
  evaluateQuality: vi
    .fn()
    .mockResolvedValue({ decision: "accept", reason: "" }),
}));

vi.mock("./duplicate-detection.js", () => ({
  detectDuplicates: vi.fn(),
}));

import { executeWritePipeline } from "./index.js";
import { createMemory as storeMemory, updateMemory } from "@fixonce/storage";
import { detectDuplicates } from "./duplicate-detection.js";

const mockedStoreMemory = vi.mocked(storeMemory);
const mockedUpdateMemory = vi.mocked(updateMemory);
const mockedDetectDuplicates = vi.mocked(detectDuplicates);

function makeMemory(overrides?: Partial<Memory>): Memory {
  return {
    id: "mem-merged-001",
    title: "Merged Title",
    content: "Merged content",
    summary: "Merged summary",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "ai",
    source_url: null,
    tags: ["ts"],
    language: "en",
    version_predicates: null,
    project_name: null,
    project_repo_url: null,
    project_workspace_path: null,
    confidence: 0.5,
    surfaced_count: 0,
    last_surfaced_at: null,
    enabled: true,
    created_at: "2026-03-11T00:00:00Z",
    updated_at: "2026-03-11T00:00:00Z",
    embedding: null,
    ...overrides,
  };
}

function makeInput(overrides?: Partial<CreateMemoryInput>): CreateMemoryInput {
  return {
    title: "New Memory",
    content: "New content",
    summary: "New summary",
    memory_type: "guidance",
    source_type: "discovery",
    created_by: "ai",
    language: "en",
    tags: ["ts"],
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("executeWritePipeline – merge case", () => {
  it("creates merged memory before disabling the original", async () => {
    const mergedMemory = makeMemory({
      id: "mem-merged-001",
      title: "Merged Title",
    });
    const disabledOriginal = makeMemory({
      id: "mem-original-001",
      enabled: false,
    });

    mockedDetectDuplicates.mockResolvedValue({
      outcome: "merge",
      reason: "memories overlap",
      target_memory_id: "mem-original-001",
      merged_title: "Merged Title",
      merged_content: "Merged content",
      merged_summary: "Merged summary",
    });
    const callOrder: string[] = [];
    mockedStoreMemory.mockImplementation(() => {
      callOrder.push("storeMemory");
      return Promise.resolve(mergedMemory);
    });
    mockedUpdateMemory.mockImplementation(() => {
      callOrder.push("updateMemory");
      return Promise.resolve(disabledOriginal);
    });

    const result = await executeWritePipeline(makeInput());

    expect(result.status).toBe("merged");
    expect(result.memory).toEqual({
      id: "mem-merged-001",
      title: "Merged Title",
      created_at: "2026-03-11T00:00:00Z",
    });
    expect(result.dedup_outcome).toBe("merge");
    expect(result.affected_memory_ids).toEqual(["mem-original-001"]);

    // storeMemory (create) must be called before updateMemory (disable)
    expect(callOrder.indexOf("storeMemory")).toBeLessThan(
      callOrder.indexOf("updateMemory"),
    );

    // updateMemory was called with enabled: false
    expect(mockedUpdateMemory).toHaveBeenCalledWith("mem-original-001", {
      enabled: false,
    });
  });

  it("does not disable the original if creating the merged memory fails", async () => {
    mockedDetectDuplicates.mockResolvedValue({
      outcome: "merge",
      reason: "memories overlap",
      target_memory_id: "mem-original-001",
      merged_title: "Merged Title",
      merged_content: "Merged content",
      merged_summary: "Merged summary",
    });
    mockedStoreMemory.mockRejectedValue(new Error("Storage write failed"));

    await expect(executeWritePipeline(makeInput())).rejects.toThrow(
      "Storage write failed",
    );

    // The original memory must NOT have been disabled
    expect(mockedUpdateMemory).not.toHaveBeenCalled();
  });
});
