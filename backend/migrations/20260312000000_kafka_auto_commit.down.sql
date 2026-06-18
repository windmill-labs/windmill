DROP TABLE IF EXISTS kafka_pending_commits;
ALTER TABLE kafka_trigger DROP COLUMN auto_commit;
