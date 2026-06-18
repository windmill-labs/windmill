-- Add Google to native_trigger_service enum
-- 'google' is a unified service that handles both Drive and Calendar triggers
-- The trigger_type field in service_config determines which Google service is used
ALTER TYPE native_trigger_service ADD VALUE IF NOT EXISTS 'google';

-- Add to TRIGGER_KIND enum (used for trigger tracking)
ALTER TYPE TRIGGER_KIND ADD VALUE IF NOT EXISTS 'google';

-- Add to job_trigger_kind enum (used for job tracking)
ALTER TYPE job_trigger_kind ADD VALUE IF NOT EXISTS 'google';
