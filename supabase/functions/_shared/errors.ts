/**
 * Structured error response builder.
 *
 * Per Constitution §VI: all API errors use the format:
 *   { "error": { "type": string, "message": string, "action": string } }
 */
import { corsHeaders } from "./cors.ts";

export function errorResponse(
  status: number,
  type: string,
  message: string,
  action: string,
): Response {
  return new Response(
    JSON.stringify({ error: { type, message, action } }),
    {
      status,
      headers: {
        "Content-Type": "application/json",
        ...corsHeaders,
      },
    },
  );
}
