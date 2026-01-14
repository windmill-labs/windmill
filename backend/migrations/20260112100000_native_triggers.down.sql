-- Add down migration script here

DROP TABLE IF EXISTS native_trigger;

DROP TABLE IF EXISTS workspace_integrations;

DROP TYPE IF EXISTS native_trigger_service;
