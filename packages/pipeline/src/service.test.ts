import { describe, it, expect, beforeEach, vi } from "vitest";
import type { Memory } from "@fixonce/shared";

vi.mock("@fixonce/storage", () => ({
  getMemoryById: vi.fn(),
  updateMemory: vi.fn(),
  generateEmbedding: vi.fn(),
}));

vi.mock("@fixonce/activity", () => ({
  logActivity: vi.fn().mockResolvedValue(undefined),
}));

import {
  getMemoryById,
  updateMemory as storageUpdateMemory,
  generateEmbedding,
} from "@fixonce/storage";
import { updateMemory } from "./service.js";

const mockGetMemoryById = vi.mocked(getMemoryById);
const mockStorageUpdateMemory = vi.mocked(storageUpdateMemory);
const mockGenerateEmbedding = vi.mocked(generateEmbedding);

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
});
