/**
 * memory-update — POST /functions/v1/memory-update
 *
 * Updates an existing memory record owned by the authenticated user.
 *
 * Request body (JSON):
 *   Required: id (UUID of the memory to update)
 *   Optional: title, content, summary, memory_type, source_type,
 *             language, embedding, compact_pragma, compact_compiler,
 *             midnight_js, indexer_version, node_version, source_url,
 *             repo_url, task_summary, session_id
 *
 * If content is changed and no new embedding is provided, embedding_status
 * is set to "pending" so the pipeline can regenerate it.
 *
 * Response 200: { id: string, updated_at: string }
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

const memoryUpdateSchema = z.object({
  // Required
  id: z.string().uuid(),

  // Optional updatable fields
  title: z.string().min(1).max(500).optional(),
  content: z.string().min(1).optional(),
  summary: z.string().min(1).max(2000).optional(),
  memory_type: z.enum(MEMORY_TYPES).optional(),
  source_type: z.enum(SOURCE_TYPES).optional(),
  language: z.string().max(100).optional(),
  embedding: z.array(z.number()).length(1024).optional(),

  // Version / environment metadata
  compact_pragma: z.string().max(200).optional(),
  compact_compiler: z.string().max(200).optional(),
  midnight_js: z.string().max(200).optional(),
  indexer_version: z.string().max(200).optional(),
  node_version: z.string().max(200).optional(),

  // Provenance
  source_url: z.string().url().optional(),
  repo_url: z.string().url().optional(),
  task_summary: z.string().max(1000).optional(),
  session_id: z.string().max(200).optional(),
});

type MemoryUpdateInput = z.infer<typeof memoryUpdateSchema>;

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
  let input: MemoryUpdateInput;
  try {
    input = validateBody(memoryUpdateSchema, rawBody);
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

  // Build the update patch — only include fields that were provided
  const { id, embedding, content, ...rest } = input;

  // deno-lint-ignore no-explicit-any
  const patch: Record<string, any> = {};

  // Copy optional scalar fields if present
  const optionalFields = [
    "title",
    "summary",
    "memory_type",
    "source_type",
    "language",
    "compact_pragma",
    "compact_compiler",
    "midnight_js",
    "indexer_version",
    "node_version",
    "source_url",
    "repo_url",
    "task_summary",
    "session_id",
  ] as const;

  for (const field of optionalFields) {
    if (rest[field] !== undefined) {
      patch[field] = rest[field];
    }
  }

  if (content !== undefined) {
    patch["content"] = content;
  }

  if (embedding !== undefined) {
    // Caller supplied a new embedding — store it and mark complete
    patch["embedding"] = embedding;
    patch["embedding_status"] = "complete";
  } else if (content !== undefined) {
    // Content changed but no embedding provided — flag for regeneration
    patch["embedding_status"] = "pending";
  }

  if (Object.keys(patch).length === 0) {
    return errorResponse(
      400,
      "NO_FIELDS",
      "No updatable fields were provided.",
      "Include at least one field to update in the request body.",
    );
  }

  // Perform the update — RLS ensures only the owner can update
  const { data, error } = await supabase
    .from("memory")
    .update(patch)
    .eq("id", id)
    .eq("created_by", userId)
    .is("deleted_at", null)
    .select("id, updated_at")
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
    console.error("memory-update: update error", error);
    return errorResponse(
      500,
      "UPDATE_FAILED",
      "Failed to update memory record.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Log activity (non-fatal)
  await logActivity(supabase, {
    userId,
    action: "memory.update",
    entityType: "memory",
    entityId: id,
    metadata: { updated_fields: Object.keys(patch) },
  });

  return new Response(
    JSON.stringify({
      id: (data as { id: string; updated_at: string }).id,
      updated_at: (data as { id: string; updated_at: string }).updated_at,
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
