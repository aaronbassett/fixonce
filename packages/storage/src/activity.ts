import type { ActivityLog, OperationType } from "@fixonce/shared";
import { createSupabaseClient } from "./client.js";

export async function appendActivity(
  data: Omit<ActivityLog, "id" | "created_at">,
): Promise<ActivityLog> {
  const supabase = createSupabaseClient();
  const { data: log, error } = await supabase
    .from("activity_log")
    .insert({
      operation: data.operation,
      memory_id: data.memory_id ?? null,
      details: data.details,
    })
    .select()
    .single();

  if (error) throw error;
  return log as ActivityLog;
}

export async function listActivity(options?: {
  operation?: OperationType;
  memory_id?: string;
  limit?: number;
  offset?: number;
  since?: string;
}): Promise<ActivityLog[]> {
  const supabase = createSupabaseClient();
  let query = supabase
    .from("activity_log")
    .select()
    .order("created_at", { ascending: false });

  if (options?.operation) query = query.eq("operation", options.operation);
  if (options?.memory_id) query = query.eq("memory_id", options.memory_id);
  if (options?.since) query = query.gte("created_at", options.since);
  if (options?.limit) query = query.limit(options.limit);
  if (options?.offset)
    query = query.range(
      options.offset,
      options.offset + (options.limit ?? 50) - 1,
    );

  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as ActivityLog[];
}
