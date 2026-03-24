/**
 * secret-create — POST /functions/v1/secret-create
 *
 * Creates an encrypted secret. Encryption uses AES-256-GCM with a master key
 * loaded from the ENCRYPTION_MASTER_KEY environment variable.
 *
 * Constitution §II: secrets never touch disk — only the encrypted ciphertext
 * and IV are persisted. The plaintext value is only held in memory during
 * this request and is never logged.
 *
 * Requires: Authorization: Bearer <token>
 *           Caller must be an admin (service_role or designated admin user).
 *
 * Request body (JSON):
 *   { "name": "string", "value": "string" }
 *
 * Response 201:
 *   { "name": "string", "created_at": "ISO timestamp" }
 */
import { z } from "zod";
import { corsHeaders, handleCors } from "../_shared/cors.ts";
import { errorResponse } from "../_shared/errors.ts";
import { verifyAuth } from "../_shared/auth.ts";
import { validateBody, ValidationError } from "../_shared/validate.ts";
import { createServiceClient } from "../_shared/activity.ts";

// ---------------------------------------------------------------------------
// Input schema
// ---------------------------------------------------------------------------

const secretCreateSchema = z.object({
  name: z
    .string()
    .min(1)
    .max(255)
    .regex(
      /^[a-zA-Z0-9_.-]+$/,
      "name may only contain alphanumeric characters, underscores, hyphens, and dots",
    ),
  value: z.string().min(1),
});

type SecretCreateInput = z.infer<typeof secretCreateSchema>;

// ---------------------------------------------------------------------------
// Crypto helpers
// ---------------------------------------------------------------------------

/**
 * Derive an AES-256-GCM CryptoKey from the base64-encoded master key string.
 * The master key must be exactly 32 bytes when decoded.
 */
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
 * Encrypt plaintext with AES-256-GCM.
 * Returns { ciphertext: Uint8Array, iv: Uint8Array }.
 * The IV is 12 bytes (96 bits) as recommended for GCM.
 */
async function encrypt(
  key: CryptoKey,
  plaintext: string,
): Promise<{ ciphertext: Uint8Array; iv: Uint8Array }> {
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const encoded = new TextEncoder().encode(plaintext);
  const ciphertextBuffer = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv: toArrayBuffer(iv) },
    key,
    toArrayBuffer(encoded),
  );
  return { ciphertext: new Uint8Array(ciphertextBuffer), iv };
}

/** Convert Uint8Array to a hex string for storage in bytea columns. */
function bytesToHex(bytes: Uint8Array): string {
  return (
    "\\x" +
    Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("")
  );
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
      "Send a POST request with a JSON body containing name and value.",
    );
  }

  // Verify authentication — allow service_role key for admin bootstrap
  let userId: string;
  const authHeader = req.headers.get("Authorization");
  const token = authHeader?.replace("Bearer ", "");

  // Check if the caller is using a service_role JWT (admin bootstrap)
  let isServiceRole = false;
  if (token) {
    try {
      const payload = JSON.parse(atob(token.split(".")[1]));
      if (payload.role === "service_role") {
        isServiceRole = true;
      }
    } catch {
      // Not a valid JWT — fall through to normal auth
    }
  }

  if (isServiceRole) {
    userId = "00000000-0000-0000-0000-000000000000";
  } else {
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

  // Validate input — value is accepted but never logged
  let input: SecretCreateInput;
  try {
    input = validateBody(secretCreateSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid name (alphanumeric, _, -, .) and non-empty value.",
      );
    }
    throw err;
  }

  // Load and validate master key
  const masterKeyB64 = Deno.env.get("ENCRYPTION_MASTER_KEY");
  if (!masterKeyB64) {
    console.error("secret-create: ENCRYPTION_MASTER_KEY not set");
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
    console.error("secret-create: master key import failed", err);
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error: invalid encryption key format.",
      "Contact support.",
    );
  }

  // Encrypt the value (Constitution §II: plaintext never touches disk)
  let ciphertext: Uint8Array;
  let iv: Uint8Array;
  try {
    ({ ciphertext, iv } = await encrypt(cryptoKey, input.value));
  } catch (err) {
    console.error("secret-create: encryption failed", err);
    return errorResponse(
      500,
      "ENCRYPTION_FAILED",
      "Failed to encrypt secret value.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  // Use service-role client — secrets table is service_role only
  const serviceClient = createServiceClient();

  const { data, error } = await serviceClient
    .from("secrets")
    .insert({
      name: input.name,
      ciphertext: bytesToHex(ciphertext),
      iv: bytesToHex(iv),
      created_by: userId,
    })
    .select("name, created_at")
    .single();

  if (error) {
    // Unique constraint on name
    if (
      error.code === "23505" ||
      error.message?.toLowerCase().includes("unique")
    ) {
      return errorResponse(
        409,
        "DUPLICATE_SECRET",
        `A secret named '${input.name}' already exists.`,
        "Use a different name, or delete the existing secret first.",
      );
    }
    console.error("secret-create: insert error", error);
    return errorResponse(
      500,
      "INSERT_FAILED",
      "Failed to store secret.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  const typedData = data as { name: string; created_at: string };

  // Log access — name only, NEVER the value (Constitution §II)
  await serviceClient.from("activity_log").insert({
    user_id: userId,
    action: "secret.created",
    entity_type: "secret",
    entity_id: null,
    metadata: { name: typedData.name },
  });

  return new Response(
    JSON.stringify({ name: typedData.name, created_at: typedData.created_at }),
    {
      status: 201,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
