/**
 * feedback-submit — POST /functions/v1/feedback-submit
 *
 * Records user feedback on a memory and adjusts its scoring signals.
 *
 * Rating effects:
 *   helpful  → reinforcement_score += 1
 *   outdated → decay_score *= 0.8
 *   damaging → decay_score *= 0.5
 *
 * Request body (JSON):
 *   memory_id  (required) — UUID of the memory being rated
 *   rating     (required) — "helpful" | "outdated" | "damaging"
 *   context    (optional) — free-text explanation
 *
 * Response 201: { id: string, memory_id: string, rating: string }
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

const RATINGS = ["helpful", "outdated", "damaging"] as const;

const feedbackSubmitSchema = z.object({
  memory_id: z.string().uuid(),
  rating: z.enum(RATINGS),
  context: z.string().max(2000).optional(),
});

type FeedbackSubmitInput = z.infer<typeof feedbackSubmitSchema>;

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
  let input: FeedbackSubmitInput;
  try {
    input = validateBody(feedbackSubmitSchema, rawBody);
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

  const { memory_id, rating, context } = input;

  // Insert feedback record
  const { data: feedbackData, error: feedbackError } = await supabase
    .from("feedback")
    .insert({
      memory_id,
      user_id: userId,
      rating,
      context: context ?? null,
    })
    .select("id, memory_id, rating")
    .single();

  if (feedbackError) {
    // If the memory_id FK constraint fails the memory does not exist
    if (
      feedbackError.code === "23503" ||
      feedbackError.code === "PGRST116"
    ) {
      return errorResponse(
        404,
        "NOT_FOUND",
        `Memory with id '${memory_id}' not found.`,
        "Check that the memory_id is correct.",
      );
    }
    console.error("feedback-submit: insert error", feedbackError);
    return errorResponse(
      500,
      "INSERT_FAILED",
      "Failed to record feedback.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Adjust memory scores based on rating.
  // We read current scores first and compute the new values in application
  // code to avoid needing a Postgres function.
  const { data: memoryData, error: fetchError } = await supabase
    .from("memory")
    .select("reinforcement_score, decay_score")
    .eq("id", memory_id)
    .is("deleted_at", null)
    .single();

  if (!fetchError && memoryData) {
    const mem = memoryData as {
      reinforcement_score: number;
      decay_score: number;
    };

    let newReinforcement = mem.reinforcement_score;
    let newDecay = mem.decay_score;

    if (rating === "helpful") {
      newReinforcement += 1;
    } else if (rating === "outdated") {
      newDecay *= 0.8;
    } else if (rating === "damaging") {
      newDecay *= 0.5;
    }

    const { error: scoreError } = await supabase
      .from("memory")
      .update({
        reinforcement_score: newReinforcement,
        decay_score: newDecay,
      })
      .eq("id", memory_id);

    if (scoreError) {
      // Score update failure is non-fatal — feedback was recorded successfully
      console.error(
        "feedback-submit: failed to update memory scores",
        scoreError,
      );
    }
  } else if (fetchError) {
    console.error(
      "feedback-submit: failed to fetch memory for score update",
      fetchError,
    );
  }

  // Log activity (non-fatal)
  await logActivity(supabase, {
    userId,
    action: "feedback.submitted",
    entityType: "memory",
    entityId: memory_id,
    metadata: { rating },
  });

  const fb = feedbackData as { id: string; memory_id: string; rating: string };

  return new Response(
    JSON.stringify({
      id: fb.id,
      memory_id: fb.memory_id,
      rating: fb.rating,
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
