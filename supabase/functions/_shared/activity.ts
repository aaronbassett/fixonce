/**
 * Activity log insertion helper.
 *
 * Uses the service_role client so it can bypass RLS
 * (activity_log INSERT is restricted to service_role only).
 */
import { createClient, type SupabaseClient } from "@supabase/supabase-js";

export interface ActivityParams {
  userId?: string;
  action: string;
  entityType: string;
  entityId?: string;
  metadata?: Record<string, unknown>;
}

/**
 * Create a service-role Supabase client.
 * Exported so functions that need service-role access beyond activity logging
 * can reuse it without duplicating env-var reads.
 */
export function createServiceClient(): SupabaseClient {
  const supabaseUrl = Deno.env.get("SUPABASE_URL");
  const serviceRoleKey = Deno.env.get("SUPABASE_SERVICE_ROLE_KEY");

  if (!supabaseUrl || !serviceRoleKey) {
    throw new Error(
      "Server configuration error: missing SUPABASE_URL or SUPABASE_SERVICE_ROLE_KEY",
    );
  }

  return createClient(supabaseUrl, serviceRoleKey, {
    auth: {
      // Disable auto-refresh / persistence — not needed in edge functions.
      autoRefreshToken: false,
      persistSession: false,
    },
  });
}

/**
 * Insert a row into activity_log using the service-role client.
 * Errors are logged to console but do NOT propagate — a logging failure
 * should never prevent the primary operation from completing.
 */
export async function logActivity(
  _supabase: SupabaseClient,
  params: ActivityParams,
): Promise<void> {
  try {
    const serviceClient = createServiceClient();

    const { error } = await serviceClient.from("activity_log").insert({
      user_id: params.userId ?? null,
      action: params.action,
      entity_type: params.entityType,
      entity_id: params.entityId ?? null,
      metadata: params.metadata ?? {},
    });

    if (error) {
      console.error("logActivity: failed to insert activity_log row", error);
    }
  } catch (err) {
    console.error("logActivity: unexpected error", err);
  }
}
