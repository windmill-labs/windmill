ALTER TABLE workspace_settings ADD COLUMN code_completion_enabled BOOLEAN DEFAULT FALSE NOT NULL;

UPDATE workspace_settings SET code_completion_enabled = TRUE WHERE code_completion_model IS NOT NULL;

ALTER TABLE workspace_settings DROP COLUMN ai_models, DROP COLUMN code_completion_model;