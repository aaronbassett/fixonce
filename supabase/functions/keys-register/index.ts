/**
 * keys-register — POST /functions/v1/keys-register
 *
 * Registers an Ed25519 public key for the authenticated user.
 *
 * Requires: Authorization: Bearer <token>
 *
 * Request body (JSON):
 *   { "public_key": "base64 Ed25519 public key", "label": "optional string" }
 *
 * Response 201:
 *   { "id": "uuid", "created_at": "ISO timestamp" }
 *
 * Error codes enforced:
 *   EC-15: Invalid key format (not a 32-byte Ed25519 key)
 *   EC-16: Unique constraint — duplicate key returns 409
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { createServiceClient, logActivity } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const registerSchema = z.object({
  public_key: z
    .string()
    .min(1)
    .refine(
      (v) => {
        try {
          const bytes = Uint8Array.from(atob(v), (c) => c.charCodeAt(0));
          // EC-15: Ed25519 public keys are exactly 32 bytes.
          return bytes.length === 32;
        } catch {
          return false;
        }
      },
      {
        message:
          "public_key must be a valid base64-encoded 32-byte Ed25519 public key (EC-15)",
      },
    ),
  label: z.string().max(255).optional(),
});

type RegisterInput = z.infer<typeof registerSchema>;

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

  // Verify authentication
  let userId: string;
  let supabase: Awaited<ReturnType<typeof verifyAuth>>["supabase"];
  try {
    ({ userId, supabase } = await verifyAuth(req));
  } catch (err) {
    const status = (err as Error & { status?: number }).status ?? 401;
    return errorResponse(
      status,
      status === 401 ? "UNAUTHORIZED" : "INTERNAL_ERROR",
      (err as Error).message,
      status === 401
        ? "Provide a valid Bearer token in the Authorization header."
        : "Contact support if this persists.",
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

  // Validate input (EC-15 enforced by schema)
  let input: RegisterInput;
  try {
    input = validateBody(registerSchema, rawBody);
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

  // Use service client for insertion since RLS requires user_id = auth.uid()
  // The authenticated supabase client respects RLS correctly for the user.
  const { data, error } = await supabase
    .from("cli_keys")
    .insert({
      user_id: userId,
      public_key: input.public_key,
      label: input.label ?? null,
    })
    .select("id, created_at")
    .single();

  if (error) {
    // EC-16: unique constraint violation on public_key
    if (
      error.code === "23505" ||
      error.message?.toLowerCase().includes("unique")
    ) {
      return errorResponse(
        409,
        "DUPLICATE_KEY",
        "This public key is already registered. (EC-16)",
        "Each public key may only be registered once. Use a different key or list your existing keys.",
      );
    }
    console.error("keys-register: insert error", error);
    return errorResponse(
      500,
      "INSERT_FAILED",
      "Failed to register public key.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  const typedData = data as { id: string; created_at: string };

  // Log activity (non-fatal)
  const serviceClient = createServiceClient();
  await logActivity(serviceClient, {
    userId,
    action: "key.registered",
    entityType: "cli_key",
    entityId: typedData.id,
    metadata: { label: input.label ?? null },
  });

  return new Response(
    JSON.stringify({ id: typedData.id, created_at: typedData.created_at }),
    {
      status: 201,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
