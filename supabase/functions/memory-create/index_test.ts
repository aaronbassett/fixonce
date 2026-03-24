/**
 * Tests for memory-create edge function.
 *
 * T058: Verify Zod schema validation for memory-create payloads.
 *
 * Strategy: The schema is defined in index.ts but not exported, so we
 * reconstruct it here using the same Zod definitions. This tests the
 * schema logic in isolation (pure unit tests — no network, no Supabase).
 *
 * We also test the error response structure produced by _shared/errors.ts
 * and the malformed-JSON handling path (EC-10).
 */
import { assertEquals, assertStringIncludes } from "@std/assert";
import { z } from "zod";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { errorResponse } from "../_shared/errors.ts";

// ---------------------------------------------------------------------------
// Schema mirror — identical to the one in memory-create/index.ts
// Keep in sync if the production schema changes.
// ---------------------------------------------------------------------------
const MEMORY_TYPES = [
  "gotcha",
  "best_practice",
  "correction",
  "anti_pattern",
  "discovery",
] as const;

const SOURCE_TYPES = [
  "correction",
  "observation",
  "pr_feedback",
  "manual",
  "harvested",
] as const;

const memoryCreateSchema = z.object({
  title: z.string().min(1).max(500),
  content: z.string().min(1),
  summary: z.string().min(1).max(2000),
  memory_type: z.enum(MEMORY_TYPES),
  source_type: z.enum(SOURCE_TYPES),

  language: z.string().max(100).optional(),
  embedding: z.array(z.number()).length(1024).optional(),

  compact_pragma: z.string().max(200).optional(),
  compact_compiler: z.string().max(200).optional(),
  midnight_js: z.string().max(200).optional(),
  indexer_version: z.string().max(200).optional(),
  node_version: z.string().max(200).optional(),

  source_url: z.string().url().optional(),
  repo_url: z.string().url().optional(),
  task_summary: z.string().max(1000).optional(),
  session_id: z.string().max(200).optional(),
});

// ---------------------------------------------------------------------------
// Helper: minimal valid payload
// ---------------------------------------------------------------------------
const VALID_PAYLOAD = {
  title: "Use early returns to reduce nesting",
  content: "Deeply nested if-blocks hurt readability. Prefer guard clauses.",
  summary: "Prefer early returns over deep nesting.",
  memory_type: "best_practice",
  source_type: "manual",
} as const;

// ===========================================================================
// T058-A: Valid payloads pass schema validation
// ===========================================================================

Deno.test("memory-create schema: minimal valid payload passes", () => {
  const result = memoryCreateSchema.safeParse(VALID_PAYLOAD);
  assertEquals(result.success, true);
});

Deno.test("memory-create schema: all optional fields accepted when provided", () => {
  const payload = {
    ...VALID_PAYLOAD,
    language: "typescript",
    compact_pragma: "1.2.3",
    compact_compiler: "0.9.0",
    midnight_js: "4.5.0",
    indexer_version: "2.0.1",
    node_version: "20.11.0",
    source_url: "https://github.com/example/repo",
    repo_url: "https://github.com/example/repo",
    task_summary: "Refactored module to use guard clauses.",
    session_id: "sess_abc123",
  };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, true);
});

Deno.test("memory-create schema: all valid memory_type values accepted", () => {
  for (const mt of MEMORY_TYPES) {
    const payload = { ...VALID_PAYLOAD, memory_type: mt };
    const result = memoryCreateSchema.safeParse(payload);
    assertEquals(result.success, true, `Expected ${mt} to be valid`);
  }
});

Deno.test("memory-create schema: all valid source_type values accepted", () => {
  for (const st of SOURCE_TYPES) {
    const payload = { ...VALID_PAYLOAD, source_type: st };
    const result = memoryCreateSchema.safeParse(payload);
    assertEquals(result.success, true, `Expected ${st} to be valid`);
  }
});

// ===========================================================================
// T058-B: Missing required fields → validation failure
// ===========================================================================

Deno.test("memory-create schema: missing 'title' fails validation", () => {
  const { title: _title, ...rest } = VALID_PAYLOAD;
  const result = memoryCreateSchema.safeParse(rest);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("title"), true);
  }
});

Deno.test("memory-create schema: missing 'content' fails validation", () => {
  const { content: _content, ...rest } = VALID_PAYLOAD;
  const result = memoryCreateSchema.safeParse(rest);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("content"), true);
  }
});

Deno.test("memory-create schema: missing 'summary' fails validation", () => {
  const { summary: _summary, ...rest } = VALID_PAYLOAD;
  const result = memoryCreateSchema.safeParse(rest);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("summary"), true);
  }
});

