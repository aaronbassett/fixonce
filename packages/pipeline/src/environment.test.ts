import { describe, it, expect, beforeEach, vi } from "vitest";
import { readFile } from "node:fs/promises";

vi.mock("node:fs/promises", () => ({
  readFile: vi.fn(),
}));

import { detectEnvironment } from "./environment.js";

const mockReadFile = vi.mocked(readFile);

function makePackageJson(
  deps?: Record<string, string>,
  devDeps?: Record<string, string>,
): string {
  const pkg: Record<string, unknown> = {};
  if (deps) pkg.dependencies = deps;
  if (devDeps) pkg.devDependencies = devDeps;
  return JSON.stringify(pkg);
}

describe("detectEnvironment", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    mockReadFile.mockRejectedValue(new Error("ENOENT"));
  });

  it("returns empty detected_versions when no files exist", async () => {
    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions).toEqual({});
    expect(result.scan_sources).toEqual({});
    expect(result.undetected_components.length).toBeGreaterThan(0);
  });

  it("detects versions from package.json dependencies", async () => {
    mockReadFile.mockImplementation((filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return Promise.resolve(
          makePackageJson({
            "@aspect-build/midnight-js": "^1.2.3",
            "@aspect-build/wallet-sdk": "~2.0.0",
          }),
        );
      }
      return Promise.reject(new Error("ENOENT"));
    });

    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions.midnight_js).toBe("1.2.3");
    expect(result.detected_versions.wallet_sdk).toBe("2.0.0");
    expect(result.scan_sources.midnight_js).toBe("package.json");
    expect(result.scan_sources.wallet_sdk).toBe("package.json");
  });

  it("detects versions from devDependencies", async () => {
    mockReadFile.mockImplementation((filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return Promise.resolve(
          makePackageJson(undefined, {
            "@aspect-build/compact-js": ">=3.0.0",
          }),
        );
      }
      return Promise.reject(new Error("ENOENT"));
    });

    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions.compact_js).toBe("3.0.0");
    expect(result.scan_sources.compact_js).toBe("package.json");
  });

  it("detects compiler version from compact.toml", async () => {
    mockReadFile.mockImplementation((filePath) => {
      if (String(filePath).endsWith("compact.toml")) {
        return Promise.resolve('compiler_version = "0.14.0"');
      }
      return Promise.reject(new Error("ENOENT"));
    });

    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions.compact_compiler).toBe("0.14.0");
    expect(result.scan_sources.compact_compiler).toBe("compact.toml");
  });

  it("package.json takes priority over compact.toml for same component", async () => {
    mockReadFile.mockImplementation((filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return Promise.resolve(
          makePackageJson({
            "@aspect-build/compact-compiler": "^0.15.0",
          }),
        );
      }
      if (String(filePath).endsWith("compact.toml")) {
        return Promise.resolve('compiler_version = "0.14.0"');
      }
      return Promise.reject(new Error("ENOENT"));
    });

    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions.compact_compiler).toBe("0.15.0");
    expect(result.scan_sources.compact_compiler).toBe("package.json");
  });

  it("lists undetected components", async () => {
    mockReadFile.mockImplementation((filePath) => {
      if (String(filePath).endsWith("package.json")) {
        return Promise.resolve(
          makePackageJson({
            "@aspect-build/ledger": "1.0.0",
          }),
        );
      }
      return Promise.reject(new Error("ENOENT"));
    });

    const result = await detectEnvironment({ project_path: "/fake" });

    expect(result.detected_versions.ledger).toBe("1.0.0");
    expect(result.undetected_components).not.toContain("ledger");
    expect(result.undetected_components).toContain("network");
    expect(result.undetected_components).toContain("midnight_js");
    expect(result.undetected_components).toContain("compact_compiler");
  });
});
