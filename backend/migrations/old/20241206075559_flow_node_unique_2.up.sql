-- Add up migration script here
ALTER TABLE flow_node ADD CONSTRAINT flow_node_unique_2 UNIQUE (path, workspace_id, hash_v2);
ALTER TABLE flow_node DROP CONSTRAINT IF EXISTS flow_node_hash_v2_key;
