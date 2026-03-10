-- Git sync test fixture
-- Repo 1: syncs scripts+flows+apps under f/
-- Repo 2: syncs only scripts under g/, excludes g/internal/
UPDATE workspace_settings SET git_sync = '{
  "include_path": ["f/**"],
  "include_type": ["script", "flow", "app", "folder", "resource", "variable"],
  "repositories": [
    {
      "script_path": "hub/28160/sync-script-to-git-repo-windmill",
      "git_repo_resource_path": "$res:u/test-user/git_repo_1",
      "use_individual_branch": false,
      "group_by_folder": false,
      "settings": {
        "include_path": ["f/**"],
        "include_type": ["script", "flow", "app"],
        "exclude_path": [],
        "extra_include_path": []
      }
    },
    {
      "script_path": "hub/28160/sync-script-to-git-repo-windmill",
      "git_repo_resource_path": "$res:u/test-user/git_repo_2",
      "use_individual_branch": false,
      "group_by_folder": true,
      "settings": {
        "include_path": ["g/**"],
        "include_type": ["script"],
        "exclude_path": ["g/internal/**"],
        "extra_include_path": []
      }
    }
  ]
}'::jsonb WHERE workspace_id = 'test-workspace';
