/**
 * secret-get — GET /functions/v1/secret-get?name={name}
 *
 * Retrieves and decrypts a secret by name.
 * Decryption uses AES-256-GCM with the master key from ENCRYPTION_MASTER_KEY.
 *
 * Constitution §II: Only the secret name is logged. The decrypted value is
 * held in memory during this request only and is never written to any log.
 *
 * Requires: Authorization: Bearer <token>
 *
 * Query parameters:
 *   name  (required) — the secret name
 *
 * Response 200:
 *   { "name": "string", "value": "decrypted plaintext" }
 */
import { z } from "https://deno.land/x/zod@v3.22.4/mod.ts";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { createServiceClient } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

const nameSchema = z
  .string()
  .min(1)
  .max(255)
  .regex(
    /^[a-zA-Z0-9_.-]+$/,
    "name may only contain alphanumeric characters, underscores, hyphens, and dots",
  );

// ---------------------------------------------------------------------------
// Crypto helpers
// ---------------------------------------------------------------------------

/**
 * Copy a Uint8Array into a fresh ArrayBuffer.
 * Required because TypeScript's strict typings for SubtleCrypto require
 * `ArrayBuffer` (not `ArrayBufferLike`), and Uint8Array.from() produces
 * `Uint8Array<ArrayBufferLike>`.
 */
function toArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(
    bytes.byteOffset,
    bytes.byteOffset + bytes.byteLength,
  ) as ArrayBuffer;
}

async function importMasterKey(masterKeyB64: string): Promise<CryptoKey> {
  let keyBytes: Uint8Array;
  try {
    keyBytes = Uint8Array.from(atob(masterKeyB64), (c) => c.charCodeAt(0));
  } catch {
    throw new Error("ENCRYPTION_MASTER_KEY is not valid base64");
  }
  if (keyBytes.length !== 32) {
    throw new Error(
      `ENCRYPTION_MASTER_KEY must decode to exactly 32 bytes (got ${keyBytes.length})`,
    );
  }
  return await crypto.subtle.importKey(
    "raw",
    toArrayBuffer(keyBytes),
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"],
  );
}

/**
 * Decrypt AES-256-GCM ciphertext.
 * Supabase returns bytea columns as hex strings prefixed with \x.
 */
async function decrypt(
  key: CryptoKey,
  ciphertextHex: string,
  ivHex: string,
): Promise<string> {
  const ciphertext = hexToBytes(ciphertextHex);
  const iv = hexToBytes(ivHex);
  const plainBuffer = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv: toArrayBuffer(iv) },
    key,
    toArrayBuffer(ciphertext),
  );
  return new TextDecoder().decode(plainBuffer);
}

/**
 * Convert a Postgres bytea hex string (e.g. "\\xdeadbeef") to Uint8Array.
 * Also handles plain hex strings without the prefix.
 */
function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("\\x") ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    bytes[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  }
  return bytes;
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
      "Send a GET request with ?name=<secret-name> and a valid Authorization header.",
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

  // Parse and validate query param
  const url = new URL(req.url);
  const rawName = url.searchParams.get("name");

  if (!rawName) {
    return errorResponse(
      400,
      "MISSING_PARAMETER",
      "Query parameter 'name' is required.",
      "Append ?name=<secret-name> to the request URL.",
    );
  }

  const parsedName = nameSchema.safeParse(rawName);
  if (!parsedName.success) {
    return errorResponse(
      400,
      "INVALID_PARAMETER",
      parsedName.error.errors.map((e) => e.message).join("; "),
      "Secret names may only contain alphanumeric characters, underscores, hyphens, and dots.",
    );
  }

  const secretName = parsedName.data;

  // Load master key
  const masterKeyB64 = Deno.env.get("ENCRYPTION_MASTER_KEY");
  if (!masterKeyB64) {
    console.error("secret-get: ENCRYPTION_MASTER_KEY not set");
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error: encryption key not configured.",
      "Contact support.",
    );
  }

  let cryptoKey: CryptoKey;
  try {
    cryptoKey = await importMasterKey(masterKeyB64);
  } catch (err) {
    console.error("secret-get: master key import failed", err);
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error: invalid encryption key format.",
      "Contact support.",
    );
  }

  // Use service-role client — secrets table is service_role only
  const serviceClient = createServiceClient();

  const { data, error } = await serviceClient
    .from("secrets")
    .select("name, ciphertext, iv")
    .eq("name", secretName)
    .maybeSingle();

  if (error) {
    console.error("secret-get: select error", error);
    return errorResponse(
      500,
      "FETCH_FAILED",
      "Failed to retrieve secret.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  if (!data) {
    return errorResponse(
      404,
      "SECRET_NOT_FOUND",
      `Secret '${secretName}' not found.`,
      "Check the name and ensure the secret has been created.",
    );
  }

  const typedData = data as { name: string; ciphertext: string; iv: string };

  // Decrypt the secret value
  let plaintext: string;
  try {
    plaintext = await decrypt(cryptoKey, typedData.ciphertext, typedData.iv);
  } catch (err) {
    console.error("secret-get: decryption failed", err);
    return errorResponse(
      500,
      "DECRYPTION_FAILED",
      "Failed to decrypt secret. The master key may have changed.",
      "Ensure ENCRYPTION_MASTER_KEY matches the key used when the secret was created.",
    );
  }

  // Log access — secret name only, NEVER the value (Constitution §II)
  await serviceClient.from("activity_log").insert({
    user_id: userId,
    action: "secret.accessed",
    entity_type: "secret",
    entity_id: null,
    metadata: { name: secretName },
  });

  return new Response(
    JSON.stringify({ name: typedData.name, value: plaintext }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
