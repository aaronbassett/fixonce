import { describe, it, expect, vi, beforeEach } from "vitest";

vi.mock("node:fs/promises", () => ({
  readFile: vi.fn(),
}));

import { readFile } from "node:fs/promises";
import { detectEnvironment } from "./environment.js";

const mockReadFile = vi.mocked(readFile);

describe("detectEnvironment", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockReadFile.mockRejectedValue(new Error("ENOENT"));
  });

  it("returns empty detected_versions when no files exist", async () => {
    const result = await detectEnvironment({ project_path: "/empty" });
    expect(Object.keys(result.detected_versions)).toHaveLength(0);
    expect(result.undetected_components).toHaveLength(12);
  });

  it("detects versions from package.json dependencies", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.14.0",
            "@aspect-build/wallet-sdk": "~1.2.3",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.14.0");
    expect(result.detected_versions.wallet_sdk).toBe("1.2.3");
    expect(result.scan_sources.compact_compiler).toBe("package.json");
  });

  it("detects versions from devDependencies", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          devDependencies: {
            "@aspect-build/midnight-js": ">=2.0.0",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.midnight_js).toBe("2.0.0");
  });

  it("detects compiler version from compact.toml", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("compact.toml")) {
        return 'compiler_version = "0.14.0"';
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.14.0");
    expect(result.scan_sources.compact_compiler).toBe("compact.toml");
  });

  it("package.json takes priority over compact.toml for same component", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      const path = String(filePath);
      if (path.endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.15.0",
          },
        });
      }
      if (path.endsWith("compact.toml")) {
        return 'compiler_version = "0.14.0"';
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.detected_versions.compact_compiler).toBe("0.15.0");
  });

  it("lists undetected components", async () => {
    mockReadFile.mockImplementation(async (filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return JSON.stringify({
          dependencies: {
            "@aspect-build/compact-compiler": "^0.14.0",
          },
        });
      }
      throw new Error("ENOENT");
    });

    const result = await detectEnvironment({ project_path: "/project" });
    expect(result.undetected_components).toContain("wallet_sdk");
    expect(result.undetected_components).toContain("midnight_js");
    expect(result.undetected_components).not.toContain("compact_compiler");
    expect(result.undetected_components).toHaveLength(11);
  });
});
