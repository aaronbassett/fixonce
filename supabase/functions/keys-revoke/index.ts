/**
 * keys-revoke — POST /functions/v1/keys-revoke
 *
 * Revokes (deletes) a registered Ed25519 public key.
 * RLS ensures users may only delete their own keys.
 *
 * Requires: Authorization: Bearer <token>
 *
 * Request body (JSON):
 *   { "key_id": "uuid" }
 *
 * Response 200:
 *   { "revoked": true }
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

const revokeSchema = z.object({
  key_id: z.string().uuid({ message: "key_id must be a valid UUID" }),
});

type RevokeInput = z.infer<typeof revokeSchema>;

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
      "Send a POST request with a JSON body containing key_id.",
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

  // Validate input
  let input: RevokeInput;
  try {
    input = validateBody(revokeSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid UUID for key_id.",
      );
    }
    throw err;
  }

  // Delete the key — RLS (cli_keys_delete_own) ensures only own keys are deleted.
  // We check rowCount to distinguish "not found / not owned" from DB errors.
  const { error, count } = await supabase
    .from("cli_keys")
    .delete({ count: "exact" })
    .eq("id", input.key_id)
    .eq("user_id", userId);

  if (error) {
    console.error("keys-revoke: delete error", error);
    return errorResponse(
      500,
      "DELETE_FAILED",
      "Failed to revoke key.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  if (count === 0) {
    return errorResponse(
      404,
      "KEY_NOT_FOUND",
      "No key found with the given key_id for this user.",
      "Verify the key_id using GET /functions/v1/keys-list.",
    );
  }

  // Log activity (non-fatal)
  const serviceClient = createServiceClient();
  await logActivity(serviceClient, {
    userId,
    action: "key.revoked",
    entityType: "cli_key",
    entityId: input.key_id,
    metadata: {},
  });

  return new Response(
    JSON.stringify({ revoked: true }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
