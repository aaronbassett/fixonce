ALTER TABLE memory ADD COLUMN fts tsvector
  GENERATED ALWAYS AS (
    setweight(to_tsvector('english', coalesce(title, '')), 'A') ||
    setweight(to_tsvector('english', coalesce(summary, '')), 'B') ||
    setweight(to_tsvector('english', coalesce(content, '')), 'C') ||
    setweight(to_tsvector('english', coalesce(array_to_string(tags, ' '), '')), 'D')
  ) STORED;

CREATE INDEX idx_memory_fts ON memory USING gin (fts);
