/**
 * Tests for memory-search edge function.
 *
 * T059: Verify:
 *   - Schema validation: query_text required for fts/hybrid modes
 *   - tsvector query sanitization removes special characters (EC-08)
 *   - Empty results return empty array, not an error (EC-09)
 */
import {
  assertEquals,
  assertStringIncludes,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";

// ---------------------------------------------------------------------------
// Schema mirror — identical to the one in memory-search/index.ts
// Keep in sync if the production schema changes.
// ---------------------------------------------------------------------------
const SEARCH_TYPES = ["hybrid", "fts", "vector"] as const;
const MEMORY_TYPES = [
  "gotcha",
  "best_practice",
  "correction",
  "anti_pattern",
  "discovery",
] as const;

const versionFiltersSchema = z
  .object({
    compact_pragma: z.string().optional(),
    compact_compiler: z.string().optional(),
    midnight_js: z.string().optional(),
    indexer_version: z.string().optional(),
    node_version: z.string().optional(),
  })
  .optional();

const memorySearchSchema = z
  .object({
    query_text: z.string().max(2000).optional(),
    query_embedding: z.array(z.number()).length(1024).optional(),
    search_type: z.enum(SEARCH_TYPES).default("hybrid"),
    limit: z.number().int().min(1).max(100).default(20),
    version_filters: versionFiltersSchema,
    memory_type: z.enum(MEMORY_TYPES).optional(),
    language: z.string().max(100).optional(),
  })
  .refine(
    (v) =>
      v.search_type === "vector"
        ? v.query_embedding !== undefined
        : v.search_type === "fts"
        ? v.query_text !== undefined
        : v.query_text !== undefined || v.query_embedding !== undefined,
    {
      message: "Provide at least one of query_text or query_embedding. " +
        "search_type='fts' requires query_text; " +
        "search_type='vector' requires query_embedding.",
      path: ["query_text"],
    },
  );

// ---------------------------------------------------------------------------
// sanitizeQueryText — same implementation as in memory-search/index.ts (EC-08)
// ---------------------------------------------------------------------------
function sanitizeQueryText(text: string): string {
  return text.replace(/[&|!():*'\\]/g, " ").replace(/\s+/g, " ").trim();
}

// ===========================================================================
// T059-A: Schema validation — query_text required for fts mode
// ===========================================================================

Deno.test("memory-search schema: fts without query_text fails", () => {
  const result = memorySearchSchema.safeParse({
    search_type: "fts",
    // query_text omitted
  });
  assertEquals(result.success, false);
  if (!result.success) {
    const messages = result.error.issues.map((i) => i.message).join(" ");
    assertStringIncludes(messages, "query_text");
  }
});

Deno.test("memory-search schema: fts with query_text passes", () => {
  const result = memorySearchSchema.safeParse({
    search_type: "fts",
    query_text: "early return pattern",
  });
  assertEquals(result.success, true);
});

// ===========================================================================
// T059-B: Schema validation — query_embedding required for vector mode
// ===========================================================================

Deno.test("memory-search schema: vector without query_embedding fails", () => {
  const result = memorySearchSchema.safeParse({
    search_type: "vector",
    // query_embedding omitted
  });
  assertEquals(result.success, false);
  if (!result.success) {
    const messages = result.error.issues.map((i) => i.message).join(" ");
    assertStringIncludes(messages, "query_embedding");
  }
});

Deno.test("memory-search schema: vector with query_embedding (1024 dims) passes", () => {
  const embedding = Array.from({ length: 1024 }, () => Math.random());
  const result = memorySearchSchema.safeParse({
    search_type: "vector",
    query_embedding: embedding,
  });
  assertEquals(result.success, true);
});

// ===========================================================================
// T059-C: Schema validation — hybrid mode
// ===========================================================================

Deno.test("memory-search schema: hybrid without query_text or embedding fails", () => {
  const result = memorySearchSchema.safeParse({
    search_type: "hybrid",
    // neither query_text nor query_embedding
  });
  assertEquals(result.success, false);
});

Deno.test("memory-search schema: hybrid with query_text only passes", () => {
  const result = memorySearchSchema.safeParse({
    search_type: "hybrid",
    query_text: "avoid deep nesting",
  });
  assertEquals(result.success, true);
});

Deno.test("memory-search schema: hybrid with embedding only passes", () => {
  const embedding = Array.from({ length: 1024 }, () => 0.1);
  const result = memorySearchSchema.safeParse({
    search_type: "hybrid",
    query_embedding: embedding,
  });
  assertEquals(result.success, true);
});

Deno.test("memory-search schema: default search_type is hybrid", () => {
  const embedding = Array.from({ length: 1024 }, () => 0.0);
  const result = memorySearchSchema.safeParse({
    query_embedding: embedding,
  });
  assertEquals(result.success, true);
  if (result.success) {
    assertEquals(result.data.search_type, "hybrid");
  }
});

// ===========================================================================
// T059-D: Schema defaults and constraints
// ===========================================================================

Deno.test("memory-search schema: default limit is 20", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
  });
  assertEquals(result.success, true);
  if (result.success) {
    assertEquals(result.data.limit, 20);
  }
});

Deno.test("memory-search schema: limit 0 fails (min is 1)", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
    limit: 0,
  });
  assertEquals(result.success, false);
});

Deno.test("memory-search schema: limit 101 fails (max is 100)", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
    limit: 101,
  });
  assertEquals(result.success, false);
});

Deno.test("memory-search schema: limit 100 passes", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
    limit: 100,
  });
  assertEquals(result.success, true);
});

