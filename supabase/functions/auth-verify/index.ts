/**
 * auth-verify — POST /functions/v1/auth-verify
 *
 * Verifies an Ed25519 signature of a nonce and issues a JWT.
 * No authorization required — this IS the auth endpoint.
 *
 * Request body (JSON):
 *   {
 *     "public_key": "base64 Ed25519 public key",
 *     "nonce":      "hex string (from auth-nonce)",
 *     "signature":  "base64 Ed25519 signature of the nonce bytes"
 *   }
 *
 * Response 200:
 *   { "access_token": "JWT", "expires_at": "ISO timestamp (8hr)" }
 *
 * Error codes:
 *   EC-14: Unknown public key → 401 with registration guidance
 */
import { z } from "zod";
import * as jose from "jose";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { createServiceClient } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const verifyRequestSchema = z.object({
  public_key: z
    .string()
    .min(1)
    .refine(
      (v) => {
        try {
          const bytes = Uint8Array.from(atob(v), (c) => c.charCodeAt(0));
          return bytes.length === 32;
        } catch {
          return false;
        }
      },
      {
        message:
          "public_key must be a valid base64-encoded 32-byte Ed25519 key",
      },
    ),
  nonce: z
    .string()
    .regex(/^[0-9a-f]{64}$/, "nonce must be a 64-character hex string"),
  signature: z
    .string()
    .min(1)
    .refine(
      (v) => {
        try {
          const bytes = Uint8Array.from(atob(v), (c) => c.charCodeAt(0));
          // Ed25519 signatures are exactly 64 bytes.
          return bytes.length === 64;
        } catch {
          return false;
        }
      },
      {
        message:
          "signature must be a valid base64-encoded 64-byte Ed25519 signature",
      },
    ),
});

type VerifyRequest = z.infer<typeof verifyRequestSchema>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function base64ToBytes(b64: string): Uint8Array {
  return Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
}

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Verify an Ed25519 signature using the Web Crypto API.
 * The message is the raw nonce bytes (hex-decoded).
 *
 * We copy the Uint8Array into a fresh ArrayBuffer to satisfy TypeScript's
 * strict overload resolution (which requires ArrayBuffer, not ArrayBufferLike).
 */
async function verifyEd25519(
  publicKeyBytes: Uint8Array,
  signatureBytes: Uint8Array,
  message: Uint8Array,
): Promise<boolean> {
  // Copy into fresh ArrayBuffers to satisfy TypeScript's strict Buffer typing.
  const pubKeyBuffer = publicKeyBytes.buffer.slice(
    publicKeyBytes.byteOffset,
    publicKeyBytes.byteOffset + publicKeyBytes.byteLength,
  ) as ArrayBuffer;
  const sigBuffer = signatureBytes.buffer.slice(
    signatureBytes.byteOffset,
    signatureBytes.byteOffset + signatureBytes.byteLength,
  ) as ArrayBuffer;
  const msgBuffer = message.buffer.slice(
    message.byteOffset,
    message.byteOffset + message.byteLength,
  ) as ArrayBuffer;

  const cryptoKey = await crypto.subtle.importKey(
    "raw",
    pubKeyBuffer,
    { name: "Ed25519" },
    false,
    ["verify"],
  );
  return crypto.subtle.verify("Ed25519", cryptoKey, sigBuffer, msgBuffer);
}

/**
 * Sign a custom JWT using the JWT_SECRET environment variable.
 * Algorithm: HS256. Uses the Web Crypto API directly for HMAC key import
 * since jose does not expose a standalone importHMAC function.
 */
