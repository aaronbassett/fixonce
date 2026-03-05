-- Version predicates (JSONB key existence with GIN)
CREATE INDEX idx_memory_version_predicates ON memory USING gin (version_predicates jsonb_path_ops);

-- Tags array containment
CREATE INDEX idx_memory_tags ON memory USING gin (tags);

-- Partial index on enabled memories
CREATE INDEX idx_memory_enabled ON memory (enabled) WHERE enabled = true;

-- Language filter
CREATE INDEX idx_memory_language ON memory (language);

-- Memory type filter
CREATE INDEX idx_memory_memory_type ON memory (memory_type);

-- Vector similarity (HNSW)
CREATE INDEX idx_memory_embedding ON memory USING hnsw (embedding vector_cosine_ops);

-- Feedback indexes
CREATE INDEX idx_feedback_memory_id ON feedback (memory_id);
CREATE INDEX idx_feedback_suggested_action ON feedback (suggested_action) WHERE suggested_action IN ('remove', 'fix');

-- Activity log indexes
CREATE INDEX idx_activity_log_created_at ON activity_log (created_at DESC);
CREATE INDEX idx_activity_log_memory_id ON activity_log (memory_id);
CREATE INDEX idx_activity_log_operation ON activity_log (operation);
