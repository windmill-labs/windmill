ALTER TABLE kafka_pending_commits
    DROP CONSTRAINT kafka_pending_commits_workspace_id_kafka_trigger_path_fkey,
    ADD CONSTRAINT kafka_pending_commits_workspace_id_kafka_trigger_path_fkey
        FOREIGN KEY (workspace_id, kafka_trigger_path) REFERENCES kafka_trigger(workspace_id, path)
        ON DELETE CASCADE;
