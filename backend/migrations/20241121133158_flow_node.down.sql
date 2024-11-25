-- Add down migration script here
DROP TABLE IF EXISTS flow_version_lite;
DROP TABLE IF EXISTS flow_node;
DROP INDEX IF EXISTS flow_node_hash;
