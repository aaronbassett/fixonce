import type { CreateMemoryInput, CreateMemoryResult } from "@fixonce/shared";
import { CreateMemoryInputSchema } from "@fixonce/shared";
import { logActivity } from "@fixonce/activity";
import { executeWritePipeline } from "./write/index.js";

export async function createMemory(rawInput: CreateMemoryInput): Promise<CreateMemoryResult> {
  const input = CreateMemoryInputSchema.parse(rawInput);
  const result = await executeWritePipeline(input);

  await logActivity("create", {
    status: result.status,
    memory_id: result.memory?.id,
    dedup_outcome: result.dedup_outcome,
  }, result.memory?.id);

  return result;
}