Deno.test("memory-search schema: invalid memory_type filter fails", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
    memory_type: "not_a_valid_type",
  });
  assertEquals(result.success, false);
});

Deno.test("memory-search schema: valid memory_type filter passes", () => {
  for (const mt of MEMORY_TYPES) {
    const result = memorySearchSchema.safeParse({
      query_text: "some query",
      memory_type: mt,
    });
    assertEquals(result.success, true, `Expected memory_type '${mt}' to pass`);
  }
});

Deno.test("memory-search schema: invalid search_type fails", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "some query",
    search_type: "semantic",
  });
  assertEquals(result.success, false);
});

Deno.test("memory-search schema: query_text exceeding 2000 chars fails", () => {
  const result = memorySearchSchema.safeParse({
    query_text: "x".repeat(2001),
    search_type: "fts",
  });
  assertEquals(result.success, false);
});

// ===========================================================================
// T059-E: EC-08 — tsvector query sanitization removes special characters
// ===========================================================================

Deno.test("EC-08 sanitizeQueryText: removes tsquery operator &", () => {
  const result = sanitizeQueryText("foo & bar");
  assertEquals(result.includes("&"), false);
  assertStringIncludes(result, "foo");
  assertStringIncludes(result, "bar");
});

Deno.test("EC-08 sanitizeQueryText: removes tsquery operator |", () => {
  const result = sanitizeQueryText("foo | bar");
  assertEquals(result.includes("|"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes tsquery operator !", () => {
  const result = sanitizeQueryText("!important");
  assertEquals(result.includes("!"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes parentheses", () => {
  const result = sanitizeQueryText("(foo OR bar)");
  assertEquals(result.includes("("), false);
  assertEquals(result.includes(")"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes colon operator", () => {
  const result = sanitizeQueryText("word:prefix");
  assertEquals(result.includes(":"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes asterisk wildcard", () => {
  const result = sanitizeQueryText("foo*");
  assertEquals(result.includes("*"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes single quotes", () => {
  const result = sanitizeQueryText("it's a test");
  assertEquals(result.includes("'"), false);
});

Deno.test("EC-08 sanitizeQueryText: removes backslash", () => {
  const result = sanitizeQueryText("foo\\bar");
  assertEquals(result.includes("\\"), false);
});

Deno.test("EC-08 sanitizeQueryText: collapses multiple spaces to single", () => {
  const result = sanitizeQueryText("foo   &   bar");
  assertEquals(result.includes("  "), false); // no double spaces
  assertEquals(result, "foo   bar".replace(/\s+/g, " ").trim());
});

Deno.test("EC-08 sanitizeQueryText: preserves normal words", () => {
  const result = sanitizeQueryText("early return guard clause");
  assertEquals(result, "early return guard clause");
});

Deno.test("EC-08 sanitizeQueryText: trims leading and trailing whitespace", () => {
  const result = sanitizeQueryText("  hello world  ");
  assertEquals(result, "hello world");
});

Deno.test("EC-08 sanitizeQueryText: injection attempt with multiple operators", () => {
  const malicious = "foo & bar | baz ! (qux) : prefix*";
  const result = sanitizeQueryText(malicious);
  // None of the tsquery special characters should survive
  for (const ch of ["&", "|", "!", "(", ")", ":", "*"]) {
    assertEquals(
      result.includes(ch),
      false,
      `Character '${ch}' should be removed`,
    );
  }
});

Deno.test("EC-08 sanitizeQueryText: empty string remains empty", () => {
  assertEquals(sanitizeQueryText(""), "");
});

// ===========================================================================
// T059-F: EC-09 — empty results return empty array, not an error
//         We test the result-mapping logic directly.
// ===========================================================================

Deno.test("EC-09: null RPC data maps to empty array", () => {
  const data = null;
  const rows = Array.isArray(data) ? data : [];
  assertEquals(rows, []);
  assertEquals(rows.length, 0);
});

Deno.test("EC-09: undefined RPC data maps to empty array", () => {
  const data = undefined;
  const rows = Array.isArray(data) ? data : [];
  assertEquals(rows, []);
});

Deno.test("EC-09: empty array RPC data stays as empty array", () => {
  const data: unknown[] = [];
  const rows = Array.isArray(data) ? data : [];
  assertEquals(rows, []);
  assertEquals(rows.length, 0);
});

Deno.test("EC-09: non-empty array RPC data is preserved", () => {
  const data = [{ memory_id: "abc", title: "Test" }];
  const rows = Array.isArray(data) ? data : [];
  assertEquals(rows.length, 1);
  assertEquals((rows[0] as { memory_id: string }).memory_id, "abc");
});

// ===========================================================================
// T059-G: validateBody integration — ValidationError thrown correctly
// ===========================================================================

Deno.test("validateBody with memorySearchSchema: fts without query_text throws ValidationError", () => {
  try {
    validateBody(memorySearchSchema, { search_type: "fts" });
    throw new Error("Expected ValidationError");
  } catch (err) {
    if (!(err instanceof ValidationError)) throw err;
    assertEquals(err instanceof ValidationError, true);
    assertStringIncludes(err.message, "query_text");
  }
});

Deno.test("validateBody with memorySearchSchema: invalid limit throws ValidationError", () => {
  try {
    validateBody(memorySearchSchema, { query_text: "test", limit: -5 });
    throw new Error("Expected ValidationError");
  } catch (err) {
    if (!(err instanceof ValidationError)) throw err;
    assertStringIncludes(err.message, "limit");
  }
});
