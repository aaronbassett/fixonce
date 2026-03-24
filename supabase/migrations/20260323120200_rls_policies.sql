-- Migration: 003_rls_policies
-- Enable Row Level Security on all tables.
-- Default: DENY ALL. Explicit policies grant access.

-- ============================================================
-- Enable RLS
-- ============================================================
ALTER TABLE public.memory             ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.feedback           ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.activity_log       ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.secrets            ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.cli_keys           ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.memory_lineage     ENABLE ROW LEVEL SECURITY;
ALTER TABLE public.contradiction_pairs ENABLE ROW LEVEL SECURITY;

-- ============================================================
-- memory policies
-- ============================================================

-- Any authenticated user can read non-deleted memories
CREATE POLICY "memory_select_authenticated"
    ON public.memory
    FOR SELECT
    TO authenticated
    USING (deleted_at IS NULL);

-- Any authenticated user can insert a memory (created_by enforced in app / trigger)
CREATE POLICY "memory_insert_authenticated"
    ON public.memory
    FOR INSERT
    TO authenticated
    WITH CHECK (created_by = auth.uid());

-- Only the creator can update their own memories
CREATE POLICY "memory_update_own"
    ON public.memory
    FOR UPDATE
    TO authenticated
    USING (created_by = auth.uid())
    WITH CHECK (created_by = auth.uid());

-- Only the creator can hard-delete (soft-delete preferred; hard-delete via service_role)
CREATE POLICY "memory_delete_own"
    ON public.memory
    FOR DELETE
    TO authenticated
    USING (created_by = auth.uid());

-- ============================================================
-- feedback policies
-- ============================================================

-- Authenticated users can read all feedback
CREATE POLICY "feedback_select_authenticated"
    ON public.feedback
    FOR SELECT
    TO authenticated
    USING (true);

-- Authenticated users can submit feedback
CREATE POLICY "feedback_insert_authenticated"
    ON public.feedback
    FOR INSERT
    TO authenticated
    WITH CHECK (user_id = auth.uid());

-- No UPDATE or DELETE allowed for regular users (immutable audit trail)

-- ============================================================
-- activity_log policies
-- ============================================================

-- Authenticated users can read the activity log
CREATE POLICY "activity_log_select_authenticated"
    ON public.activity_log
    FOR SELECT
    TO authenticated
    USING (true);

-- Only service_role may write to the activity log (no direct user writes)
CREATE POLICY "activity_log_insert_service_role"
    ON public.activity_log
    FOR INSERT
    TO service_role
    WITH CHECK (true);

-- Only service_role may delete old records (cron job retention)
CREATE POLICY "activity_log_delete_service_role"
    ON public.activity_log
    FOR DELETE
    TO service_role
    USING (true);

-- ============================================================
-- secrets policies
-- All operations are service_role only. Regular users have NO access.
-- ============================================================

CREATE POLICY "secrets_all_service_role"
    ON public.secrets
    FOR ALL
    TO service_role
    USING (true)
    WITH CHECK (true);

-- ============================================================
-- cli_keys policies
-- Users may only manage their own keys.
-- ============================================================

CREATE POLICY "cli_keys_select_own"
    ON public.cli_keys
    FOR SELECT
    TO authenticated
    USING (user_id = auth.uid());

CREATE POLICY "cli_keys_insert_own"
    ON public.cli_keys
    FOR INSERT
    TO authenticated
    WITH CHECK (user_id = auth.uid());

CREATE POLICY "cli_keys_update_own"
    ON public.cli_keys
    FOR UPDATE
    TO authenticated
    USING (user_id = auth.uid())
    WITH CHECK (user_id = auth.uid());

CREATE POLICY "cli_keys_delete_own"
    ON public.cli_keys
    FOR DELETE
    TO authenticated
    USING (user_id = auth.uid());

-- ============================================================
-- memory_lineage policies
-- ============================================================

-- Authenticated users can read lineage (provenance transparency)
CREATE POLICY "memory_lineage_select_authenticated"
    ON public.memory_lineage
    FOR SELECT
    TO authenticated
    USING (true);

-- Only service_role writes lineage records (written by the pipeline, not directly by users)
CREATE POLICY "memory_lineage_insert_service_role"
    ON public.memory_lineage
    FOR INSERT
    TO service_role
    WITH CHECK (true);

-- ============================================================
-- contradiction_pairs policies
-- ============================================================

-- Authenticated users can read contradiction pairs
CREATE POLICY "contradiction_pairs_select_authenticated"
    ON public.contradiction_pairs
    FOR SELECT
    TO authenticated
    USING (true);

-- Authenticated users can insert new contradiction pairs (detected by pipeline running as user)
CREATE POLICY "contradiction_pairs_insert_authenticated"
    ON public.contradiction_pairs
    FOR INSERT
    TO authenticated
    WITH CHECK (true);

-- Authenticated users can update (e.g., cast tiebreaker votes, resolve)
CREATE POLICY "contradiction_pairs_update_authenticated"
    ON public.contradiction_pairs
    FOR UPDATE
    TO authenticated
    USING (true)
    WITH CHECK (true);

-- No DELETE allowed on contradiction pairs (preserve history)
