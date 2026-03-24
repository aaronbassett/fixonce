/**
 * Tests for _shared/validate.ts — validateBody and ValidationError.
 *
 * These are pure unit tests; no network or Supabase connection needed.
 */
import {
  assertEquals,
  assertInstanceOf,
  assertThrows,
} from "@std/assert";
import { z } from "zod";
import { validateBody, ValidationError } from "./validate.ts";

// ---------------------------------------------------------------------------
// Helper schema used throughout the tests
// ---------------------------------------------------------------------------
const testSchema = z.object({
  name: z.string().min(1),
  age: z.number().int().min(0),
  role: z.enum(["admin", "user"]),
});

// ---------------------------------------------------------------------------
// Valid payload passes without throwing
// ---------------------------------------------------------------------------
Deno.test("validateBody: valid payload returns typed data", () => {
  const result = validateBody(testSchema, {
    name: "Alice",
    age: 30,
    role: "admin",
  });
  assertEquals(result.name, "Alice");
  assertEquals(result.age, 30);
  assertEquals(result.role, "admin");
});

// ---------------------------------------------------------------------------
// Missing required field → ValidationError
// ---------------------------------------------------------------------------
Deno.test("validateBody: missing required field throws ValidationError", () => {
  assertThrows(
    () => validateBody(testSchema, { name: "Alice" }), // missing age and role
    ValidationError,
  );
});

// ---------------------------------------------------------------------------
// Wrong type → ValidationError
// ---------------------------------------------------------------------------
Deno.test("validateBody: wrong field type throws ValidationError", () => {
  assertThrows(
    () =>
      validateBody(testSchema, { name: "Alice", age: "thirty", role: "admin" }),
    ValidationError,
  );
});

// ---------------------------------------------------------------------------
// Invalid enum value → ValidationError
// ---------------------------------------------------------------------------
Deno.test("validateBody: invalid enum value throws ValidationError", () => {
  assertThrows(
    () =>
      validateBody(testSchema, { name: "Alice", age: 30, role: "superuser" }),
    ValidationError,
  );
});

// ---------------------------------------------------------------------------
// ValidationError carries the Zod issues array
// ---------------------------------------------------------------------------
Deno.test("ValidationError: issues array is populated", () => {
  try {
    validateBody(testSchema, { name: "", age: -1, role: "unknown" });
    throw new Error("Expected ValidationError to be thrown");
  } catch (err) {
    assertInstanceOf(err, ValidationError);
    // Should have at least one issue per failing field
    assertEquals((err as ValidationError).issues.length >= 1, true);
  }
});

// ---------------------------------------------------------------------------
// ValidationError message contains path information
// ---------------------------------------------------------------------------
Deno.test("ValidationError: message includes field path info", () => {
  try {
    validateBody(testSchema, { name: "Alice", age: 30, role: "bad_role" });
    throw new Error("Expected ValidationError to be thrown");
  } catch (err) {
    assertInstanceOf(err, ValidationError);
    // Message should mention the field name
    assertEquals((err as ValidationError).message.includes("role"), true);
  }
});

// ---------------------------------------------------------------------------
// null body → ValidationError (not a raw crash)
// ---------------------------------------------------------------------------
Deno.test("validateBody: null body throws ValidationError", () => {
  assertThrows(() => validateBody(testSchema, null), ValidationError);
});

// ---------------------------------------------------------------------------
// Empty object → ValidationError
// ---------------------------------------------------------------------------
Deno.test("validateBody: empty object throws ValidationError", () => {
  assertThrows(() => validateBody(testSchema, {}), ValidationError);
});
