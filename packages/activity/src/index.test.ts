import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("@fixonce/storage", () => ({
  appendActivity: vi.fn(),
}));

import { appendActivity } from "@fixonce/storage";
import { logActivity } from "./index.js";

const mockAppendActivity = vi.mocked(appendActivity);

describe("logActivity", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls appendActivity with correct operation, details, and memory_id", async () => {
    mockAppendActivity.mockResolvedValue({
      id: "activity-1",
      operation: "create",
      memory_id: "mem-123",
      details: { status: "created" },
      created_at: "2026-03-11T00:00:00Z",
    });

    await logActivity("create", { status: "created" }, "mem-123");

    expect(mockAppendActivity).toHaveBeenCalledOnce();
    expect(mockAppendActivity).toHaveBeenCalledWith({
      operation: "create",
      memory_id: "mem-123",
      details: { status: "created" },
    });
  });

  it("passes null for memory_id when not provided", async () => {
    mockAppendActivity.mockResolvedValue({
      id: "activity-2",
      operation: "query",
      memory_id: null,
      details: { query: "test" },
      created_at: "2026-03-11T00:00:00Z",
    });

    await logActivity("query", { query: "test" });

    expect(mockAppendActivity).toHaveBeenCalledWith({
      operation: "query",
      memory_id: null,
      details: { query: "test" },
    });
  });

  it("swallows errors without disrupting the caller", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    mockAppendActivity.mockRejectedValue(new Error("DB connection lost"));

    await expect(
      logActivity("update", { memory_id: "mem-1" }, "mem-1"),
    ).resolves.toBeUndefined();

    expect(errorSpy).toHaveBeenCalledWith(
      "Failed to log activity:",
      expect.any(Error),
    );
    errorSpy.mockRestore();
  });

  it("does not throw when appendActivity throws synchronously", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    mockAppendActivity.mockImplementation(() => {
      throw new Error("Synchronous failure");
    });

    await expect(
      logActivity("create", { status: "test" }),
    ).resolves.toBeUndefined();

    expect(errorSpy).toHaveBeenCalled();
    errorSpy.mockRestore();
  });
});
