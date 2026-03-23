-- Migration: 008_cron_jobs
-- Scheduled maintenance via pg_cron.
-- Requires the pg_cron extension (enabled in 001_extensions).

-- activity_log 90-day retention: runs daily at 03:00 UTC
-- Removes rows older than 90 days to bound table growth.
-- Uses service_role context implicitly (pg_cron runs as superuser).
SELECT cron.schedule(
    'cleanup-activity-log',
    '0 3 * * *',
    $$DELETE FROM public.activity_log WHERE created_at < now() - interval '90 days'$$
);
