import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { readFileSync } from "node:fs";

vi.mock("node:fs", () => ({
  readFileSync: vi.fn(),
  existsSync: vi.fn(),
}));

import { existsSync } from "node:fs";
import { getConfig, resetConfig, SETTINGS_PATH } from "./config.js";

describe("getConfig", () => {
  beforeEach(() => {
    resetConfig();
    vi.unstubAllEnvs();
    vi.mocked(existsSync).mockReturnValue(false);
    vi.mocked(readFileSync).mockReturnValue("{}");
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reads from FIXONCE_ env vars", () => {
    vi.stubEnv("FIXONCE_SUPABASE_URL", "https://env.supabase.co");
    vi.stubEnv("FIXONCE_SUPABASE_ANON_KEY", "env-anon-key");
    vi.stubEnv("FIXONCE_VOYAGE_API_KEY", "env-voyage-key");
    vi.stubEnv("FIXONCE_OPENROUTER_API_KEY", "env-openrouter-key");

    const config = getConfig();
    expect(config.supabaseUrl).toBe("https://env.supabase.co");
    expect(config.supabaseAnonKey).toBe("env-anon-key");
    expect(config.voyageApiKey).toBe("env-voyage-key");
    expect(config.openrouterApiKey).toBe("env-openrouter-key");
  });

  it("reads from settings file when env vars missing", () => {
    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({
        supabaseUrl: "https://file.supabase.co",
        supabaseAnonKey: "file-anon-key",
        voyageApiKey: "file-voyage-key",
        openrouterApiKey: "file-openrouter-key",
      }),
    );

    const config = getConfig();
    expect(config.supabaseUrl).toBe("https://file.supabase.co");
    expect(config.supabaseAnonKey).toBe("file-anon-key");
    expect(config.voyageApiKey).toBe("file-voyage-key");
    expect(config.openrouterApiKey).toBe("file-openrouter-key");
  });

  it("env vars take priority over settings file", () => {
    vi.stubEnv("FIXONCE_SUPABASE_URL", "https://env.supabase.co");
    vi.stubEnv("FIXONCE_SUPABASE_ANON_KEY", "env-anon-key");
    vi.stubEnv("FIXONCE_VOYAGE_API_KEY", "env-voyage-key");
    vi.stubEnv("FIXONCE_OPENROUTER_API_KEY", "env-openrouter-key");

    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({
        supabaseUrl: "https://file.supabase.co",
        supabaseAnonKey: "file-anon-key",
        voyageApiKey: "file-voyage-key",
        openrouterApiKey: "file-openrouter-key",
      }),
    );

    const config = getConfig();
    expect(config.supabaseUrl).toBe("https://env.supabase.co");
  });

  it("throws when a required key is missing from both sources", () => {
    expect(() => getConfig()).toThrow("FIXONCE_SUPABASE_URL");
    expect(() => getConfig()).toThrow("fixonce config");
  });

  it("caches config after first call", () => {
    vi.stubEnv("FIXONCE_SUPABASE_URL", "https://env.supabase.co");
    vi.stubEnv("FIXONCE_SUPABASE_ANON_KEY", "env-anon-key");
    vi.stubEnv("FIXONCE_VOYAGE_API_KEY", "env-voyage-key");
    vi.stubEnv("FIXONCE_OPENROUTER_API_KEY", "env-openrouter-key");

    getConfig();
    getConfig();
    expect(vi.mocked(existsSync).mock.calls.length).toBeLessThanOrEqual(1);
  });

  it("SETTINGS_PATH points to ~/.config/fixonce/settings.json", () => {
    expect(SETTINGS_PATH).toContain(".config");
    expect(SETTINGS_PATH).toContain("fixonce");
    expect(SETTINGS_PATH).toContain("settings.json");
  });

  it("falls back to file settings when env var is empty string", () => {
    vi.stubEnv("FIXONCE_SUPABASE_URL", "");
    vi.stubEnv("FIXONCE_SUPABASE_ANON_KEY", "env-anon-key");
    vi.stubEnv("FIXONCE_VOYAGE_API_KEY", "env-voyage-key");
    vi.stubEnv("FIXONCE_OPENROUTER_API_KEY", "env-openrouter-key");

    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue(
      JSON.stringify({
        supabaseUrl: "https://file.supabase.co",
      }),
    );

    const config = getConfig();
    expect(config.supabaseUrl).toBe("https://file.supabase.co");
  });

  it("returns empty object for malformed JSON in settings file", () => {
    vi.stubEnv("FIXONCE_SUPABASE_URL", "https://env.supabase.co");
    vi.stubEnv("FIXONCE_SUPABASE_ANON_KEY", "env-anon-key");
    vi.stubEnv("FIXONCE_VOYAGE_API_KEY", "env-voyage-key");
    vi.stubEnv("FIXONCE_OPENROUTER_API_KEY", "env-openrouter-key");

    vi.mocked(existsSync).mockReturnValue(true);
    vi.mocked(readFileSync).mockReturnValue("{invalid json");

    const config = getConfig();
    expect(config.supabaseUrl).toBe("https://env.supabase.co");
  });
});
