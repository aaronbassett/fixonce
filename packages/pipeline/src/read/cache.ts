const DEFAULT_TTL_MS = 30 * 60 * 1000; // 30 minutes

interface CacheEntry {
  memoryId: string;
  expiresAt: number;
}

const cache = new Map<string, CacheEntry>();

/**
 * Generate a deterministic-but-unique cache key for a memory ID.
 * Returns a string in the format "ck_<6chars>".
 */
export function generateCacheKey(memoryId: string): string {
  const source = `${memoryId}:${Date.now()}`;
  let hash = 0;
  for (let i = 0; i < source.length; i++) {
    const char = source.charCodeAt(i);
    hash = ((hash << 5) - hash + char) | 0;
  }
  const hashStr = Math.abs(hash).toString(36).padStart(6, "0").slice(0, 6);
  const key = `ck_${hashStr}`;

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
