-- Add up migration script here
CREATE SEQUENCE IF NOT EXISTS flow_node_hash_seq;
ALTER TABLE flow_node ALTER COLUMN hash DROP NOT NULL;
ALTER TABLE flow_node ADD COLUMN hash_v2 CHAR(64) NOT NULL UNIQUE DEFAULT to_hex(nextval('flow_node_hash_seq'));
