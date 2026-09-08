DROP INDEX IF EXISTS index_log_file_premigration;
DROP INDEX IF EXISTS index_log_file_pending_path;
DROP INDEX IF EXISTS index_log_file_pending;
ALTER TABLE log_file DROP COLUMN IF EXISTS indexed_at;
