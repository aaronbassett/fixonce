import { createClient, SupabaseClient } from "@supabase/supabase-js";
import { getConfig } from "@fixonce/shared";

let client: SupabaseClient | null = null;

export function createSupabaseClient(): SupabaseClient {
  if (client) return client;

  const { supabaseUrl, supabaseAnonKey } = getConfig();
  client = createClient(supabaseUrl, supabaseAnonKey);
  return client;
}

export function resetClient(): void {
  client = null;
}