Deno.test("memory-create schema: missing 'memory_type' fails validation", () => {
  const { memory_type: _memory_type, ...rest } = VALID_PAYLOAD;
  const result = memoryCreateSchema.safeParse(rest);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("memory_type"), true);
  }
});

Deno.test("memory-create schema: missing 'source_type' fails validation", () => {
  const { source_type: _source_type, ...rest } = VALID_PAYLOAD;
  const result = memoryCreateSchema.safeParse(rest);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("source_type"), true);
  }
});

Deno.test("memory-create schema: completely empty body fails validation", () => {
  const result = memoryCreateSchema.safeParse({});
  assertEquals(result.success, false);
});

// ===========================================================================
// T058-C: Invalid enum values → validation failure
// ===========================================================================

Deno.test("memory-create schema: invalid memory_type value fails", () => {
  const payload = { ...VALID_PAYLOAD, memory_type: "unknown_type" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("memory_type"), true);
  }
});

Deno.test("memory-create schema: invalid source_type value fails", () => {
  const payload = { ...VALID_PAYLOAD, source_type: "invalid_source" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
  if (!result.success) {
    const paths = result.error.issues.map((i) => i.path[0]);
    assertEquals(paths.includes("source_type"), true);
  }
});

Deno.test("memory-create schema: empty string for memory_type fails", () => {
  const payload = { ...VALID_PAYLOAD, memory_type: "" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

// ===========================================================================
// T058-D: Field length constraints
// ===========================================================================

Deno.test("memory-create schema: title exceeding 500 chars fails", () => {
  const payload = { ...VALID_PAYLOAD, title: "x".repeat(501) };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

Deno.test("memory-create schema: empty title fails (min 1)", () => {
  const payload = { ...VALID_PAYLOAD, title: "" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

Deno.test("memory-create schema: summary exceeding 2000 chars fails", () => {
  const payload = { ...VALID_PAYLOAD, summary: "x".repeat(2001) };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

Deno.test("memory-create schema: invalid source_url fails", () => {
  const payload = { ...VALID_PAYLOAD, source_url: "not-a-url" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

Deno.test("memory-create schema: invalid repo_url fails", () => {
  const payload = { ...VALID_PAYLOAD, repo_url: "just-a-string" };
  const result = memoryCreateSchema.safeParse(payload);
  assertEquals(result.success, false);
});

// ===========================================================================
// T058-E: validateBody integration — throws ValidationError correctly
// ===========================================================================

Deno.test("validateBody with memoryCreateSchema: missing required field throws ValidationError", () => {
  const { title: _title, ...rest } = VALID_PAYLOAD;
  try {
    validateBody(memoryCreateSchema, rest);
    throw new Error("Expected ValidationError");
  } catch (err) {
    if (!(err instanceof ValidationError)) throw err;
    assertEquals(err instanceof ValidationError, true);
    assertStringIncludes(err.message, "title");
  }
});

// ===========================================================================
// T058-F: EC-10 — malformed JSON returns 400 with INVALID_JSON error type
//         We test errorResponse() directly since we cannot call Deno.serve.
// ===========================================================================

Deno.test("EC-10: errorResponse produces correct structured error body", async () => {
  const resp = errorResponse(
    400,
    "INVALID_JSON",
    "Request body is not valid JSON.",
    "Ensure the request body is well-formed JSON.",
  );

  assertEquals(resp.status, 400);
  assertEquals(resp.headers.get("Content-Type"), "application/json");

  const body = await resp.json();
  assertEquals(typeof body.error, "object");
  assertEquals(body.error.type, "INVALID_JSON");
  assertEquals(body.error.message, "Request body is not valid JSON.");
  assertEquals(typeof body.error.action, "string");
});

Deno.test("EC-10: errorResponse 400 VALIDATION_ERROR has correct structure", async () => {
  const resp = errorResponse(
    400,
    "VALIDATION_ERROR",
    "title: String must contain at least 1 character(s)",
    "Check the request body against the required schema.",
  );

  assertEquals(resp.status, 400);
  const body = await resp.json();
  assertEquals(body.error.type, "VALIDATION_ERROR");
  assertStringIncludes(body.error.message, "title");
});

Deno.test("EC-10: errorResponse 401 UNAUTHORIZED has correct structure", async () => {
  const resp = errorResponse(
    401,
    "UNAUTHORIZED",
    "Missing or malformed Authorization header",
    "Provide a valid Bearer token in the Authorization header.",
  );

  assertEquals(resp.status, 401);
  const body = await resp.json();
  assertEquals(body.error.type, "UNAUTHORIZED");
});
