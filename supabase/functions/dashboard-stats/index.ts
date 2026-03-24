/**
 * dashboard-stats — POST /functions/v1/dashboard-stats
 *
 * Bundles all dashboard data into a single response by calling four RPC
 * functions in parallel.
 *
 * Response 200: { stats, heatmap, recent_views, most_accessed }
 */
import { handleCors, corsHeaders } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";

// ---------------------------------------------------------------------------
// Defaults
// ---------------------------------------------------------------------------

const DEFAULT_STATS = {
  total_memories: 0,
  searches_24h: 0,
  reports_24h: 0,
};

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
      "Send a POST request.",
    );
  }

  // Verify JWT and get authenticated client
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

  // Call all four RPC functions in parallel
  const [statsResult, heatmapResult, recentViewsResult, mostAccessedResult] =
    await Promise.all([
      supabase.rpc("dashboard_stats"),
      supabase.rpc("dashboard_activity_heatmap", { months: 6 }),
      supabase.rpc("dashboard_recent_views", { lim: 20 }),
      supabase.rpc("dashboard_most_accessed", { lim: 20 }),
    ]);

  // Stats is critical — return error if it fails
  if (statsResult.error) {
    console.error("dashboard-stats: dashboard_stats RPC error", statsResult.error);
    return errorResponse(
      500,
      "STATS_FAILED",
      "Failed to retrieve dashboard statistics.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Extract single-row stats result, defaulting if null
  const statsRow = Array.isArray(statsResult.data) ? statsResult.data[0] : null;
  const stats = statsRow ?? DEFAULT_STATS;

  // Non-critical results — silently default to empty arrays on failure
  if (heatmapResult.error) {
    console.warn("dashboard-stats: dashboard_activity_heatmap RPC error", heatmapResult.error);
  }
  if (recentViewsResult.error) {
    console.warn("dashboard-stats: dashboard_recent_views RPC error", recentViewsResult.error);
  }
  if (mostAccessedResult.error) {
    console.warn("dashboard-stats: dashboard_most_accessed RPC error", mostAccessedResult.error);
  }

  const heatmap = Array.isArray(heatmapResult.data) ? heatmapResult.data : [];
  const recent_views = Array.isArray(recentViewsResult.data) ? recentViewsResult.data : [];
  const most_accessed = Array.isArray(mostAccessedResult.data) ? mostAccessedResult.data : [];

  return new Response(
    JSON.stringify({ stats, heatmap, recent_views, most_accessed }),
    {
      status: 200,
      headers: {
        "Content-Type": "application/json",
        ...corsHeaders,
      },
    },
  );
});
