-- Add down migration script here
DROP TABLE IF EXISTS azure_trigger;
DROP TYPE IF EXISTS AZURE_TRIGGER_MODE;
-- Note: TRIGGER_KIND and JOB_TRIGGER_KIND enums cannot have values removed
-- so we leave 'azure' in place as a dead value (matches other trigger migrations)
