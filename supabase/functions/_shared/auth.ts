/**
 * JWT verification and authenticated Supabase client creation.
 *
 * Verifies the Bearer token in the Authorization header via
 * supabase.auth.getUser() and returns the validated user ID
 * along with an authenticated client for RLS-scoped operations.
 */
import { createClient, type SupabaseClient } from "jsr:@supabase/supabase-js@2";

export interface AuthResult {
  userId: string;
  supabase: SupabaseClient;
}

/**
 * Verify the JWT in the Authorization header and return an authenticated
 * Supabase client scoped to that user.
 *
 * Throws a plain Error (with a `status` property) on failure so callers
 * can convert it to the appropriate HTTP error response.
 */
export async function verifyAuth(req: Request): Promise<AuthResult> {
  const authHeader = req.headers.get("Authorization");
  if (!authHeader || !authHeader.startsWith("Bearer ")) {
    const err = new Error("Missing or malformed Authorization header");
    (err as Error & { status: number }).status = 401;
    throw err;
  }

  const supabaseUrl = Deno.env.get("SUPABASE_URL");
  const supabaseAnonKey = Deno.env.get("SUPABASE_ANON_KEY");

  if (!supabaseUrl || !supabaseAnonKey) {
    const err = new Error(
      "Server configuration error: missing Supabase env vars",
    );
    (err as Error & { status: number }).status = 500;
    throw err;
  }

  // Create a client that forwards the user's token so RLS applies correctly.
  const supabase = createClient(supabaseUrl, supabaseAnonKey, {
    global: {
      headers: { Authorization: authHeader },
    },
  });

  const { data, error } = await supabase.auth.getUser();
  if (error || !data.user) {
    const err = new Error(error?.message ?? "Invalid or expired token");
    (err as Error & { status: number }).status = 401;
    throw err;
  }

  return { userId: data.user.id, supabase };
}
