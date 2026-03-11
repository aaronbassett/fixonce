import { describe, it, expect, vi, beforeEach } from "vitest";

const mockRpc = vi.fn();
const mockFrom = vi.fn();

vi.mock("./client.js", () => ({
  createSupabaseClient: () => ({
    rpc: (...args: unknown[]) => mockRpc(...args),
    from: (...args: unknown[]) => mockFrom(...args),
  }),
}));

import { incrementSurfacedCount } from "./memories.js";

beforeEach(() => {
  vi.clearAllMocks();
});

describe("incrementSurfacedCount", () => {
  it("returns immediately for an empty array without calling supabase", async () => {
    await incrementSurfacedCount([]);

    expect(mockRpc).not.toHaveBeenCalled();
    expect(mockFrom).not.toHaveBeenCalled();
  });

  it("calls batch_increment_surfaced_count RPC with correct parameter", async () => {
    mockRpc.mockResolvedValue({ error: null });

    const ids = ["id-1", "id-2", "id-3"];
    await incrementSurfacedCount(ids);

    expect(mockRpc).toHaveBeenCalledOnce();
    expect(mockRpc).toHaveBeenCalledWith("batch_increment_surfaced_count", {
      memory_ids: ids,
    });
  });

  it("calls RPC for a single ID", async () => {
    mockRpc.mockResolvedValue({ error: null });

    await incrementSurfacedCount(["id-1"]);

    expect(mockRpc).toHaveBeenCalledOnce();
    expect(mockRpc).toHaveBeenCalledWith("batch_increment_surfaced_count", {
      memory_ids: ["id-1"],
    });
  });

  it("throws when RPC fails", async () => {
    const rpcError = { message: "function not found", code: "42883" };
    mockRpc.mockResolvedValue({ error: rpcError });

    await expect(incrementSurfacedCount(["id-1"])).rejects.toEqual(rpcError);
    expect(mockFrom).not.toHaveBeenCalled();
  });
});
