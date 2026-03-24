-- Migration: 20260324120000_dashboard_rpcs
-- RPC functions that power the FixOnce dashboard.
--
-- Functions:
--   dashboard_stats()                        — aggregate counts for the summary cards
--   dashboard_activity_heatmap(months int)   — per-day, per-action counts for the calendar heatmap
--   dashboard_recent_views(lim int)          — most-recently accessed memories
--   dashboard_most_accessed(lim int)         — memories ranked by total access count

-- ---------------------------------------------------------------------------
-- 1. dashboard_stats
-- ---------------------------------------------------------------------------
-- Returns a single row of headline metrics shown at the top of the dashboard.

CREATE OR REPLACE FUNCTION public.dashboard_stats()
RETURNS TABLE (
    total_memories  bigint,
    searches_24h    bigint,
    reports_24h     bigint
)
LANGUAGE sql
STABLE
SECURITY INVOKER
AS $$
    SELECT
        (SELECT COUNT(*) FROM public.memory         WHERE deleted_at IS NULL)                                                          AS total_memories,
        (SELECT COUNT(*) FROM public.activity_log   WHERE action = 'memory.searched'   AND created_at > now() - interval '24 hours')  AS searches_24h,
        (SELECT COUNT(*) FROM public.activity_log   WHERE action = 'feedback.submitted' AND created_at > now() - interval '24 hours') AS reports_24h;
$$;

GRANT EXECUTE ON FUNCTION public.dashboard_stats()
    TO authenticated;


-- ---------------------------------------------------------------------------
-- 2. dashboard_activity_heatmap
-- ---------------------------------------------------------------------------
-- Returns one row per (day, action) pair so the client can render a GitHub-style
-- contribution heatmap. Only the three actions that signal active usage are included.

CREATE OR REPLACE FUNCTION public.dashboard_activity_heatmap(months int DEFAULT 6)
RETURNS TABLE (
    day     date,
    action  text,
    count   bigint
)
LANGUAGE sql
STABLE
SECURITY INVOKER
AS $$
    SELECT
        DATE(created_at)    AS day,
        action,
        COUNT(*)            AS count
    FROM public.activity_log
    WHERE
        created_at > now() - (months || ' months')::interval
        AND action IN ('memory.created', 'memory.accessed', 'memory.searched')
    GROUP BY DATE(created_at), action
    ORDER BY day;
$$;

GRANT EXECUTE ON FUNCTION public.dashboard_activity_heatmap(int)
    TO authenticated;


-- ---------------------------------------------------------------------------
-- 3. dashboard_recent_views
-- ---------------------------------------------------------------------------
-- Returns the most recently accessed non-deleted memories, one row per memory
-- (DISTINCT ON entity_id keeps only the latest access event per memory).

CREATE OR REPLACE FUNCTION public.dashboard_recent_views(lim int DEFAULT 20)
RETURNS TABLE (
    memory_id   uuid,
    title       text,
    memory_type text,
    decay_score float8,
    last_viewed timestamptz
)
LANGUAGE sql
STABLE
SECURITY INVOKER
AS $$
    SELECT
        memory_id,
        title,
        memory_type,
        decay_score,
        last_viewed
    FROM (
        SELECT DISTINCT ON (a.entity_id)
            a.entity_id     AS memory_id,
            m.title,
            m.memory_type,
            m.decay_score,
            a.created_at    AS last_viewed
        FROM public.activity_log a
        JOIN public.memory m ON m.id = a.entity_id
        WHERE
            a.action = 'memory.accessed'
            AND m.deleted_at IS NULL
        ORDER BY a.entity_id, a.created_at DESC
    ) latest
    ORDER BY last_viewed DESC
    LIMIT lim;
$$;

GRANT EXECUTE ON FUNCTION public.dashboard_recent_views(int)
    TO authenticated;


-- ---------------------------------------------------------------------------
-- 4. dashboard_most_accessed
-- ---------------------------------------------------------------------------
-- Returns non-deleted memories ordered by total number of access events,
-- useful for surfacing the most valuable / frequently consulted memories.

CREATE OR REPLACE FUNCTION public.dashboard_most_accessed(lim int DEFAULT 20)
RETURNS TABLE (
    memory_id    uuid,
    title        text,
    memory_type  text,
    decay_score  float8,
    access_count bigint
)
LANGUAGE sql
STABLE
SECURITY INVOKER
AS $$
    SELECT
        m.id            AS memory_id,
        m.title,
        m.memory_type,
        m.decay_score,
        COUNT(a.id)     AS access_count
    FROM public.activity_log a
    JOIN public.memory m ON m.id = a.entity_id
    WHERE
        a.action = 'memory.accessed'
        AND m.deleted_at IS NULL
    GROUP BY m.id, m.title, m.memory_type, m.decay_score
    ORDER BY access_count DESC
    LIMIT lim;
$$;

GRANT EXECUTE ON FUNCTION public.dashboard_most_accessed(int)
    TO authenticated;
