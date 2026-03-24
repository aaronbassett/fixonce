/**
 * auth-login-exchange — POST /functions/v1/auth-login-exchange
 *
 * Exchanges an OAuth authorization code for a JWT.  The backend performs the
 * Supabase PKCE token exchange so the CLI never needs the anon key.
 *
 * No authorization required — the code itself is the proof of authentication.
 *
 * Request body (JSON):
 *   { "code": "<auth code>", "redirect_uri": "http://127.0.0.1:<port>/callback" }
 *
 * Response 200:
 *   { "access_token": "<JWT>" }
 */
import { z } from "zod";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const loginExchangeSchema = z.object({
  code: z.string().min(1),
  redirect_uri: z
    .string()
    .min(1)
    .refine((v) => /^http:\/\/127\.0\.0\.1:\d{1,5}\/callback$/.test(v), {
      message: "redirect_uri must be http://127.0.0.1:<port>/callback",
    }),
});

type LoginExchangeRequest = z.infer<typeof loginExchangeSchema>;

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

Deno.serve(async (req: Request): Promise<Response> => {
  const corsResponse = handleCors(req);
  if (corsResponse) return corsResponse;

  if (req.method !== "POST") {
    return errorResponse(
      405,
      "METHOD_NOT_ALLOWED",
      "Only POST requests are accepted.",
      "Send a POST request with a JSON body containing code and redirect_uri.",
    );
  }

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

  let input: LoginExchangeRequest;
  try {
    input = validateBody(loginExchangeSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid code and redirect_uri.",
      );
    }
    throw err;
  }

  const supabaseUrl = Deno.env.get("SUPABASE_URL");
  const supabaseAnonKey = Deno.env.get("SUPABASE_ANON_KEY");

  if (!supabaseUrl || !supabaseAnonKey) {
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Backend auth configuration is incomplete.",
      "Contact the administrator to configure SUPABASE_URL and SUPABASE_ANON_KEY.",
    );
  }

  // Exchange the authorization code for a JWT via Supabase's PKCE endpoint.
  const tokenUrl = `${supabaseUrl}/auth/v1/token?grant_type=pkce`;

  let tokenResponse: Response;
  try {
    tokenResponse = await fetch(tokenUrl, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        apikey: supabaseAnonKey,
      },
      body: JSON.stringify({
        auth_code: input.code,
        redirect_uri: input.redirect_uri,
      }),
    });
  } catch (err) {
    console.error("auth-login-exchange: token exchange failed", err);
    return errorResponse(
      502,
      "EXCHANGE_FAILED",
      "Failed to exchange authorization code.",
      "Retry the login flow. If the problem persists, contact support.",
    );
  }

  if (!tokenResponse.ok) {
    const body = await tokenResponse.text();
    console.error(
      "auth-login-exchange: token exchange error",
      tokenResponse.status,
      body,
    );
    return errorResponse(
      502,
      "EXCHANGE_REJECTED",
      "Authorization code exchange was rejected.",
      "The code may have expired. Run `fixonce login` again.",
    );
  }

  const tokenPayload = await tokenResponse.json();
  const accessToken = tokenPayload?.access_token;

  if (!accessToken) {
    console.error(
      "auth-login-exchange: no access_token in response, keys:",
      Object.keys(tokenPayload ?? {}),
    );
    return errorResponse(
      502,
      "MISSING_TOKEN",
      "Token exchange succeeded but no access token was returned.",
      "Retry the login flow. If the problem persists, contact support.",
    );
  }

  return new Response(
    JSON.stringify({ access_token: accessToken }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
