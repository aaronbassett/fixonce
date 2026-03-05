import { createClient, SupabaseClient } from "@supabase/supabase-js";

let client: SupabaseClient | null = null;

export function createSupabaseClient(): SupabaseClient {
  if (client) return client;

  const url = process.env.SUPABASE_URL;
  const key = process.env.SUPABASE_ANON_KEY;

  if (!url) {
    throw new Error(
      "SUPABASE_URL is not set. Add it to your .env file. " +
        "Get it from your Supabase project settings.",
    );
  }

  if (!key) {
    throw new Error(
      "SUPABASE_ANON_KEY is not set. Add it to your .env file. " +
        "Get it from your Supabase project settings.",
    );
  }

  client = createClient(url, key);
  return client;
}

export function resetClient(): void {
  client = null;
}
