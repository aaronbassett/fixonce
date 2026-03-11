CREATE OR REPLACE FUNCTION batch_increment_surfaced_count(memory_ids uuid[])
RETURNS void
LANGUAGE sql
SECURITY DEFINER
SET search_path = ''
AS $$
  UPDATE public.memory
  SET surfaced_count = surfaced_count + 1,
      last_surfaced_at = now()
  WHERE id = ANY(memory_ids);
$$;
