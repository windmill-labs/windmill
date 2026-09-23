-- A workspace whose auto-pulled repository last recorded a head-check failure while
-- already synced to head "aaa": the state a recovery write is decided on.

INSERT INTO workspace (id, name, owner) VALUES ('ap-ws', 'ap-ws', 'test-user');

INSERT INTO workspace_settings (workspace_id, git_sync) VALUES
    ('ap-ws', '{"repositories":[{"git_repo_resource_path":"$res:u/admin/repo",
        "auto_pull":{"enabled":true,"mode":"polling",
            "last_synced_sha":{"main":"aaa"},
            "last_pull_status":{"success":false,"at":1,"error":"head check failed: x"}}}]}');
