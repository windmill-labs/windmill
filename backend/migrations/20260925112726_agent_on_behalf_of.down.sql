ALTER TABLE resource DISABLE TRIGGER record_resource_version_update_trigger;
UPDATE resource SET value = value - 'on_behalf_of'
 WHERE resource_type = 'ai_agent' AND jsonb_typeof(value) = 'object' AND value ? 'on_behalf_of';
ALTER TABLE resource ENABLE TRIGGER record_resource_version_update_trigger;
