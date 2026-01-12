-- Add down migration script here

DROP TABLE IF EXISTS native_triggers;

DROP TYPE IF EXISTS native_trigger_service;

DROP TABLE IF EXISTS workspace_integrations;
