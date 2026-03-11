import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const THIRTY_MINUTES_MS = 30 * 60 * 1000;

async function loadCache() {
  vi.resetModules();
  return import("./cache.js");
}

describe("cache", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("generateCacheKey returns a ck_ prefixed string", async () => {
    const { generateCacheKey } = await loadCache();
    const key = generateCacheKey("mem-1");
    expect(key).toMatch(/^ck_/);
  });

  it("generateCacheKey produces unique keys for the same memoryId", async () => {
    const { generateCacheKey } = await loadCache();
    const key1 = generateCacheKey("mem-1");
    const key2 = generateCacheKey("mem-1");
    expect(key1).not.toBe(key2);
  });

  it("lookupCacheKey returns memoryId for a valid key", async () => {
    const { generateCacheKey, lookupCacheKey } = await loadCache();
    const key = generateCacheKey("mem-42");
    expect(lookupCacheKey(key)).toBe("mem-42");
  });

  it("lookupCacheKey returns null for an unknown key", async () => {
    const { lookupCacheKey } = await loadCache();
    expect(lookupCacheKey("ck_nonexistent")).toBeNull();
  });

  it("lookupCacheKey returns null for an expired key (31 min)", async () => {
    const { generateCacheKey, lookupCacheKey } = await loadCache();
    const key = generateCacheKey("mem-exp");
    vi.advanceTimersByTime(31 * 60 * 1000);
    expect(lookupCacheKey(key)).toBeNull();
  });

  it("lookupCacheKey succeeds just before TTL expires (29m59s)", async () => {
    const { generateCacheKey, lookupCacheKey } = await loadCache();
    const key = generateCacheKey("mem-edge");
    vi.advanceTimersByTime(THIRTY_MINUTES_MS - 1000);
    expect(lookupCacheKey(key)).toBe("mem-edge");
  });

  it("clearExpiredKeys removes expired entries and keeps valid ones", async () => {
    const { generateCacheKey, lookupCacheKey, clearExpiredKeys } =
      await loadCache();
    const expiredKey = generateCacheKey("mem-old");
    vi.advanceTimersByTime(THIRTY_MINUTES_MS + 1);
    const freshKey = generateCacheKey("mem-new");

    clearExpiredKeys();

    expect(lookupCacheKey(expiredKey)).toBeNull();
    expect(lookupCacheKey(freshKey)).toBe("mem-new");
  });
});
