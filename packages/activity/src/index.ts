import type { OperationType } from "@fixonce/shared";
import { appendActivity } from "@fixonce/storage";

export async function logActivity(
  operation: OperationType,
  details: Record<string, unknown>,
  memoryId?: string,
): Promise<void> {
  try {
    await appendActivity({
      operation,
      memory_id: memoryId ?? null,
      details,
    });
  } catch (err) {
    // Activity logging should never break the main flow
    console.error("Failed to log activity:", err);
  }
}
