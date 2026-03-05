CREATE TYPE memory_type AS ENUM ('guidance', 'anti_pattern');
CREATE TYPE source_type AS ENUM ('correction', 'discovery', 'instruction');
CREATE TYPE created_by AS ENUM ('ai', 'human', 'human_modified');
CREATE TYPE feedback_tag AS ENUM ('helpful', 'not_helpful', 'damaging', 'accurate', 'somewhat_accurate', 'somewhat_inaccurate', 'inaccurate', 'outdated');
CREATE TYPE suggested_action AS ENUM ('keep', 'remove', 'fix');
CREATE TYPE operation_type AS ENUM ('query', 'create', 'update', 'feedback', 'detect');
