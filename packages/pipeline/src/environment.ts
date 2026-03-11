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
  // Compact
  "@midnight-ntwrk/compact-runtime": "compact_runtime",
  "@midnight-ntwrk/compact-js": "compact_js",
  "@midnight-ntwrk/compact-js-command": "compact_js",
  "@midnight-ntwrk/compact-js-node": "compact_js",

  // Midnight JS
  "@midnight-ntwrk/midnight-js-compact": "midnight_js",
  "@midnight-ntwrk/midnight-js-contracts": "midnight_js",
  "@midnight-ntwrk/midnight-js-types": "midnight_js",
  "@midnight-ntwrk/midnight-js-utils": "midnight_js",
  "@midnight-ntwrk/midnight-js-network-id": "midnight_js",
  "@midnight-ntwrk/midnight-js-testing": "midnight_js",
  "@midnight-ntwrk/midnight-js-logger-provider": "midnight_js",
  "@midnight-ntwrk/midnight-js-fetch-zk-config-provider": "midnight_js",
  "@midnight-ntwrk/midnight-js-node-zk-config-provider": "midnight_js",
  "@midnight-ntwrk/midnight-js-level-private-state-provider": "midnight_js",
  "@midnight-ntwrk/platform-js": "midnight_js",
  "@midnight-ntwrk/testkit-js": "midnight_js",

  // DApp Connector
  "@midnight-ntwrk/dapp-connector-api": "dapp_connector_api",

  // Ledger
  "@midnight-ntwrk/ledger": "ledger",
  "@midnight-ntwrk/ledger-v6": "ledger",
  "@midnight-ntwrk/ledger-v7": "ledger",
  "@midnight-ntwrk/ledger-v8": "ledger",

  // Onchain Runtime
  "@midnight-ntwrk/onchain-runtime": "onchain_runtime",
  "@midnight-ntwrk/onchain-runtime-v1": "onchain_runtime",
  "@midnight-ntwrk/onchain-runtime-v2": "onchain_runtime",
  "@midnight-ntwrk/onchain-runtime-v3": "onchain_runtime",

  // Wallet SDK
  "@midnight-ntwrk/wallet": "wallet_sdk",
  "@midnight-ntwrk/wallet-api": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-abstractions": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-address-format": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-capabilities": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-dust-wallet": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-facade": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-hd": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-runtime": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-shielded": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-unshielded-state": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-unshielded-wallet": "wallet_sdk",
  "@midnight-ntwrk/wallet-sdk-utilities": "wallet_sdk",

  // Indexer
  "@midnight-ntwrk/midnight-js-indexer-public-data-provider":
    "midnight_indexer",
  "@midnight-ntwrk/wallet-sdk-indexer-client": "midnight_indexer",

  // Proof Server
  "@midnight-ntwrk/midnight-js-http-client-proof-provider": "proof_server",
  "@midnight-ntwrk/wallet-sdk-prover-client": "proof_server",

  // Node
  "@midnight-ntwrk/wallet-sdk-node-client": "node",

  // ZK primitives
  "@midnight-ntwrk/zkir-v2": "compact_runtime",
  "@midnight-ntwrk/zswap": "compact_runtime",
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
