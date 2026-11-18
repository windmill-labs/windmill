-- The repository URL a workspace admin last saved on an app-backed git_repository
-- resource. Anyone who can write the resource controls its URL, so the GitHub App
-- installation serves a non-admin only the repository this names. On the row
-- rather than keyed by path, so it follows every rename and dies with the row.
ALTER TABLE resource ADD COLUMN github_app_approved_url TEXT;
