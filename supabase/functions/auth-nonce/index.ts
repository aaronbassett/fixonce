/**
 * auth-nonce — POST /functions/v1/auth-nonce
 *
 * Issues a short-lived nonce for CLI authentication via Ed25519.
 * No authorization required — this is the first step of the auth handshake.
 *
 * Request body (JSON):
 *   { "public_key": "base64 Ed25519 public key" }
 *
 * Response 200:
 *   { "nonce": "hex string", "expires_at": "ISO timestamp (5 min)" }
 *
 * The nonce is stored in activity_log so auth-verify can confirm it was
 * legitimately issued. Expiry is enforced by auth-verify comparing
 * expires_at against now().
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { createServiceClient } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const nonceRequestSchema = z.object({
  public_key: z
    .string()
    .min(1)
    .refine(
      (v) => {
        try {
          const bytes = Uint8Array.from(atob(v), (c) => c.charCodeAt(0));
          // Ed25519 public keys are exactly 32 bytes.
          return bytes.length === 32;
        } catch {
          return false;
        }
      },
      {
        message:
          "public_key must be a valid base64-encoded 32-byte Ed25519 key",
      },
    ),
});

type NonceRequest = z.infer<typeof nonceRequestSchema>;

// ---------------------------------------------------------------------------
// Helper — generate a cryptographically random hex nonce (32 bytes = 64 hex chars)
// ---------------------------------------------------------------------------

function generateNonce(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(32));
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

Deno.serve(async (req: Request): Promise<Response> => {
  // Handle CORS preflight
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  if (req.method !== "POST") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only POST requests are accepted.",
      "Send a POST request with a JSON body containing public_key.",
    );
  }

  // Parse JSON body
  let rawBody: unknown;
  try {
    rawBody = await req.json();
  } catch {
    return errorResponse(
      400,
      "INVALID_JSON",
      "Request body is not valid JSON.",
      "Ensure the request body is well-formed JSON.",
    );
  }

  // Validate input
  let input: NonceRequest;
  try {
    input = validateBody(nonceRequestSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid base64-encoded 32-byte Ed25519 public key.",
      );
    }
    throw err;
  }

  // Generate nonce and expiry (5 minutes from now)
  const nonce = generateNonce();
  const expiresAt = new Date(Date.now() + 5 * 60 * 1000).toISOString();

  // Store in activity_log so auth-verify can confirm this nonce was issued.
  // Use service-role client because activity_log INSERT is service_role only.
  try {
    const serviceClient = createServiceClient();
    const { error } = await serviceClient.from("activity_log").insert({
      user_id: null,
      action: "auth.nonce_issued",
      entity_type: "cli_key",
      entity_id: null,
      metadata: {
        nonce,
        public_key: input.public_key,
        expires_at: expiresAt,
      },
    });
    if (error) {
      console.error("auth-nonce: failed to log nonce", error);
      return errorResponse(
        500,
        "NONCE_STORE_FAILED",
        "Failed to issue authentication nonce.",
        "Retry the request. If the problem persists, contact support.",
      );
    }
  } catch (err) {
    console.error("auth-nonce: unexpected error storing nonce", err);
    return errorResponse(
      500,
      "INTERNAL_ERROR",
      "Unexpected server error.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  return new Response(
    JSON.stringify({ nonce, expires_at: expiresAt }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
