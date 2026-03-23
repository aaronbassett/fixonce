-- Migration: 006_hybrid_search_rpc
-- Hybrid search function combining FTS + vector similarity via Reciprocal Rank Fusion (RRF).
--
-- search_type:
--   'hybrid' (default) — combine FTS and vector scores via RRF
--   'fts'              — full-text search only
--   'vector'           — vector similarity only
--
-- RRF formula: score = 1/(60 + rank_fts) + 1/(60 + rank_vector)
-- The constant 60 is the standard RRF k parameter (Cormack et al., 2009).
--
-- version_filters (jsonb) accepted keys:
--   compact_pragma, compact_compiler, midnight_js, indexer_version, node_version

CREATE OR REPLACE FUNCTION public.hybrid_search(
    query_text      text,
    query_embedding vector(1024),
    search_type     text    DEFAULT 'hybrid',
    result_limit    int     DEFAULT 20,
    version_filters jsonb   DEFAULT '{}'
)
RETURNS TABLE (
    memory_id           uuid,
    title               text,
    summary             text,
    content             text,
    memory_type         text,
    language            text,
    compact_pragma      text,
    compact_compiler    text,
    midnight_js         text,
    indexer_version     text,
    node_version        text,
    source_url          text,
    decay_score         float8,
    reinforcement_score float8,
    rrf_score           float8,
    rank                bigint,
    created_at          timestamptz,
    updated_at          timestamptz
)
LANGUAGE plpgsql
STABLE
SECURITY INVOKER
AS $$
DECLARE
    v_query_tsquery tsquery;
BEGIN
    -- Parse the query text into a tsquery for FTS (plain matching, tolerant of punctuation)
    v_query_tsquery := plainto_tsquery('english', query_text);

    RETURN QUERY
    WITH base AS (
        -- Pre-filter to reduce the working set before ranking
        SELECT
            m.id,
            m.title,
            m.summary,
            m.content,
            m.memory_type,
            m.language,
            m.compact_pragma,
            m.compact_compiler,
            m.midnight_js,
            m.indexer_version,
            m.node_version,
            m.source_url,
            m.decay_score,
            m.reinforcement_score,
            m.fts_vector,
            m.embedding,
            m.created_at,
            m.updated_at
        FROM public.memory m
        WHERE
            m.deleted_at IS NULL
            AND m.decay_score > 0.1
            -- Version filters (only applied when the key is present in the jsonb arg)
            AND (version_filters->>'compact_pragma'   IS NULL OR m.compact_pragma   = version_filters->>'compact_pragma')
            AND (version_filters->>'compact_compiler' IS NULL OR m.compact_compiler = version_filters->>'compact_compiler')
            AND (version_filters->>'midnight_js'      IS NULL OR m.midnight_js      = version_filters->>'midnight_js')
            AND (version_filters->>'indexer_version'  IS NULL OR m.indexer_version  = version_filters->>'indexer_version')
            AND (version_filters->>'node_version'     IS NULL OR m.node_version     = version_filters->>'node_version')
    ),

    -- FTS ranking: only executed when search_type is 'hybrid' or 'fts'
    fts_ranked AS (
        SELECT
            b.id,
            ts_rank(b.fts_vector, v_query_tsquery) AS fts_score,
            ROW_NUMBER() OVER (ORDER BY ts_rank(b.fts_vector, v_query_tsquery) DESC) AS rank_fts
        FROM base b
        WHERE
            search_type IN ('hybrid', 'fts')
            AND v_query_tsquery IS NOT NULL
            AND b.fts_vector @@ v_query_tsquery
    ),

    -- Vector ranking: only executed when search_type is 'hybrid' or 'vector'
    vector_ranked AS (
        SELECT
            b.id,
            1 - (b.embedding <=> query_embedding) AS vector_score,
            ROW_NUMBER() OVER (ORDER BY b.embedding <=> query_embedding ASC) AS rank_vector
        FROM base b
        WHERE
            search_type IN ('hybrid', 'vector')
            AND b.embedding IS NOT NULL
            AND query_embedding IS NOT NULL
    ),

    -- Merge FTS and vector results; rows may appear in one or both sets
    merged AS (
        SELECT
            b.id,
            COALESCE(fr.rank_fts,    1e9::bigint) AS rank_fts,
            COALESCE(vr.rank_vector, 1e9::bigint) AS rank_vector
        FROM base b
        LEFT JOIN fts_ranked    fr ON fr.id = b.id
        LEFT JOIN vector_ranked vr ON vr.id = b.id
        WHERE
            -- Must appear in at least one result set
            fr.id IS NOT NULL OR vr.id IS NOT NULL
    )

    SELECT
        b.id            AS memory_id,
        b.title,
        b.summary,
        b.content,
        b.memory_type,
        b.language,
        b.compact_pragma,
        b.compact_compiler,
        b.midnight_js,
        b.indexer_version,
        b.node_version,
        b.source_url,
        b.decay_score,
        b.reinforcement_score,
        -- RRF score: sum of reciprocal ranks (k=60 is the standard parameter)
        (1.0 / (60 + m.rank_fts) + 1.0 / (60 + m.rank_vector))::float8 AS rrf_score,
        ROW_NUMBER() OVER (ORDER BY (1.0 / (60 + m.rank_fts) + 1.0 / (60 + m.rank_vector)) DESC)::bigint AS rank,
        b.created_at,
        b.updated_at
    FROM merged m
    JOIN base b ON b.id = m.id
    ORDER BY rrf_score DESC
    LIMIT result_limit;
END;
$$;

-- Grant execute to authenticated users (RLS on the underlying table still applies)
GRANT EXECUTE ON FUNCTION public.hybrid_search(text, vector(1024), text, int, jsonb)
    TO authenticated;
