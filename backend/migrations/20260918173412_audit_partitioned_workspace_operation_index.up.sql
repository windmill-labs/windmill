-- The backend builds these with CREATE INDEX CONCURRENTLY instead (per partition for
-- audit_partitioned), see create_audit_operation_index_concurrently in windmill-api/src/db.rs.
-- id matches list_audit's order and before_id cursor. timestamp trails it so a
-- time window is checked in the index rather than on every row it reads.
CREATE INDEX IF NOT EXISTS ix_audit_workspace_operation
    ON audit (workspace_id, operation, id DESC, "timestamp");
CREATE INDEX IF NOT EXISTS ix_audit_partitioned_workspace_operation
    ON audit_partitioned (workspace_id, operation, id DESC, "timestamp");
