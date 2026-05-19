-- Expand the pipeline trigger enum to cover every non-integration
-- (i.e. non-native) trigger kind Windmill supports. Each value mirrors a
-- keyword the annotation parser recognises in `// on <kind> <ref>` lines.
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'webhook';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'email';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'kafka';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'mqtt';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'nats';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'postgres';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'sqs';
ALTER TYPE SCRIPT_TRIGGER_KIND ADD VALUE IF NOT EXISTS 'gcp';
