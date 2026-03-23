-- Migration: 005_fts_setup
-- Full-text search trigger: maintains the fts_vector column automatically.
-- Weight scheme:
--   title   => 'A' (highest weight)
--   summary => 'B'
--   content => 'C'

CREATE OR REPLACE FUNCTION public.memory_fts_update()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.fts_vector :=
          setweight(to_tsvector('english', coalesce(NEW.title,   '')), 'A')
       || setweight(to_tsvector('english', coalesce(NEW.summary, '')), 'B')
       || setweight(to_tsvector('english', coalesce(NEW.content, '')), 'C');
    RETURN NEW;
END;
$$;

-- Fire before INSERT or UPDATE so the computed value is stored in the same row write
CREATE OR REPLACE TRIGGER memory_fts_update_trigger
    BEFORE INSERT OR UPDATE OF title, summary, content
    ON public.memory
    FOR EACH ROW
    EXECUTE FUNCTION public.memory_fts_update();
