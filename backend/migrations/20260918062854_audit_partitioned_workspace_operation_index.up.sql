-- The backend builds this per partition with CREATE INDEX CONCURRENTLY instead,
-- see create_audit_operation_index_concurrently in windmill-api/src/db.rs
CREATE INDEX IF NOT EXISTS ix_audit_partitioned_workspace_operation
    ON audit_partitioned (workspace_id, operation, "timestamp" DESC);
