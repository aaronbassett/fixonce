/**
 * memory-create — POST /functions/v1/memory-create
 *
 * Creates a new memory record for the authenticated user.
 *
 * Request body (JSON):
 *   Required: title, content, summary, memory_type, source_type
 *   Optional: language, embedding, version fields (compact_pragma, compact_compiler,
 *             midnight_js, indexer_version, node_version), source_url, repo_url,
 *             task_summary, session_id
 *
 * Response 201: { id: string, created_at: string }
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

const memoryCreateSchema = z.object({
  // Required fields
  title: z.string().min(1).max(500),
  content: z.string().min(1),
  summary: z.string().min(1).max(2000),
  memory_type: z.enum(MEMORY_TYPES),
  source_type: z.enum(SOURCE_TYPES),

  // Optional fields
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

type MemoryCreateInput = z.infer<typeof memoryCreateSchema>;

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
  let input: MemoryCreateInput;
  try {
    input = validateBody(memoryCreateSchema, rawBody);
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

  // Insert into memory table
  const { data, error } = await supabase
    .from("memory")
    .insert({
      title: input.title,
      content: input.content,
      summary: input.summary,
      memory_type: input.memory_type,
      source_type: input.source_type,
      language: input.language ?? null,
      embedding: input.embedding ?? null,
      compact_pragma: input.compact_pragma ?? null,
      compact_compiler: input.compact_compiler ?? null,
      midnight_js: input.midnight_js ?? null,
      indexer_version: input.indexer_version ?? null,
      node_version: input.node_version ?? null,
      source_url: input.source_url ?? null,
      repo_url: input.repo_url ?? null,
      task_summary: input.task_summary ?? null,
      session_id: input.session_id ?? null,
      created_by: userId,
    })
    .select("id, created_at")
    .single();

  if (error) {
    console.error("memory-create: insert error", error);
    return errorResponse(
      500,
      "INSERT_FAILED",
      "Failed to create memory record.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Log activity (non-fatal — errors are swallowed inside logActivity)
  await logActivity(supabase, {
    userId,
    action: "memory.created",
    entityType: "memory",
    entityId: (data as { id: string; created_at: string }).id,
    metadata: {
      memory_type: input.memory_type,
      source_type: input.source_type,
    },
  });

  return new Response(
    JSON.stringify({
      id: (data as { id: string; created_at: string }).id,
      created_at: (data as { id: string; created_at: string }).created_at,
    }),
    {
      status: 201,
      headers: {
        "Content-Type": "application/json",
        ...corsHeaders,
      },
    },
  );
});
