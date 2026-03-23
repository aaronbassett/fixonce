/**
 * memory-search — POST /functions/v1/memory-search
 *
 * Runs a hybrid (FTS + vector) search over the memory table via the
 * hybrid_search RPC function.
 *
 * Request body (JSON):
 *   query_text       (string, optional)  — full-text query
 *   query_embedding  (number[1024], opt) — vector for similarity search
 *   search_type      (string, optional)  — "hybrid" | "fts" | "vector" (default: "hybrid")
 *   limit            (number, optional)  — max results, 1-100 (default: 20)
 *   version_filters  (object, optional)  — version equality filters
 *   memory_type      (string, optional)  — filter by memory_type (post-filter)
 *   language         (string, optional)  — filter by language (post-filter)
 *
 * Response 200: { results: SearchResult[], total: number, search_type: string }
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const SEARCH_TYPES = ["hybrid", "fts", "vector"] as const;
const MEMORY_TYPES = [
  "gotcha",
  "best_practice",
  "correction",
  "anti_pattern",
  "discovery",
] as const;

const versionFiltersSchema = z
  .object({
    compact_pragma: z.string().optional(),
    compact_compiler: z.string().optional(),
    midnight_js: z.string().optional(),
    indexer_version: z.string().optional(),
    node_version: z.string().optional(),
  })
  .optional();

const memorySearchSchema = z
  .object({
    query_text: z.string().max(2000).optional(),
    query_embedding: z.array(z.number()).length(1024).optional(),
    search_type: z.enum(SEARCH_TYPES).default("hybrid"),
    limit: z.number().int().min(1).max(100).default(20),
    version_filters: versionFiltersSchema,
    memory_type: z.enum(MEMORY_TYPES).optional(),
    language: z.string().max(100).optional(),
  })
  .refine(
    (v) =>
      v.search_type === "vector"
        ? v.query_embedding !== undefined
        : v.search_type === "fts"
        ? v.query_text !== undefined
        : v.query_text !== undefined || v.query_embedding !== undefined,
    {
      message: "Provide at least one of query_text or query_embedding. " +
        "search_type='fts' requires query_text; " +
        "search_type='vector' requires query_embedding.",
      path: ["query_text"],
    },
  );

// z.output extracts the type after defaults are applied (search_type and limit
// are guaranteed non-undefined after parsing)
type MemorySearchInput = z.output<typeof memorySearchSchema>;

// ---------------------------------------------------------------------------
// tsvector query sanitization (EC-08)
// Strip characters that break plainto_tsquery: & | ! ( ) : * '
// plainto_tsquery() is actually tolerant of most punctuation, but we strip
// the tsquery operator characters to prevent injection via query_text.
// ---------------------------------------------------------------------------
function sanitizeQueryText(text: string): string {
  // Remove tsquery special characters
  return text.replace(/[&|!():*'\\]/g, " ").replace(/\s+/g, " ").trim();
}

// ---------------------------------------------------------------------------
// RPC result row type (matches hybrid_search return columns)
// ---------------------------------------------------------------------------
interface SearchResultRow {
  memory_id: string;
  title: string;
  summary: string;
  content: string;
  memory_type: string;
  language: string | null;
  compact_pragma: string | null;
  compact_compiler: string | null;
  midnight_js: string | null;
  indexer_version: string | null;
  node_version: string | null;
  source_url: string | null;
  decay_score: number;
  reinforcement_score: number;
  rrf_score: number;
  created_at: string;
  updated_at: string;
}

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

  // Parse JSON body — handle malformed JSON explicitly (EC-10)
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
  let input: MemorySearchInput;
  try {
    input = validateBody(memorySearchSchema, rawBody);
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

  // Verify JWT
  try {
    await verifyAuth(req);
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

  // We need the supabase client from verifyAuth for the RPC call
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

  // Sanitize query_text to prevent tsvector injection (EC-08)
  const sanitizedQueryText = input.query_text
    ? sanitizeQueryText(input.query_text)
    : null;

  // Build RPC params
  const rpcParams: Record<string, unknown> = {
    search_type: input.search_type,
    result_limit: input.limit,
    version_filters: input.version_filters ?? {},
  };

  // Only pass non-null values to avoid type errors in the RPC
  if (sanitizedQueryText) {
    rpcParams["query_text"] = sanitizedQueryText;
  } else {
    rpcParams["query_text"] = "";
  }

  if (input.query_embedding) {
    rpcParams["query_embedding"] = `[${input.query_embedding.join(",")}]`;
  } else {
    rpcParams["query_embedding"] = null;
  }

  const { data, error } = await supabase.rpc("hybrid_search", rpcParams);

  if (error) {
    console.error("memory-search: RPC error", error);
    return errorResponse(
      500,
      "SEARCH_FAILED",
      "Search operation failed.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // EC-09: empty vector results return empty array, not an error
  const rows: SearchResultRow[] = Array.isArray(data) ? data : [];

  // Apply post-filters (memory_type, language) — these are not supported
  // directly by the RPC, so we filter the results in-process.
  let filtered = rows;
  if (input.memory_type) {
    filtered = filtered.filter((r) => r.memory_type === input.memory_type);
  }
  if (input.language) {
    filtered = filtered.filter((r) => r.language === input.language);
  }

  return new Response(
    JSON.stringify({
      results: filtered,
      total: filtered.length,
      search_type: input.search_type,
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
