import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  clearExpiredKeys,
  generateCacheKey,
  lookupCacheKey,
} from "./cache.js";

const THIRTY_MINUTES_MS = 30 * 60 * 1000;

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

  it("lookupCacheKey returns null for an unknown key", () => {
    expect(lookupCacheKey("ck_nonexistent")).toBeNull();
  });

  it("lookupCacheKey returns null for an expired key (31 min)", () => {
    const key = generateCacheKey("mem-exp");
    vi.advanceTimersByTime(31 * 60 * 1000);
    expect(lookupCacheKey(key)).toBeNull();
  });

  it("lookupCacheKey succeeds just before TTL expires (29m59s)", () => {
    const key = generateCacheKey("mem-edge");
    vi.advanceTimersByTime(THIRTY_MINUTES_MS - 1000);
    expect(lookupCacheKey(key)).toBe("mem-edge");
  });

  it("clearExpiredKeys removes expired entries and keeps valid ones", () => {
    const expiredKey = generateCacheKey("mem-old");
    vi.advanceTimersByTime(THIRTY_MINUTES_MS + 1);
    const freshKey = generateCacheKey("mem-new");

    clearExpiredKeys();

    expect(lookupCacheKey(expiredKey)).toBeNull();
    expect(lookupCacheKey(freshKey)).toBe("mem-new");
  });
});
