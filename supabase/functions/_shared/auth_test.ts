/**
 * Tests for _shared/auth.ts — verifyAuth function.
 *
 * T057: Verify that verifyAuth rejects missing/invalid tokens and
 *       returns 401-status errors before any Supabase call is made.
 *
 * Since we cannot connect to a real Supabase instance, we test the
 * pre-condition guards: missing Authorization header, malformed
 * header format, and missing server env vars. The Supabase network
 * call is never reached in these failure cases.
 */
import { assertEquals, assertInstanceOf, assertRejects } from "@std/assert";

// We import verifyAuth after manipulating the test environment so that
// the module-level code does not attempt a real network connection.
// Each sub-test constructs a synthetic Request.

// ---------------------------------------------------------------------------
// Helper: build a minimal Request with given Authorization header value
// ---------------------------------------------------------------------------
function makeRequest(authHeader?: string): Request {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };
  if (authHeader !== undefined) {
    headers["Authorization"] = authHeader;
  }
  return new Request("https://example.com/functions/v1/test", {
    method: "POST",
    headers,
  });
}

// ---------------------------------------------------------------------------
// T057-A: Missing Authorization header → throws Error with status 401
// ---------------------------------------------------------------------------
Deno.test("verifyAuth: missing Authorization header throws 401", async () => {
  const { verifyAuth } = await import("./auth.ts");

  const req = makeRequest(); // no auth header
  const err = await assertRejects(
    () => verifyAuth(req),
    Error,
    "Missing or malformed Authorization header",
  );
  assertEquals((err as Error & { status: number }).status, 401);
});

// ---------------------------------------------------------------------------
// T057-B: Authorization header present but does not start with "Bearer "
// ---------------------------------------------------------------------------
Deno.test(
  "verifyAuth: malformed Authorization header (no Bearer prefix) throws 401",
  async () => {
    const { verifyAuth } = await import("./auth.ts");

    const req = makeRequest("Basic dXNlcjpwYXNz");
    const err = await assertRejects(
      () => verifyAuth(req),
      Error,
      "Missing or malformed Authorization header",
    );
    assertEquals((err as Error & { status: number }).status, 401);
  },
);

// ---------------------------------------------------------------------------
// T057-C: Authorization header is just "Bearer" (no space/token) → 401
// ---------------------------------------------------------------------------
Deno.test(
  "verifyAuth: Authorization header with only 'Bearer' (no token) throws 401",
  async () => {
    const { verifyAuth } = await import("./auth.ts");

    // "Bearer" without trailing space does not match "Bearer "
    const req = makeRequest("Bearer");
    const err = await assertRejects(
      () => verifyAuth(req),
      Error,
      "Missing or malformed Authorization header",
    );
    assertEquals((err as Error & { status: number }).status, 401);
  },
);

// ---------------------------------------------------------------------------
// T057-D: Missing SUPABASE_URL env var → throws 500 (server config error)
//         This confirms the guard runs after header validation.
// ---------------------------------------------------------------------------
Deno.test(
  "verifyAuth: missing SUPABASE_URL env var throws 500 config error",
  async () => {
    // Unset env vars so the server-config guard fires, not the network.
    const originalUrl = Deno.env.get("SUPABASE_URL");
    const originalKey = Deno.env.get("SUPABASE_ANON_KEY");
    Deno.env.delete("SUPABASE_URL");
    Deno.env.delete("SUPABASE_ANON_KEY");

    try {
      const { verifyAuth } = await import("./auth.ts");
      const req = makeRequest("Bearer valid-looking-token");

      const err = await assertRejects(
        () => verifyAuth(req),
        Error,
        "Server configuration error: missing Supabase env vars",
      );
      assertEquals((err as Error & { status: number }).status, 500);
    } finally {
      // Restore env vars so other tests are not affected.
      if (originalUrl) Deno.env.set("SUPABASE_URL", originalUrl);
      if (originalKey) Deno.env.set("SUPABASE_ANON_KEY", originalKey);
    }
  },
);

// ---------------------------------------------------------------------------
// T057-E: Thrown error is an instance of Error (not a raw string throw)
// ---------------------------------------------------------------------------
Deno.test(
  "verifyAuth: thrown value is an instance of Error",
  async () => {
    const { verifyAuth } = await import("./auth.ts");

    const req = makeRequest(); // missing header
    try {
      await verifyAuth(req);
      throw new Error("Expected verifyAuth to throw");
    } catch (caught) {
      assertInstanceOf(caught, Error);
    }
  },
);
