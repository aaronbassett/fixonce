/**
 * memory-delete — POST /functions/v1/memory-delete
 *
 * Soft-deletes a memory record owned by the authenticated user.
 *
 * Soft-delete sets deleted_at to now() — the row is preserved in full
 * to maintain lineage.  No cascade occurs.
 *
 * Request body (JSON):
 *   id  (required) — UUID of the memory to delete
 *
 * Response 200: { id: string, deleted_at: string }
 */
import { z } from "zod";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { logActivity } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const memoryDeleteSchema = z.object({
  id: z.string().uuid(),
});

type MemoryDeleteInput = z.infer<typeof memoryDeleteSchema>;

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

Deno.serve(async (req: Request): Promise<Response> => {
  // Handle CORS preflight
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  // Only accept POST
  if (req.method !== "POST") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only POST requests are accepted.",
      "Send a POST request with a JSON body.",
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

  // Validate input schema
  let input: MemoryDeleteInput;
  try {
    input = validateBody(memoryDeleteSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Check the request body against the required schema.",
      );
    }
    throw err;
  }

  // Verify JWT and get authenticated client
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

  const { id } = input;

  // Soft-delete: set deleted_at = now() for the memory owned by this user.
  // RLS and the .eq("created_by", userId) filter together ensure only the
  // owner can delete.  Lineage is preserved — no cascade.
  const { data, error } = await supabase
    .from("memory")
    .update({ deleted_at: new Date().toISOString() })
    .eq("id", id)
    .eq("created_by", userId)
    .is("deleted_at", null)
    .select("id, deleted_at")
    .single();

  if (error) {
    if (error.code === "PGRST116") {
      return errorResponse(
        404,
        "NOT_FOUND",
        `Memory with id '${id}' not found or not owned by you.`,
        "Check the id and ensure you own this memory.",
      );
    }
    console.error("memory-delete: update error", error);
    return errorResponse(
      500,
      "DELETE_FAILED",
      "Failed to soft-delete memory record.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Log activity (non-fatal)
  await logActivity(supabase, {
    userId,
    action: "memory.delete",
    entityType: "memory",
    entityId: id,
    metadata: {},
  });

  return new Response(
    JSON.stringify({
      id: (data as { id: string; deleted_at: string }).id,
      deleted_at: (data as { id: string; deleted_at: string }).deleted_at,
    }),
    {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        ...corsHeaders,
      },
    },
  );
});
