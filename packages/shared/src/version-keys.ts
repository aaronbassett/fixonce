export const ComponentKey = {
  NETWORK: "network",
  NODE: "node",
  COMPACT_COMPILER: "compact_compiler",
  COMPACT_RUNTIME: "compact_runtime",
  COMPACT_JS: "compact_js",
  ONCHAIN_RUNTIME: "onchain_runtime",
  LEDGER: "ledger",
  WALLET_SDK: "wallet_sdk",
  MIDNIGHT_JS: "midnight_js",
  DAPP_CONNECTOR_API: "dapp_connector_api",
  MIDNIGHT_INDEXER: "midnight_indexer",
  PROOF_SERVER: "proof_server",
} as const;
export type ComponentKey = (typeof ComponentKey)[keyof typeof ComponentKey];

/** All valid component key values as an array (for runtime validation) */
export const COMPONENT_KEYS = Object.values(ComponentKey);

/**
 * Version predicates for filtering memories by component versions.
 * Keys are component names, values are arrays of version strings.
 * OR logic within a component, AND logic across components.
 * Missing key = no constraint on that component.
 */
export type VersionPredicates = Partial<Record<ComponentKey, string[]>>;

/**
 * Detected environment versions (single version per component).
 * Result of scanning package.json, compact.toml, etc.
 */
export type DetectedVersions = Partial<Record<ComponentKey, string>>;
