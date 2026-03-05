CREATE TABLE feedback (
  id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  memory_id uuid NOT NULL REFERENCES memory(id) ON DELETE CASCADE,
  text text,
  tags feedback_tag[] DEFAULT '{}',
  suggested_action suggested_action,
  created_at timestamptz NOT NULL DEFAULT now()
);
