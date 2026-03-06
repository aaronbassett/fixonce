import type {
  DetectEnvironmentInput,
  DetectEnvironmentResult,
  ComponentKey,
  DetectedVersions,
} from "@fixonce/shared";
import { COMPONENT_KEYS } from "@fixonce/shared";
import { readFile } from "node:fs/promises";
import { join } from "node:path";

/**
 * Maps npm package names to their corresponding component keys.
 */
const PACKAGE_MAP: Record<string, ComponentKey> = {
  "@aspect-build/midnight-js": "midnight_js",
  "@aspect-build/compact-compiler": "compact_compiler",
  "@aspect-build/compact-runtime": "compact_runtime",
  "@aspect-build/wallet-sdk": "wallet_sdk",
  "@aspect-build/dapp-connector-api": "dapp_connector_api",
  "@aspect-build/compact-js": "compact_js",
  "@aspect-build/onchain-runtime": "onchain_runtime",
  "@aspect-build/ledger": "ledger",
  "@aspect-build/midnight-indexer": "midnight_indexer",
  "@aspect-build/proof-server": "proof_server",
  "@aspect-build/midnight-node": "node",
  "@aspect-build/midnight-network": "network",
};

async function readJsonFile(
  filePath: string,
): Promise<Record<string, unknown> | null> {
  try {
    const raw = await readFile(filePath, "utf-8");
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return null;
  }
}

async function readTextFile(filePath: string): Promise<string | null> {
  try {
    return await readFile(filePath, "utf-8");
  } catch {
    return null;
  }
}

function toDependencyRecord(
  value: unknown,
): Record<string, string> | undefined {
  if (value === null || typeof value !== "object" || Array.isArray(value))
    return undefined;
  const obj = value as Record<string, unknown>;
  const result: Record<string, string> = {};
  for (const [k, v] of Object.entries(obj)) {
    if (typeof v === "string") result[k] = v;
  }
  return Object.keys(result).length > 0 ? result : undefined;
}

function scanDependencies(
  deps: Record<string, string> | undefined,
  detected: DetectedVersions,
  scanSources: Partial<Record<ComponentKey, string>>,
  sourceName: string,
): void {
  if (!deps) return;

  for (const [pkg, version] of Object.entries(deps)) {
    const componentKey = PACKAGE_MAP[pkg];
    if (componentKey && !detected[componentKey]) {
      // Strip common version prefixes (^, ~, >=, etc.)
      detected[componentKey] = version.replace(/^[\^~>=<]+/, "");
      scanSources[componentKey] = sourceName;
    }
  }
}

function parseCompactTomlVersion(content: string): string | null {
  // Look for a version line like: compiler_version = "0.14.0" or version = "0.14.0"
  const match = content.match(/(?:compiler_version|version)\s*=\s*"([^"]+)"/);
  return match?.[1] ?? null;
}

export async function detectEnvironment(
  input: DetectEnvironmentInput,
): Promise<DetectEnvironmentResult> {
  const projectPath = input.project_path ?? process.cwd();
  const detected: DetectedVersions = {};
  const scanSources: Partial<Record<ComponentKey, string>> = {};

  // Scan package.json
  const packageJsonPath = join(projectPath, "package.json");
  const packageJson = await readJsonFile(packageJsonPath);

  if (packageJson) {
    scanDependencies(
      toDependencyRecord(packageJson.dependencies),
      detected,
      scanSources,
      "package.json",
    );
    scanDependencies(
      toDependencyRecord(packageJson.devDependencies),
      detected,
      scanSources,
      "package.json",
    );
  }

  // Scan compact.toml for compiler version
  const compactTomlPath = join(projectPath, "compact.toml");
  const compactToml = await readTextFile(compactTomlPath);

  if (compactToml) {
    const compilerVersion = parseCompactTomlVersion(compactToml);
    if (compilerVersion && !detected.compact_compiler) {
      detected.compact_compiler = compilerVersion;
      scanSources.compact_compiler = "compact.toml";
    }
  }

  // Determine undetected components
  const undetectedComponents = COMPONENT_KEYS.filter(
    (key) => !(key in detected),
  );

  return {
    detected_versions: detected,
    scan_sources: scanSources,
    undetected_components: undetectedComponents,
  };
}
