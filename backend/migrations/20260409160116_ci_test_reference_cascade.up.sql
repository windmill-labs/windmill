ALTER TABLE ci_test_reference
    DROP CONSTRAINT ci_test_reference_workspace_id_fkey,
    ADD CONSTRAINT ci_test_reference_workspace_id_fkey
        FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE;
