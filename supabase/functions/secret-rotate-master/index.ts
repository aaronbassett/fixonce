/**
 * secret-rotate-master — POST /functions/v1/secret-rotate-master
 *
 * Re-encrypts all secrets with a new master key. This is an admin-only
 * operation. Each secret is decrypted with the current ENCRYPTION_MASTER_KEY
 * and re-encrypted with the new_master_key provided in the request body.
 *
 * Constitution §II: Plaintext values are held in memory only during this
 * request. They are never written to any log or intermediate storage.
 *
 * Requires: Authorization: Bearer <token> (admin only)
 *
 * Request body (JSON):
 *   { "new_master_key": "base64-encoded 32-byte AES-256 key" }
 *
 * Response 200:
 *   { "rotated_count": integer, "completed_at": "ISO timestamp" }
 *
 * IMPORTANT: After a successful rotation, the caller MUST update the
 * ENCRYPTION_MASTER_KEY environment variable to the new key. Failure to do
 * so will make all secrets unreadable.
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

const rotateMasterSchema = z.object({
  new_master_key: z
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
          "new_master_key must be a valid base64-encoded 32-byte (256-bit) AES key",
      },
    ),
});

type RotateMasterInput = z.infer<typeof rotateMasterSchema>;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface SecretRow {
  id: string;
  name: string;
  ciphertext: string;
  iv: string;
}

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

async function importAesKey(
  keyB64: string,
  usages: KeyUsage[],
): Promise<CryptoKey> {
  let keyBytes: Uint8Array;
  try {
    keyBytes = Uint8Array.from(atob(keyB64), (c) => c.charCodeAt(0));
  } catch {
    throw new Error("Key is not valid base64");
  }
  if (keyBytes.length !== 32) {
    throw new Error(
      `Key must decode to exactly 32 bytes (got ${keyBytes.length})`,
    );
  }
  return await crypto.subtle.importKey(
    "raw",
    toArrayBuffer(keyBytes),
    { name: "AES-GCM", length: 256 },
    false,
    usages,
  );
}

function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith("\\x") ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < clean.length; i += 2) {
    bytes[i / 2] = parseInt(clean.slice(i, i + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return (
    "\\x" +
    Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("")
  );
}

async function decryptSecret(
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

async function encryptSecret(
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
      "Send a POST request with a JSON body containing new_master_key.",
    );
  }

  // Verify authentication (admin only check)
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
  let input: RotateMasterInput;
  try {
    input = validateBody(rotateMasterSchema, rawBody);
  } catch (err) {
    if (err instanceof ValidationError) {
      return errorResponse(
        400,
        "VALIDATION_ERROR",
        err.message,
        "Provide a valid base64-encoded 32-byte AES-256 key as new_master_key.",
      );
    }
    throw err;
  }

  // Load current master key
  const currentMasterKeyB64 = Deno.env.get("ENCRYPTION_MASTER_KEY");
  if (!currentMasterKeyB64) {
    console.error("secret-rotate-master: ENCRYPTION_MASTER_KEY not set");
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Server configuration error: current encryption key not configured.",
      "Contact support.",
    );
  }

  // Import both keys
  let oldKey: CryptoKey;
  let newKey: CryptoKey;
  try {
    oldKey = await importAesKey(currentMasterKeyB64, ["decrypt"]);
    newKey = await importAesKey(input.new_master_key, ["encrypt"]);
  } catch (err) {
    console.error("secret-rotate-master: key import failed", err);
    return errorResponse(
      500,
      "CONFIG_ERROR",
      "Failed to import encryption keys.",
      "Ensure both the current ENCRYPTION_MASTER_KEY and new_master_key are valid.",
    );
  }

  // Use service-role client — secrets table is service_role only
  const serviceClient = createServiceClient();

  // Fetch all secrets
  const { data: secrets, error: fetchError } = await serviceClient
    .from("secrets")
    .select("id, name, ciphertext, iv");

  if (fetchError) {
    console.error("secret-rotate-master: failed to fetch secrets", fetchError);
    return errorResponse(
      500,
      "FETCH_FAILED",
      "Failed to retrieve secrets for rotation.",
      "Retry the request. If the problem persists, contact support.",
    );
  }

  const secretRows = (secrets ?? []) as SecretRow[];
  let rotatedCount = 0;
  const errors: string[] = [];

  // Re-encrypt each secret (Constitution §II: plaintext only in memory)
  for (const secret of secretRows) {
    try {
      // Decrypt with old key
      const plaintext = await decryptSecret(
        oldKey,
        secret.ciphertext,
        secret.iv,
      );

      // Encrypt with new key (new random IV per secret)
      const { ciphertext, iv } = await encryptSecret(newKey, plaintext);

      // Update in database
      const { error: updateError } = await serviceClient
        .from("secrets")
        .update({
          ciphertext: bytesToHex(ciphertext),
          iv: bytesToHex(iv),
          updated_at: new Date().toISOString(),
        })
        .eq("id", secret.id);

      if (updateError) {
        console.error(
          `secret-rotate-master: failed to update secret '${secret.name}'`,
          updateError,
        );
        errors.push(secret.name);
      } else {
        rotatedCount++;
      }
    } catch (err) {
      console.error(
        `secret-rotate-master: failed to re-encrypt secret '${secret.name}'`,
        err,
      );
      errors.push(secret.name);
    }
  }

  const completedAt = new Date().toISOString();

  // Log the rotation event (name only — no values ever logged)
  await serviceClient.from("activity_log").insert({
    user_id: userId,
    action: "secret.master_key_rotated",
    entity_type: "secrets",
    entity_id: null,
    metadata: {
      rotated_count: rotatedCount,
      failed_count: errors.length,
      completed_at: completedAt,
    },
  });

  // If any secrets failed to rotate, return partial success with 207
  if (errors.length > 0) {
    return new Response(
      JSON.stringify({
        rotated_count: rotatedCount,
        failed_count: errors.length,
        failed_secrets: errors,
        completed_at: completedAt,
        warning:
          "Some secrets failed to rotate. The master key has NOT been changed for failed secrets. " +
          "ENCRYPTION_MASTER_KEY must NOT be updated until all secrets are successfully rotated.",
      }),
      {
        status: 207,
        headers: { "Content-Type": "application/json", ...corsHeaders },
      },
    );
  }

  return new Response(
    JSON.stringify({ rotated_count: rotatedCount, completed_at: completedAt }),
    {
      status: 200,
      headers: { "Content-Type": "application/json", ...corsHeaders },
    },
  );
});
