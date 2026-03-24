-- Migration: 001_extensions
-- Enable required PostgreSQL extensions
-- These are idempotent (IF NOT EXISTS) per EC-06

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS vector;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
CREATE EXTENSION IF NOT EXISTS pg_cron;
