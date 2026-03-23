/**
 * activity-stream — GET /functions/v1/activity-stream
 *
 * Returns recent entries from the `activity_log` table.
 *
 * Query parameters:
 *   since  (optional) — ISO-8601 timestamp; only return entries after this time
 *   limit  (optional) — maximum number of entries to return (default: 50, max: 200)
 *
 * Response 200:
 *   {
 *     "entries": [
 *       {
 *         "id":          "uuid",
 *         "user_id":     "uuid | null",
 *         "action":      "string",
 *         "entity_type": "string",
 *         "entity_id":   "uuid | null",
 *         "metadata":    "object",
 *         "created_at":  "ISO timestamp"
 *       }
 *     ],
 *     "total": number
 *   }
 *
 * Requires: Authorization: Bearer <token>
 */
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_LIMIT = 50;
const MAX_LIMIT = 200;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ActivityLogRow {
  id: string;
  user_id: string | null;
  action: string;
  entity_type: string;
  entity_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

Deno.serve(async (req: Request): Promise<Response> => {
  // Handle CORS preflight
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  if (req.method !== "GET") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only GET requests are accepted.",
      "Send a GET request with a valid Authorization header.",
    );
  }

  // Verify authentication
  let supabase: Awaited<ReturnType<typeof verifyAuth>>["supabase"];
  try {
    ({ supabase } = await verifyAuth(req));
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

  // Parse query parameters
  const url = new URL(req.url);
  const sinceParam = url.searchParams.get("since");
  const limitParam = url.searchParams.get("limit");

  // Validate and clamp limit
  let limit = DEFAULT_LIMIT;
  if (limitParam !== null) {
    const parsed = parseInt(limitParam, 10);
    if (isNaN(parsed) || parsed < 1) {
      return errorResponse(
        400,
        "INVALID_PARAMETER",
        "Query parameter 'limit' must be a positive integer.",
        `Provide a number between 1 and ${MAX_LIMIT}.`,
      );
    }
    limit = Math.min(parsed, MAX_LIMIT);
  }

  // Validate since timestamp if provided
  if (sinceParam !== null) {
    const ts = Date.parse(sinceParam);
    if (isNaN(ts)) {
      return errorResponse(
        400,
        "INVALID_PARAMETER",
        "Query parameter 'since' must be a valid ISO-8601 timestamp.",
        "Example: since=2024-01-01T00:00:00Z",
      );
    }
  }

  // Build query
  let query = supabase
    .from("activity_log")
    .select(
      "id, user_id, action, entity_type, entity_id, metadata, created_at",
    )
    .order("created_at", { ascending: false })
    .limit(limit);

  if (sinceParam !== null) {
    query = query.gt("created_at", sinceParam);
  }

  const { data, error } = await query;

  if (error) {
    console.error("activity-stream: select error", error);
    return errorResponse(
      500,
      "FETCH_FAILED",
      "Failed to retrieve activity log entries.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  const entries: ActivityLogRow[] = (data as ActivityLogRow[]) ?? [];

  return new Response(
    JSON.stringify({
      entries,
      total: entries.length,
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
