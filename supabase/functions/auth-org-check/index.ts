/**
 * auth-org-check — POST /functions/v1/auth-org-check
 *
 * Checks whether the authenticated user is a member of the configured
 * GitHub organisation. Results are cached in activity_log for 1 hour
 * to respect GitHub API rate limits (EC-12).
 *
 * Requires: Authorization: Bearer <token>
 *
 * Request body (JSON):
 *   { "github_access_token": "string" }
 *
 * Response 200:
 *   { "is_member": boolean, "org": string, "cached_until": "ISO timestamp" }
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { createServiceClient } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const orgCheckSchema = z.object({
  github_access_token: z.string().min(1),
});

type OrgCheckInput = z.infer<typeof orgCheckSchema>;

// Cache TTL: 1 hour
const CACHE_TTL_MS = 60 * 60 * 1000;

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

Deno.serve(async (req: Request): Promise<Response> => {
  // Handle CORS preflight
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  if (req.method !== "POST") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only POST requests are accepted.",
      "Send a POST request with a JSON body containing github_access_token.",
    );
  }

  // Verify authentication
  let userId: string;
  try {
    ({ userId } = await verifyAuth(req));
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

  // Validate input
  let input: OrgCheckInput;
  try {
    input = validateBody(orgCheckSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid github_access_token string.",
      );
    }
    throw err;
  }

  const githubOrg = Deno.env.get("GITHUB_ORG");
  if (!githubOrg) {
    console.error("auth-org-check: GITHUB_ORG env var not set");
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error: GITHUB_ORG is not configured.",
      "Contact support.",
    );
  }

  const serviceClient = createServiceClient();

  // Check activity_log cache (EC-12: respect GitHub API rate limits)
  // Look for a cached result within the TTL window for this user+org
  const cacheThreshold = new Date(Date.now() - CACHE_TTL_MS).toISOString();
  const { data: cacheRows, error: cacheError } = await serviceClient
    .from("activity_log")
    .select("id, metadata, created_at")
    .eq("user_id", userId)
    .eq("action", "auth.org_check")
    .eq("metadata->>org", githubOrg)
    .gte("created_at", cacheThreshold)
    .order("created_at", { ascending: false })
    .limit(1);

  if (!cacheError && cacheRows && cacheRows.length > 0) {
    const cached = cacheRows[0] as {
      metadata: Record<string, unknown>;
      created_at: string;
    };
    const cachedUntil = new Date(
      new Date(cached.created_at).getTime() + CACHE_TTL_MS,
    ).toISOString();

    return new Response(
      JSON.stringify({
        is_member: cached.metadata["is_member"] as boolean,
        org: githubOrg,
        cached_until: cachedUntil,
      }),
      {
        status: 200,
        headers: { "Content-Type": "application/json", ...corsHeaders },
      },
    );
  }

  // Cache miss — call GitHub API
  let isMember = false;
  try {
    // GET /orgs/{org}/members/{username} returns 204 if member, 302/404 if not
    // We first need the GitHub username from the token owner
    const userResponse = await fetch("https://api.github.com/user", {
      headers: {
        Authorization: `Bearer ${input.github_access_token}`,
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
        "User-Agent": "FixOnce-Auth",
      },
    });

    if (userResponse.status === 401 || userResponse.status === 403) {
      return errorResponse(
        401,
        "GITHUB_AUTH_FAILED",
        "The GitHub access token is invalid or lacks required permissions.",
        "Provide a valid GitHub access token with read:org scope.",
      );
    }

    if (userResponse.status === 429) {
      // Rate limited — return last cached value if available, else error
      return errorResponse(
        429,
        "GITHUB_RATE_LIMITED",
        "GitHub API rate limit exceeded.",
        "Wait before retrying. Cached results will be used when available.",
      );
    }

    if (!userResponse.ok) {
      console.error(
        "auth-org-check: GitHub /user failed",
        userResponse.status,
      );
      return errorResponse(
        502,
        "GITHUB_API_ERROR",
        "Failed to fetch GitHub user information.",
        "Retry the request. If the problem persists, contact support.",
      );
    }

    const githubUser = (await userResponse.json()) as Record<string, unknown>;
    const githubLogin = githubUser["login"];
    if (typeof githubLogin !== "string") {
      return errorResponse(
        502,
        "GITHUB_API_ERROR",
        "Unexpected response from GitHub API.",
        "Retry the request.",
      );
    }

    // Check org membership
    const memberResponse = await fetch(
      `https://api.github.com/orgs/${encodeURIComponent(githubOrg)}/members/${
        encodeURIComponent(githubLogin)
      }`,
      {
        headers: {
          Authorization: `Bearer ${input.github_access_token}`,
          Accept: "application/vnd.github+json",
          "X-GitHub-Api-Version": "2022-11-28",
          "User-Agent": "FixOnce-Auth",
        },
      },
    );

    if (memberResponse.status === 429) {
      return errorResponse(
        429,
        "GITHUB_RATE_LIMITED",
        "GitHub API rate limit exceeded.",
        "Wait before retrying.",
      );
    }

    // 204 = member, 404 = not a member, 302 = requester not an org member
    isMember = memberResponse.status === 204;
  } catch (err) {
    console.error("auth-org-check: GitHub API error", err);
    return errorResponse(
      502,
      "GITHUB_API_ERROR",
      "Failed to reach GitHub API.",
      "Check network connectivity and retry.",
    );
  }

  // Cache the result in activity_log
  const cachedUntil = new Date(Date.now() + CACHE_TTL_MS).toISOString();

  const { error: logError } = await serviceClient.from("activity_log").insert({
    user_id: userId,
    action: "auth.org_check",
    entity_type: "org",
    entity_id: null,
    metadata: {
      is_member: isMember,
      org: githubOrg,
      cached_until: cachedUntil,
    },
  });

  if (logError) {
    console.error("auth-org-check: failed to cache result", logError);
    // Non-fatal: still return the result
  }

  return new Response(
    JSON.stringify({
      is_member: isMember,
      org: githubOrg,
      cached_until: cachedUntil,
    }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
