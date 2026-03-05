import { readFileSync, existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

export interface FixOnceConfig {
  supabaseUrl: string;
  supabaseAnonKey: string;
  voyageApiKey: string;
  openrouterApiKey: string;
}

export const SETTINGS_DIR = join(homedir(), ".config", "fixonce");
export const SETTINGS_PATH = join(SETTINGS_DIR, "settings.json");

const CONFIG_KEYS: Array<{
  envVar: string;
  jsonKey: keyof FixOnceConfig;
  label: string;
}> = [
  { envVar: "FIXONCE_SUPABASE_URL", jsonKey: "supabaseUrl", label: "Supabase project URL" },
  { envVar: "FIXONCE_SUPABASE_ANON_KEY", jsonKey: "supabaseAnonKey", label: "Supabase anonymous key" },
  { envVar: "FIXONCE_VOYAGE_API_KEY", jsonKey: "voyageApiKey", label: "Voyage AI API key" },
  { envVar: "FIXONCE_OPENROUTER_API_KEY", jsonKey: "openrouterApiKey", label: "OpenRouter API key" },
];

export const SETTINGS_TEMPLATE: Record<string, string> = {
  supabaseUrl: "",
  supabaseAnonKey: "",
  voyageApiKey: "",
  openrouterApiKey: "",
};

let cached: FixOnceConfig | null = null;

function loadSettingsFile(): Record<string, string> {
  if (!existsSync(SETTINGS_PATH)) return {};
  try {
    const raw = readFileSync(SETTINGS_PATH, "utf-8");
    return JSON.parse(raw) as Record<string, string>;
  } catch {
    return {};
  }
}

export function getConfig(): FixOnceConfig {
  if (cached) return cached;

  const fileSettings = loadSettingsFile();
  const config: Partial<FixOnceConfig> = {};

  for (const { envVar, jsonKey, label } of CONFIG_KEYS) {
    const value = process.env[envVar] || fileSettings[jsonKey];
    if (!value) {
      throw new Error(
        `${envVar} is not set. Run "fixonce config" to create a settings file, or export ${envVar} in your shell. ` +
          `This is your ${label}.`,
      );
    }
    config[jsonKey] = value;
  }

  cached = config as FixOnceConfig;
  return cached;
}

export function resetConfig(): void {
  cached = null;
}
