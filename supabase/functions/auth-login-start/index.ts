/**
 * auth-login-start — POST /functions/v1/auth-login-start
 *
 * Returns the OAuth authorization URL so the CLI can open the user's browser
 * without knowing Supabase auth details directly.
 *
 * No authorization required — this is the first step of the browser login flow.
 *
 * Request body (JSON):
 *   { "redirect_uri": "http://127.0.0.1:<port>/callback" }
 *
 * Response 200:
 *   { "auth_url": "https://<project>.supabase.co/auth/v1/authorize?..." }
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const loginStartSchema = z.object({
  redirect_uri: z
    .string()
    .min(1)
    .refine((v) => /^http:\/\/127\.0\.0\.1:\d{1,5}\/callback$/.test(v), {
      message: "redirect_uri must be http://127.0.0.1:<port>/callback",
    }),
});

type LoginStartRequest = z.infer<typeof loginStartSchema>;

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
      "Send a POST request with a JSON body containing redirect_uri.",
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

  let input: LoginStartRequest;
  try {
    input = validateBody(loginStartSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid redirect_uri pointing to http://127.0.0.1:<port>/callback.",
      );
    }
    throw err;
  }

  const supabaseUrl = Deno.env.get("SUPABASE_URL");
  if (!supabaseUrl) {
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "SUPABASE_URL is not configured.",
      "Contact the administrator to configure the backend.",
    );
  }

  const authUrl =
    `${supabaseUrl}/auth/v1/authorize?provider=github&redirect_to=${encodeURIComponent(input.redirect_uri)}`;

  return new Response(
    JSON.stringify({ auth_url: authUrl }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
