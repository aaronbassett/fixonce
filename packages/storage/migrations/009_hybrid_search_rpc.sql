CREATE OR REPLACE FUNCTION hybrid_search(
  query_text text,
  query_embedding vector(1024),
  match_count int,
  full_text_weight float DEFAULT 1.0,
  semantic_weight float DEFAULT 1.0,
  rrf_k int DEFAULT 50
)
RETURNS SETOF memory
LANGUAGE sql
AS $$
WITH full_text AS (
  SELECT id,
    row_number() OVER (
      ORDER BY ts_rank_cd(fts, websearch_to_tsquery(query_text)) DESC
    ) AS rank_ix
  FROM memory
  WHERE fts @@ websearch_to_tsquery(query_text)
    AND enabled = true
  LIMIT least(match_count, 30) * 2
),
semantic AS (
  SELECT id,
    row_number() OVER (
      ORDER BY embedding <=> query_embedding
    ) AS rank_ix
  FROM memory
  WHERE enabled = true
    AND embedding IS NOT NULL
  LIMIT least(match_count, 30) * 2
)
SELECT memory.*
FROM full_text
FULL OUTER JOIN semantic ON full_text.id = semantic.id
JOIN memory ON coalesce(full_text.id, semantic.id) = memory.id
ORDER BY
  coalesce(1.0 / (rrf_k + full_text.rank_ix), 0.0) * full_text_weight +
  coalesce(1.0 / (rrf_k + semantic.rank_ix), 0.0) * semantic_weight
  DESC
LIMIT least(match_count, 30);
$$;
