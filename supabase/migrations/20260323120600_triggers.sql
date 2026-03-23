-- Migration: 007_triggers
-- Auto-maintenance triggers:
--   1. updated_at: keep updated_at current on memory and secrets tables
--   2. Soft-delete: no cascading required; lineage is preserved by design

-- ============================================================
-- updated_at trigger function (shared)
-- ============================================================
CREATE OR REPLACE FUNCTION public.set_updated_at()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- ============================================================
-- memory.updated_at trigger
-- ============================================================
CREATE OR REPLACE TRIGGER memory_set_updated_at
    BEFORE UPDATE ON public.memory
    FOR EACH ROW
    EXECUTE FUNCTION public.set_updated_at();

-- ============================================================
-- secrets.updated_at trigger
-- ============================================================
CREATE OR REPLACE TRIGGER secrets_set_updated_at
    BEFORE UPDATE ON public.secrets
    FOR EACH ROW
    EXECUTE FUNCTION public.set_updated_at();
