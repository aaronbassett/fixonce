-- Migration: 002_tables
-- Create all 7 application tables per the v2 data model

-- 1. memory: core knowledge store
CREATE TABLE IF NOT EXISTS public.memory (
    id                  uuid            NOT NULL DEFAULT gen_random_uuid(),
    title               text            NOT NULL,
    content             text            NOT NULL,
    summary             text            NOT NULL,
    memory_type         text            NOT NULL CHECK (memory_type IN ('gotcha', 'best_practice', 'correction', 'anti_pattern', 'discovery')),
    source_type         text            NOT NULL CHECK (source_type IN ('correction', 'observation', 'pr_feedback', 'manual', 'harvested')),
    language            text,
    embedding           vector(1024),
    fts_vector          tsvector,
    -- version / environment metadata
    compact_pragma      text,
    compact_compiler    text,
    midnight_js         text,
    indexer_version     text,
    node_version        text,
    -- provenance
    source_url          text,
    repo_url            text,
    task_summary        text,
    session_id          text,
    -- scoring
    decay_score         float8          NOT NULL DEFAULT 1.0,
    reinforcement_score float8          NOT NULL DEFAULT 0,
    last_accessed_at    timestamptz,
    -- pipeline state
    embedding_status    text            NOT NULL DEFAULT 'complete' CHECK (embedding_status IN ('complete', 'pending', 'failed')),
    pipeline_status     text            NOT NULL DEFAULT 'complete' CHECK (pipeline_status IN ('complete', 'incomplete')),
    -- soft-delete
    deleted_at          timestamptz,
    -- audit
    created_at          timestamptz     NOT NULL DEFAULT now(),
    updated_at          timestamptz     NOT NULL DEFAULT now(),
    created_by          uuid            NOT NULL REFERENCES auth.users(id),

    CONSTRAINT memory_pkey PRIMARY KEY (id)
);

-- 2. feedback: user ratings on memories
CREATE TABLE IF NOT EXISTS public.feedback (
    id          uuid        NOT NULL DEFAULT gen_random_uuid(),
    memory_id   uuid        NOT NULL REFERENCES public.memory(id),
    user_id     uuid        NOT NULL,
    rating      text        NOT NULL CHECK (rating IN ('helpful', 'outdated', 'damaging')),
    context     text,
    created_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT feedback_pkey PRIMARY KEY (id)
);

-- 3. activity_log: audit trail of all significant actions
CREATE TABLE IF NOT EXISTS public.activity_log (
    id          uuid        NOT NULL DEFAULT gen_random_uuid(),
    user_id     uuid,
    action      text        NOT NULL,
    entity_type text        NOT NULL,
    entity_id   uuid,
    metadata    jsonb               DEFAULT '{}',
    created_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT activity_log_pkey PRIMARY KEY (id)
);

-- 4. secrets: encrypted key-value store for sensitive config
CREATE TABLE IF NOT EXISTS public.secrets (
    id          uuid        NOT NULL DEFAULT gen_random_uuid(),
    name        text        NOT NULL UNIQUE,
    ciphertext  bytea       NOT NULL,
    iv          bytea       NOT NULL,
    created_by  uuid        NOT NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT secrets_pkey PRIMARY KEY (id)
);

-- 5. cli_keys: public keys registered by CLI clients
CREATE TABLE IF NOT EXISTS public.cli_keys (
    id          uuid        NOT NULL DEFAULT gen_random_uuid(),
    user_id     uuid        NOT NULL REFERENCES auth.users(id),
    public_key  text        NOT NULL UNIQUE,
    label       text,
    last_used_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT cli_keys_pkey PRIMARY KEY (id)
);

-- 6. memory_lineage: tracks how memories evolve over time
CREATE TABLE IF NOT EXISTS public.memory_lineage (
    id          uuid        NOT NULL DEFAULT gen_random_uuid(),
    memory_id   uuid        NOT NULL REFERENCES public.memory(id),
    parent_id   uuid        REFERENCES public.memory(id),
    action      text        NOT NULL CHECK (action IN ('replace', 'update', 'merge', 'feedback', 'create')),
    rationale   text,
    metadata    jsonb               DEFAULT '{}',
    created_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT memory_lineage_pkey PRIMARY KEY (id)
);

-- 7. contradiction_pairs: detected contradictions between memories
CREATE TABLE IF NOT EXISTS public.contradiction_pairs (
    id                  uuid        NOT NULL DEFAULT gen_random_uuid(),
    memory_a_id         uuid        NOT NULL REFERENCES public.memory(id),
    memory_b_id         uuid        NOT NULL REFERENCES public.memory(id),
    resolution_status   text        NOT NULL DEFAULT 'open' CHECK (resolution_status IN ('open', 'resolved', 'dismissed')),
    tiebreaker_votes    jsonb       NOT NULL DEFAULT '[]',
    detected_at         timestamptz NOT NULL DEFAULT now(),
    resolved_at         timestamptz,

    CONSTRAINT contradiction_pairs_pkey         PRIMARY KEY (id),
    CONSTRAINT contradiction_pairs_unique_pair  UNIQUE (memory_a_id, memory_b_id)
);
