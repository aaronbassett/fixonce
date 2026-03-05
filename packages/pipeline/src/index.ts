// Service layer
export {
  createMemory,
  queryMemories,
  expandCacheKey,
  getMemory,
  updateMemory,
  submitFeedback,
  detectEnvironment,
} from "./service.js";

// LLM utilities
export { llmCall, llmCallJSON, resetLLMClient } from "./llm.js";

// Write pipeline
export { evaluateQuality } from "./write/quality-gate.js";
export { detectDuplicates } from "./write/duplicate-detection.js";
export { checkForCredentials } from "./write/credential-check.js";
export { executeWritePipeline } from "./write/index.js";

// Read pipeline
export { executeReadPipeline } from "./read/index.js";
export { rewriteQuery } from "./read/query-rewriter.js";
export { rerankResults } from "./read/reranker.js";
export type { RankedMemory } from "./read/reranker.js";
export { generateCacheKey, lookupCacheKey, clearExpiredKeys } from "./read/cache.js";

// Environment detection
export { detectEnvironment as detectEnvironmentDirect } from "./environment.js";
