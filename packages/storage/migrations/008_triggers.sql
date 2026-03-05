CREATE OR REPLACE FUNCTION update_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER memory_updated_at
  BEFORE UPDATE ON memory
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at();
