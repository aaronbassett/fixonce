// Memory type
export const MemoryType = {
  GUIDANCE: "guidance",
  ANTI_PATTERN: "anti_pattern",
} as const;
export type MemoryType = (typeof MemoryType)[keyof typeof MemoryType];

// Source type
export const SourceType = {
  CORRECTION: "correction",
  DISCOVERY: "discovery",
  INSTRUCTION: "instruction",
} as const;
export type SourceType = (typeof SourceType)[keyof typeof SourceType];

// Created by
export const CreatedBy = {
  AI: "ai",
  HUMAN: "human",
  HUMAN_MODIFIED: "human_modified",
} as const;
export type CreatedBy = (typeof CreatedBy)[keyof typeof CreatedBy];

// Feedback tag
export const FeedbackTag = {
  HELPFUL: "helpful",
  NOT_HELPFUL: "not_helpful",
  DAMAGING: "damaging",
  ACCURATE: "accurate",
  SOMEWHAT_ACCURATE: "somewhat_accurate",
  SOMEWHAT_INACCURATE: "somewhat_inaccurate",
  INACCURATE: "inaccurate",
  OUTDATED: "outdated",
} as const;
export type FeedbackTag = (typeof FeedbackTag)[keyof typeof FeedbackTag];

// Suggested action
export const SuggestedAction = {
  KEEP: "keep",
  REMOVE: "remove",
  FIX: "fix",
} as const;
export type SuggestedAction =
  (typeof SuggestedAction)[keyof typeof SuggestedAction];

// Operation type
export const OperationType = {
  QUERY: "query",
  CREATE: "create",
  UPDATE: "update",
  FEEDBACK: "feedback",
  DETECT: "detect",
} as const;
export type OperationType = (typeof OperationType)[keyof typeof OperationType];

// Search type (parameter value, not DB enum)
export const SearchType = {
  SIMPLE: "simple",
  VECTOR: "vector",
  HYBRID: "hybrid",
} as const;
export type SearchType = (typeof SearchType)[keyof typeof SearchType];

// Verbosity (parameter value, not DB enum)
export const Verbosity = {
  SMALL: "small",
  MEDIUM: "medium",
  LARGE: "large",
} as const;
export type Verbosity = (typeof Verbosity)[keyof typeof Verbosity];
