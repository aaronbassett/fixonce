import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { generateCacheKey, lookupCacheKey, clearExpiredKeys } from "./cache.js";

describe("cache", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("generateCacheKey returns a ck_ prefixed string", () => {
    const key = generateCacheKey("mem-1");
    expect(key).toMatch(/^ck_/);
  });

  it("generateCacheKey produces unique keys for the same memoryId", () => {
    const key1 = generateCacheKey("mem-1");
    const key2 = generateCacheKey("mem-1");
    expect(key1).not.toBe(key2);
  });

  it("lookupCacheKey returns memoryId for a valid key", () => {
    const key = generateCacheKey("mem-42");
    expect(lookupCacheKey(key)).toBe("mem-42");
  });

  it("lookupCacheKey returns null for unknown key", () => {
    expect(lookupCacheKey("ck_nonexistent")).toBeNull();
  });

  it("lookupCacheKey returns null for expired key", () => {
    const key = generateCacheKey("mem-expire");
    vi.advanceTimersByTime(31 * 60 * 1000);
    expect(lookupCacheKey(key)).toBeNull();
  });

  it("lookupCacheKey succeeds just before TTL expires", () => {
    const key = generateCacheKey("mem-boundary");
    vi.advanceTimersByTime(29 * 60 * 1000 + 59 * 1000);
    expect(lookupCacheKey(key)).toBe("mem-boundary");
  });

  it("clearExpiredKeys removes expired entries and keeps valid ones", () => {
    const oldKey = generateCacheKey("mem-old");
    vi.advanceTimersByTime(15 * 60 * 1000);
    const newKey = generateCacheKey("mem-new");
    vi.advanceTimersByTime(16 * 60 * 1000);

    clearExpiredKeys();

    expect(lookupCacheKey(oldKey)).toBeNull();
    expect(lookupCacheKey(newKey)).toBe("mem-new");
  });
});
