/**
 * keys-list — GET /functions/v1/keys-list
 *
 * Returns all Ed25519 public keys registered by the authenticated user.
 * Public keys are truncated in the response for display purposes.
 *
 * Requires: Authorization: Bearer <token>
 *
 * Response 200:
 *   {
 *     "keys": [
 *       {
 *         "id": "uuid",
 *         "label": "string | null",
 *         "public_key": "truncated base64 (first 12 chars + '...')",
 *         "last_used_at": "ISO timestamp | null",
 *         "created_at": "ISO timestamp"
 *       }
 *     ]
 *   }
 */
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface CliKeyRow {
  id: string;
  label: string | null;
  public_key: string;
  last_used_at: string | null;
  created_at: string;
}

interface CliKeyDisplay {
  id: string;
  label: string | null;
  public_key: string;
  last_used_at: string | null;
  created_at: string;
}

// ---------------------------------------------------------------------------
// Helper — truncate public key for display
// ---------------------------------------------------------------------------

function truncatePublicKey(key: string): string {
  return key.length > 12 ? `${key.slice(0, 12)}...` : key;
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

  // Fetch the user's keys (RLS ensures only own keys are returned)
  const { data, error } = await supabase
    .from("cli_keys")
    .select("id, label, public_key, last_used_at, created_at")
    .eq("user_id", userId)
    .order("created_at", { ascending: false });

  if (error) {
    console.error("keys-list: select error", error);
    return errorResponse(
      500,
      "FETCH_FAILED",
      "Failed to retrieve registered keys.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  const keys: CliKeyDisplay[] = ((data as CliKeyRow[]) ?? []).map((row) => ({
    id: row.id,
    label: row.label,
    public_key: truncatePublicKey(row.public_key),
    last_used_at: row.last_used_at,
    created_at: row.created_at,
  }));

  return new Response(
    JSON.stringify({ keys }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
