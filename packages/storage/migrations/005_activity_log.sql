CREATE TABLE activity_log (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  operation operation_type NOT NULL,
  memory_id uuid REFERENCES memory(id) ON DELETE SET NULL,
  details jsonb NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
