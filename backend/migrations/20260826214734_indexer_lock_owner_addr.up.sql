-- The indexer records the base URL other instances can reach it on, so a server
-- instance without an in-memory index reader can forward search requests to it
-- instead of requiring the ingress to pin /api/srch/* to the indexer pod.
ALTER TABLE concurrency_locks ADD COLUMN IF NOT EXISTS owner_addr VARCHAR;
