/**
 * memory-get — GET /functions/v1/memory-get?id=<uuid>
 *
 * Retrieves a single memory by its UUID.
 *
 * Query parameters:
 *   id                (required) — UUID of the memory to fetch
 *   include_embedding (optional) — "true" to include the embedding vector
 *
 * Response 200: full memory object (embedding excluded by default)
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";

// UUID v4 regex for basic format validation
const uuidSchema = z.string().uuid();

Deno.serve(async (req: Request): Promise<Response> => {
  // Handle CORS preflight
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  // Only accept GET
  if (req.method !== "GET") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only GET requests are accepted.",
      "Send a GET request with the memory id as a query parameter.",
    );
  }

  // Verify JWT
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

  // Parse query params
  const url = new URL(req.url);
  const id = url.searchParams.get("id");
  const includeEmbedding = url.searchParams.get("include_embedding") === "true";

  if (!id) {
    return errorResponse(
      400,
      "MISSING_PARAMETER",
      "Query parameter 'id' is required.",
      "Append ?id=<memory-uuid> to the request URL.",
    );
  }

  // Validate UUID format
  const parsed = uuidSchema.safeParse(id);
  if (!parsed.success) {
    return errorResponse(
      400,
      "INVALID_PARAMETER",
      "Query parameter 'id' must be a valid UUID.",
      "Provide a correctly formatted UUID (e.g. 550e8400-e29b-41d4-a716-446655440000).",
    );
  }

  // Build select columns — exclude embedding by default
  const columns = includeEmbedding
    ? "*"
    : "id, title, content, summary, memory_type, source_type, language, fts_vector, compact_pragma, compact_compiler, midnight_js, indexer_version, node_version, source_url, repo_url, task_summary, session_id, decay_score, reinforcement_score, last_accessed_at, embedding_status, pipeline_status, deleted_at, created_at, updated_at, created_by";

  const { data, error } = await supabase
    .from("memory")
    .select(columns)
    .eq("id", parsed.data)
    .is("deleted_at", null)
    .single();

  if (error) {
    // PostgREST returns code PGRST116 when no rows are returned from .single()
    if (error.code === "PGRST116") {
      return errorResponse(
        404,
        "NOT_FOUND",
        `Memory with id '${id}' not found.`,
        "Check the id and ensure the memory has not been deleted.",
      );
    }
    console.error("memory-get: select error", error);
    return errorResponse(
      500,
      "FETCH_FAILED",
      "Failed to retrieve memory record.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Suppress unused variable warning — userId is captured for potential future
  // last_accessed_at update or audit use, but we intentionally keep this light.
  void userId;

  return new Response(JSON.stringify(data), {
    status: 200,
    headers: {
      "Content-Type": "application/json",
      ...corsHeaders,
    },
  });
});
