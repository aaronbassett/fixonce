-- Migration: 004_indexes
-- Performance indexes for all tables

-- ============================================================
-- memory indexes
-- ============================================================

-- Vector similarity search (HNSW — best recall/performance trade-off for pgvector)
-- m=16, ef_construction=64 are balanced defaults; tune for production load
CREATE INDEX IF NOT EXISTS memory_embedding_hnsw_idx
    ON public.memory
    USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

-- Full-text search (GIN is the standard index type for tsvector)
CREATE INDEX IF NOT EXISTS memory_fts_vector_gin_idx
    ON public.memory
    USING gin (fts_vector);

-- Filter / group by memory type
CREATE INDEX IF NOT EXISTS memory_memory_type_idx
    ON public.memory (memory_type);

-- Filter by creator
CREATE INDEX IF NOT EXISTS memory_created_by_idx
    ON public.memory (created_by);

-- Partial index: only non-deleted rows (the common query pattern)
CREATE INDEX IF NOT EXISTS memory_not_deleted_idx
    ON public.memory (deleted_at)
    WHERE deleted_at IS NULL;

-- Decay score ordering (used by the hybrid_search result ranking / pruning)
CREATE INDEX IF NOT EXISTS memory_decay_score_idx
    ON public.memory (decay_score);

-- ============================================================
-- feedback indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS feedback_memory_id_idx
    ON public.feedback (memory_id);

CREATE INDEX IF NOT EXISTS feedback_user_id_idx
    ON public.feedback (user_id);

-- ============================================================
-- activity_log indexes
-- ============================================================

-- Time-range queries (the primary access pattern for the log)
CREATE INDEX IF NOT EXISTS activity_log_created_at_idx
    ON public.activity_log (created_at);

-- ============================================================
-- cli_keys indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS cli_keys_user_id_idx
    ON public.cli_keys (user_id);

-- ============================================================
-- memory_lineage indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS memory_lineage_memory_id_idx
    ON public.memory_lineage (memory_id);

CREATE INDEX IF NOT EXISTS memory_lineage_parent_id_idx
    ON public.memory_lineage (parent_id);

-- ============================================================
-- contradiction_pairs indexes
-- ============================================================

CREATE INDEX IF NOT EXISTS contradiction_pairs_resolution_status_idx
    ON public.contradiction_pairs (resolution_status);
