import type { Memory } from "@fixonce/shared";
import { createSupabaseClient } from "./client.js";

export async function createMemory(
  data: Omit<
    Memory,
    | "id"
    | "created_at"
    | "updated_at"
    | "surfaced_count"
    | "last_surfaced_at"
    | "embedding"
    | "enabled"
  > & { enabled?: boolean },
): Promise<Memory> {
  const supabase = createSupabaseClient();
  const { data: memory, error } = await supabase
    .from("memory")
    .insert({
      title: data.title,
      content: data.content,
      summary: data.summary,
      memory_type: data.memory_type,
      source_type: data.source_type,
      created_by: data.created_by,
      source_url: data.source_url ?? null,
      tags: data.tags ?? [],
      language: data.language,
      version_predicates: data.version_predicates ?? null,
      project_name: data.project_name ?? null,
      project_repo_url: data.project_repo_url ?? null,
      project_workspace_path: data.project_workspace_path ?? null,
      confidence: data.confidence ?? 0.5,
      enabled: data.enabled ?? true,
    })
    .select()
    .single();

  if (error) throw error;
  return memory as Memory;
}

export async function getMemoryById(id: string): Promise<Memory | null> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase
    .from("memory")
    .select()
    .eq("id", id)
    .single();

  if (error) {
    if (error.code === "PGRST116") return null;
    throw error;
  }
  return data as Memory;
}

export async function updateMemory(
  id: string,
  updates: Partial<
    Omit<
      Memory,
      "id" | "created_at" | "updated_at" | "surfaced_count" | "last_surfaced_at"
    >
  >,
): Promise<Memory> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase
    .from("memory")
    .update(updates)
    .eq("id", id)
    .select()
    .single();

  if (error) throw error;
  return data as Memory;
}

export async function deleteMemory(id: string): Promise<void> {
  const supabase = createSupabaseClient();
  const { error } = await supabase.from("memory").delete().eq("id", id);

  if (error) throw error;
}

export async function listEnabledMemories(options?: {
  language?: string;
  memory_type?: string;
  tags?: string[];
  limit?: number;
  offset?: number;
}): Promise<Memory[]> {
  const supabase = createSupabaseClient();
  let query = supabase
    .from("memory")
    .select()
    .eq("enabled", true)
    .order("updated_at", { ascending: false });

  if (options?.language) query = query.eq("language", options.language);
  if (options?.memory_type) query = query.eq("memory_type", options.memory_type);
  if (options?.tags?.length) query = query.contains("tags", options.tags);
  if (options?.limit) query = query.limit(options.limit);
  if (options?.offset)
    query = query.range(
      options.offset,
      options.offset + (options.limit ?? 50) - 1,
    );

  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as Memory[];
}

export async function incrementSurfacedCount(ids: string[]): Promise<void> {
  if (ids.length === 0) return;
  const supabase = createSupabaseClient();
  for (const id of ids) {
    const { error } = await supabase.rpc("increment_surfaced_count", {
      memory_id: id,
    });
    if (error) {
      const { data } = await supabase
        .from("memory")
        .select("surfaced_count")
        .eq("id", id)
        .single();
      if (data) {
        await supabase
          .from("memory")
          .update({
            surfaced_count: (data.surfaced_count ?? 0) + 1,
            last_surfaced_at: new Date().toISOString(),
          })
          .eq("id", id);
      }
    }
  }
}
