import { z } from "zod";

// Enum schemas
export const MemoryTypeSchema = z.enum(["guidance", "anti_pattern"]);
export const SourceTypeSchema = z.enum(["correction", "discovery", "instruction"]);
export const CreatedBySchema = z.enum(["ai", "human", "human_modified"]);
export const CreatedByInputSchema = z.enum(["ai", "human"]);
export const FeedbackTagSchema = z.enum([
  "helpful", "not_helpful", "damaging", "accurate",
  "somewhat_accurate", "somewhat_inaccurate", "inaccurate", "outdated",
]);
export const SuggestedActionSchema = z.enum(["keep", "remove", "fix"]);
export const OperationTypeSchema = z.enum(["query", "create", "update", "feedback", "detect"]);
export const SearchTypeSchema = z.enum(["simple", "vector", "hybrid"]);
export const VerbositySchema = z.enum(["small", "medium", "large"]);

// Component key validation
export const ComponentKeySchema = z.enum([
  "network", "node", "compact_compiler", "compact_runtime", "compact_js",
  "onchain_runtime", "ledger", "wallet_sdk", "midnight_js",
  "dapp_connector_api", "midnight_indexer", "proof_server",
]);

// Version predicates schema
export const VersionPredicatesSchema = z.record(
  ComponentKeySchema,
  z.array(z.string()),
).optional().nullable();

// Detected versions schema
export const DetectedVersionsSchema = z.record(
  ComponentKeySchema,
  z.string(),
).optional();

// Service input schemas
export const CreateMemoryInputSchema = z.object({
  title: z.string().min(1).max(500),
  content: z.string().min(1).max(51200),
  summary: z.string().min(1).max(1000),
  memory_type: MemoryTypeSchema,
  source_type: SourceTypeSchema,
  created_by: CreatedByInputSchema,
  language: z.string().min(1),
  tags: z.array(z.string().max(100)).max(20).optional().default([]),
  source_url: z.string().url().optional().nullable(),
  version_predicates: VersionPredicatesSchema,
  project_name: z.string().optional().nullable(),
  project_repo_url: z.string().optional().nullable(),
  project_workspace_path: z.string().optional().nullable(),
  confidence: z.number().min(0).max(1).optional().default(0.5),
});

export const QueryMemoriesInputSchema = z.object({
  query: z.string().min(1),
  rewrite: z.boolean().optional().default(true),
  type: SearchTypeSchema.optional().default("hybrid"),
  rerank: z.boolean().optional().default(true),
  tags: z.array(z.string()).optional(),
  language: z.string().optional(),
  project_name: z.string().optional(),
  memory_type: MemoryTypeSchema.optional(),
  created_after: z.string().datetime().optional(),
  updated_after: z.string().datetime().optional(),
  max_results: z.number().int().min(1).max(50).optional().default(5),
  max_tokens: z.number().int().positive().optional(),
  verbosity: VerbositySchema.optional().default("small"),
  version_predicates: DetectedVersionsSchema,
});

export const ExpandCacheKeyInputSchema = z.object({
  cache_key: z.string().min(1),
  verbosity: VerbositySchema.optional().default("small"),
});

export const GetMemoryInputSchema = z.object({
  id: z.string().uuid(),
  verbosity: VerbositySchema.optional().default("large"),
});

export const UpdateMemoryInputSchema = z.object({
  id: z.string().uuid(),
  title: z.string().min(1).max(500).optional(),
  content: z.string().min(1).max(51200).optional(),
  summary: z.string().min(1).max(1000).optional(),
  memory_type: MemoryTypeSchema.optional(),
  source_type: SourceTypeSchema.optional(),
  source_url: z.string().url().nullable().optional(),
  tags: z.array(z.string().max(100)).max(20).optional(),
  language: z.string().min(1).optional(),
  version_predicates: VersionPredicatesSchema,
  project_name: z.string().nullable().optional(),
  project_repo_url: z.string().nullable().optional(),
  project_workspace_path: z.string().nullable().optional(),
  confidence: z.number().min(0).max(1).optional(),
  enabled: z.boolean().optional(),
});

export const SubmitFeedbackInputSchema = z.object({
  memory_id: z.string().uuid(),
  text: z.string().optional().nullable(),
  tags: z.array(FeedbackTagSchema).optional().default([]),
  suggested_action: SuggestedActionSchema.optional().nullable(),
});

export const DetectEnvironmentInputSchema = z.object({
  project_path: z.string().optional(),
});
