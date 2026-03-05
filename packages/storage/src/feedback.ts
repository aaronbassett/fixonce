import type { Feedback } from "@fixonce/shared";
import { createSupabaseClient } from "./client.js";

export async function createFeedback(
  data: Omit<Feedback, "id" | "created_at">,
): Promise<Feedback> {
  const supabase = createSupabaseClient();
  const { data: feedback, error } = await supabase
    .from("feedback")
    .insert({
      memory_id: data.memory_id,
      text: data.text ?? null,
      tags: data.tags ?? [],
      suggested_action: data.suggested_action ?? null,
    })
    .select()
    .single();

  if (error) throw error;
  return feedback as Feedback;
}

export async function listFeedbackByMemoryId(
  memoryId: string,
): Promise<Feedback[]> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase
    .from("feedback")
    .select()
    .eq("memory_id", memoryId)
    .order("created_at", { ascending: false });

  if (error) throw error;
  return (data ?? []) as Feedback[];
}

export async function listFlaggedFeedback(): Promise<Feedback[]> {
  const supabase = createSupabaseClient();
  const { data, error } = await supabase
    .from("feedback")
    .select()
    .in("suggested_action", ["remove", "fix"])
    .order("created_at", { ascending: false });

  if (error) throw error;
  return (data ?? []) as Feedback[];
}