async function signJwt(
  payload: Record<string, unknown>,
  secret: string,
): Promise<string> {
  const secretBytes = new TextEncoder().encode(secret);
  // Import as a CryptoKey so jose's SignJWT can use it.
  const key = await crypto.subtle.importKey(
    "raw",
    secretBytes.buffer.slice(
      secretBytes.byteOffset,
      secretBytes.byteOffset + secretBytes.byteLength,
    ) as ArrayBuffer,
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  return new jose.SignJWT(payload)
    .setProtectedHeader({ alg: "HS256" })
    .setIssuedAt()
    .setExpirationTime("8h")
    .sign(key);
}

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

  // Validate input
  let input: VerifyRequest;
  try {
    input = validateBody(verifyRequestSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Check that public_key, nonce, and signature are correctly formatted.",
      );
    }
    throw err;
  }

  const serviceClient = createServiceClient();

  // Look up the public key in cli_keys (EC-14: unknown key → 401)
  const { data: keyRow, error: keyError } = await serviceClient
    .from("cli_keys")
    .select("id, user_id, last_used_at")
    .eq("public_key", input.public_key)
    .maybeSingle();

  if (keyError) {
    console.error("auth-verify: cli_keys lookup error", keyError);
    return errorResponse(
      500,
      "INTERNAL_ERROR",
      "Failed to look up public key.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  if (!keyRow) {
    // EC-14: unknown public key — guide user to register
    return errorResponse(
      401,
      "UNKNOWN_PUBLIC_KEY",
      "This public key is not registered with FixOnce.",
      "Register your key first using POST /functions/v1/keys-register.",
    );
  }

  const typedKeyRow = keyRow as {
    id: string;
    user_id: string;
    last_used_at: string | null;
  };

  // Verify the nonce was recently issued in activity_log (not expired, matching key)
  const { data: nonceRows, error: nonceError } = await serviceClient
    .from("activity_log")
    .select("id, metadata, created_at")
    .eq("action", "auth.nonce_issued")
    .eq("metadata->>nonce", input.nonce)
    .eq("metadata->>public_key", input.public_key)
    .limit(1);

  if (nonceError) {
    console.error("auth-verify: nonce lookup error", nonceError);
    return errorResponse(
      500,
      "INTERNAL_ERROR",
      "Failed to verify nonce.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  if (!nonceRows || nonceRows.length === 0) {
    return errorResponse(
      401,
      "INVALID_NONCE",
      "Nonce not found. Request a new nonce from /functions/v1/auth-nonce.",
      "Obtain a fresh nonce and retry the authentication flow.",
    );
  }

  const nonceRow = nonceRows[0] as {
    id: string;
    metadata: Record<string, unknown>;
    created_at: string;
  };

  // Check expiry
  const expiresAt = nonceRow.metadata["expires_at"];
  if (typeof expiresAt !== "string" || new Date(expiresAt) < new Date()) {
    return errorResponse(
      401,
      "NONCE_EXPIRED",
      "The nonce has expired. Nonces are valid for 5 minutes.",
      "Request a new nonce from /functions/v1/auth-nonce and retry.",
    );
  }

  // Verify Ed25519 signature
  let signatureValid = false;
  try {
    const publicKeyBytes = base64ToBytes(input.public_key);
    const signatureBytes = base64ToBytes(input.signature);
    const nonceBytes = hexToBytes(input.nonce);
    signatureValid = await verifyEd25519(
      publicKeyBytes,
      signatureBytes,
      nonceBytes,
    );
  } catch (err) {
    console.error("auth-verify: signature verification error", err);
    return errorResponse(
      400,
      "SIGNATURE_VERIFICATION_FAILED",
      "Failed to verify signature — the key or signature may be malformed.",
      "Ensure the signature is an Ed25519 signature of the hex nonce bytes.",
    );
  }

  if (!signatureValid) {
    return errorResponse(
      401,
      "INVALID_SIGNATURE",
      "Signature verification failed.",
      "Ensure you are signing the exact nonce bytes with the registered private key.",
    );
  }

  // Consume the nonce by deleting it from activity_log to prevent replay attacks
  await serviceClient.from("activity_log").delete().eq("id", nonceRow.id);

  // Retrieve JWT_SECRET from env
  const jwtSecret = Deno.env.get("JWT_SECRET");
  if (!jwtSecret) {
    console.error("auth-verify: JWT_SECRET env var not set");
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error.",
      "Contact support.",
    );
  }

  // Issue JWT (8 hour expiry)
  const tokenExpiresAt = new Date(Date.now() + 8 * 60 * 60 * 1000)
    .toISOString();
  let accessToken: string;
  try {
    accessToken = await signJwt(
      {
        sub: typedKeyRow.user_id,
        cli_key_id: typedKeyRow.id,
        role: "authenticated",
      },
      jwtSecret,
    );
  } catch (err) {
    console.error("auth-verify: JWT signing error", err);
    return errorResponse(
      500,
      "TOKEN_SIGN_FAILED",
      "Failed to issue access token.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Update cli_keys.last_used_at
  const { error: updateError } = await serviceClient
    .from("cli_keys")
    .update({ last_used_at: new Date().toISOString() })
    .eq("id", typedKeyRow.id);

  if (updateError) {
    // Non-fatal: log but don't fail the response
    console.error("auth-verify: failed to update last_used_at", updateError);
  }

  // Log successful auth
  await serviceClient.from("activity_log").insert({
    user_id: typedKeyRow.user_id,
    action: "auth.verified",
    entity_type: "cli_key",
    entity_id: typedKeyRow.id,
    metadata: { public_key: input.public_key },
  });

  return new Response(
    JSON.stringify({ access_token: accessToken, expires_at: tokenExpiresAt }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
