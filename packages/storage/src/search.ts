import type { Memory } from "@fixonce/shared";
import { createSupabaseClient } from "./client.js";

export interface SearchOptions {
  query_text?: string;
  query_embedding?: number[];
  match_count?: number;
  language?: string;
  memory_type?: string;
  tags?: string[];
  created_after?: string;
  updated_after?: string;
  project_name?: string;
}

export async function hybridSearch(
  options: SearchOptions & { query_text: string; query_embedding: number[] },
): Promise<Memory[]> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase.rpc("hybrid_search", {
    query_text: options.query_text,
    query_embedding: options.query_embedding,
    match_count: options.match_count ?? 20,
  });

  if (error) throw error;
  let results = (data ?? []) as Memory[];

  if (options.language)
    results = results.filter((m) => m.language === options.language);
  if (options.memory_type)
    results = results.filter((m) => m.memory_type === options.memory_type);
  if (options.tags?.length) {
    const { tags } = options;
    results = results.filter((m) => tags.every((t) => m.tags.includes(t)));
  }
  if (options.created_after) {
    const { created_after } = options;
    results = results.filter((m) => m.created_at >= created_after);
  }
  if (options.updated_after) {
    const { updated_after } = options;
    results = results.filter((m) => m.updated_at >= updated_after);
  }
  if (options.project_name)
    results = results.filter((m) => m.project_name === options.project_name);

  return results;
}

export async function ftsSearch(
  options: SearchOptions & { query_text: string },
): Promise<Memory[]> {
  const supabase = createSupabaseClient();
  let query = supabase
    .from("memory")
    .select()
    .eq("enabled", true)
    .textSearch("fts", options.query_text, { type: "websearch" })
    .limit(options.match_count ?? 20);

  if (options.language) query = query.eq("language", options.language);
  if (options.memory_type) query = query.eq("memory_type", options.memory_type);
  if (options.tags?.length) query = query.contains("tags", options.tags);
  if (options.created_after)
    query = query.gte("created_at", options.created_after);
  if (options.updated_after)
    query = query.gte("updated_at", options.updated_after);
  if (options.project_name)
    query = query.eq("project_name", options.project_name);

  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as Memory[];
}

export async function vectorSearch(
  options: SearchOptions & { query_embedding: number[] },
): Promise<Memory[]> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase.rpc("vector_search", {
    query_embedding: options.query_embedding,
    match_count: options.match_count ?? 20,
  });

  if (error) throw error;

  let results = (data ?? []) as Memory[];
  if (options.language)
    results = results.filter((m) => m.language === options.language);
  if (options.memory_type)
    results = results.filter((m) => m.memory_type === options.memory_type);
  if (options.project_name)
    results = results.filter((m) => m.project_name === options.project_name);

  return results;
}

export async function metadataSearch(
  options: Omit<SearchOptions, "query_text" | "query_embedding">,
): Promise<Memory[]> {
  const supabase = createSupabaseClient();
  let query = supabase
    .from("memory")
    .select()
    .eq("enabled", true)
    .order("updated_at", { ascending: false })
    .limit(options.match_count ?? 20);

  if (options.language) query = query.eq("language", options.language);
  if (options.memory_type) query = query.eq("memory_type", options.memory_type);
  if (options.tags?.length) query = query.contains("tags", options.tags);
  if (options.created_after)
    query = query.gte("created_at", options.created_after);
  if (options.updated_after)
    query = query.gte("updated_at", options.updated_after);
  if (options.project_name)
    query = query.eq("project_name", options.project_name);

  const { data, error } = await query;
  if (error) throw error;
  return (data ?? []) as Memory[];
}
