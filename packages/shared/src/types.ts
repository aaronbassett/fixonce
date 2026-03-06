import type {
  MemoryType,
  SourceType,
  CreatedBy,
  FeedbackTag,
  SuggestedAction,
  OperationType,
  SearchType,
  Verbosity,
} from "./enums.js";
import type {
  VersionPredicates,
  DetectedVersions,
  ComponentKey,
} from "./version-keys.js";

// ============= Core Entities =============

export interface Memory {
  id: string;
  title: string;
  content: string;
  summary: string;
  memory_type: MemoryType;
  source_type: SourceType;
  created_by: CreatedBy;
  source_url: string | null;
  tags: string[];
  language: string;
  version_predicates: VersionPredicates | null;
  project_name: string | null;
  project_repo_url: string | null;
  project_workspace_path: string | null;
  confidence: number;
  surfaced_count: number;
  last_surfaced_at: string | null;
  enabled: boolean;
  created_at: string;
  updated_at: string;
  embedding: number[] | null;
}

export interface Feedback {
  id: string;
  memory_id: string;
  text: string | null;
  tags: FeedbackTag[];
  suggested_action: SuggestedAction | null;
  created_at: string;
}

export interface ActivityLog {
  id: string;
  operation: OperationType;
  memory_id: string | null;
  details: Record<string, unknown>;
  created_at: string;
}

// ============= Verbosity Projections =============

export type MemorySmall = Pick<
  Memory,
  "id" | "title" | "content" | "summary" | "memory_type"
> & { relevancy_score: number };

export type MemoryMedium = MemorySmall &
  Pick<
    Memory,
    | "tags"
    | "language"
    | "version_predicates"
    | "created_by"
    | "source_type"
    | "created_at"
    | "updated_at"
  >;

export type MemoryLarge = MemoryMedium &
  Pick<
    Memory,
    | "source_url"
    | "project_name"
    | "project_repo_url"
    | "project_workspace_path"
    | "confidence"
    | "surfaced_count"
    | "last_surfaced_at"
  > & { feedback_summary: FeedbackSummary };

export interface FeedbackSummary {
  total_count: number;
  tag_counts: Partial<Record<FeedbackTag, number>>;
  flagged_actions: SuggestedAction[];
}

export interface OverflowEntry {
  id: string;
  title: string;
  summary: string;
  relevancy_score: number;
  cache_key: string;
}

// ============= Service Input/Output Types =============

export interface CreateMemoryInput {
  title: string;
  content: string;
  summary: string;
  memory_type: MemoryType;
  source_type: SourceType;
  created_by: "ai" | "human";
  language: string;
  tags?: string[];
  source_url?: string | null;
  version_predicates?: VersionPredicates | null;
  project_name?: string | null;
  project_repo_url?: string | null;
  project_workspace_path?: string | null;
  confidence?: number;
}

export interface CreateMemoryResult {
  status:
    | "created"
    | "replaced"
    | "updated"
    | "merged"
    | "rejected"
    | "discarded";
  memory?: Pick<Memory, "id" | "title" | "created_at">;
  dedup_outcome?: "new" | "discard" | "replace" | "update" | "merge";
  affected_memory_ids?: string[];
  reason?: string;
  existing_memory_id?: string;
}

export interface QueryMemoriesInput {
  query: string;
  rewrite?: boolean;
  type?: SearchType;
  rerank?: boolean;
  tags?: string[];
  language?: string;
  project_name?: string;
  memory_type?: MemoryType;
  created_after?: string;
  updated_after?: string;
  max_results?: number;
  max_tokens?: number;
  verbosity?: Verbosity;
  version_predicates?: DetectedVersions;
}

export interface QueryMemoriesResult {
  results: Array<MemorySmall | MemoryMedium | MemoryLarge>;
  overflow: OverflowEntry[];
  total_found: number;
  pipeline: {
    rewrite_used: boolean;
    search_type: SearchType;
    rerank_used: boolean;
  };
}

export interface ExpandCacheKeyInput {
  cache_key: string;
  verbosity?: Verbosity;
}

export interface ExpandCacheKeyResult {
  memory: MemorySmall | MemoryMedium | MemoryLarge;
}

export interface GetMemoryInput {
  id: string;
  verbosity?: Verbosity;
}

export interface GetMemoryResult {
  memory: MemorySmall | MemoryMedium | MemoryLarge;
}

export interface UpdateMemoryInput {
  id: string;
  title?: string;
  content?: string;
  summary?: string;
  memory_type?: MemoryType;
  source_type?: SourceType;
  source_url?: string | null;
  tags?: string[];
  language?: string;
  version_predicates?: VersionPredicates | null;
  project_name?: string | null;
  project_repo_url?: string | null;
  project_workspace_path?: string | null;
  confidence?: number;
  enabled?: boolean;
}

export interface UpdateMemoryResult {
  memory: Pick<Memory, "id" | "title" | "updated_at">;
  embedding_regenerating: boolean;
}

export interface SubmitFeedbackInput {
  memory_id: string;
  text?: string | null;
  tags?: FeedbackTag[];
  suggested_action?: SuggestedAction | null;
}

export interface SubmitFeedbackResult {
  feedback: Pick<Feedback, "id" | "memory_id" | "created_at">;
  memory_flagged: boolean;
}

export interface DetectEnvironmentInput {
  project_path?: string;
}

export interface DetectEnvironmentResult {
  detected_versions: DetectedVersions;
  scan_sources: Partial<Record<ComponentKey, string>>;
  undetected_components: ComponentKey[];
}
