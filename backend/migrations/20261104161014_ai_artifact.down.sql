CREATE TABLE ai_shared_artifact (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    artifact_id VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    kind VARCHAR(10) NOT NULL CHECK (kind IN ('md', 'html')),
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    shared_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, email, artifact_id)
);

CREATE INDEX idx_ai_shared_artifact_shared_at ON ai_shared_artifact (shared_at);

GRANT ALL ON ai_shared_artifact TO windmill_admin;
GRANT ALL ON ai_shared_artifact TO windmill_user;

ALTER TABLE ai_shared_artifact ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_shared_artifact FOR ALL TO windmill_admin USING (true);

INSERT INTO ai_shared_artifact (id, workspace_id, artifact_id, email, created_by, name, kind, version, content, shared_at)
SELECT id, workspace_id, artifact_id, email, created_by, name, kind, shown_version, content, updated_at
FROM ai_artifact
WHERE deleted_at IS NULL AND extra_perms <> '{}'::jsonb
ON CONFLICT DO NOTHING;

DROP TABLE ai_artifact;
