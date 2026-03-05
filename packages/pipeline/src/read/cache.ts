import { randomBytes } from "node:crypto";

const DEFAULT_TTL_MS = 30 * 60 * 1000; // 30 minutes

interface CacheEntry {
  memoryId: string;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();

/**
 * Generate a unique cache key for a memory ID using cryptographic randomness.
 * Returns a string in the format "ck_<12chars>".
 */
export function generateCacheKey(memoryId: string): string {
  const token = randomBytes(9).toString("base64url");
  const key = `ck_${token}`;

  cache.set(key, {
    memoryId,
    expiresAt: Date.now() + DEFAULT_TTL_MS,
  });

  return key;
}

/**
 * Look up a memory ID from a cache key. Returns null if the key
 * is not found or has expired.
 */
export function lookupCacheKey(cacheKey: string): string | null {
  const entry = cache.get(cacheKey);
  if (!entry) return null;

  if (Date.now() > entry.expiresAt) {
    cache.delete(cacheKey);
    return null;
  }

  return entry.memoryId;
}

/**
 * Remove all expired entries from the cache.
 */
export function clearExpiredKeys(): void {
  const now = Date.now();
  for (const [key, entry] of cache) {
    if (now > entry.expiresAt) {
      cache.delete(key);
    }
  }
}
