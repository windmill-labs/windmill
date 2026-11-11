-- An AI session artifact its owner shared: a path-addressed item like scripts and flows, so its
-- path and `extra_perms` decide who reads it, through the policies below. Artifacts live in their
-- owner's browser; a row exists from the first share until the owner deletes the artifact, and
-- holds only the version people with access see.
CREATE TABLE ai_artifact (
    -- The id links shared before this table carried; links use `artifact_id`.
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    -- The browser-side artifact id, which links name.
    artifact_id VARCHAR(255) NOT NULL,
    path VARCHAR(255) NOT NULL,
    extra_perms JSONB NOT NULL DEFAULT '{}',
    email VARCHAR(255) NOT NULL,
    created_by VARCHAR(255) NOT NULL,
    session_id VARCHAR(255),
    name VARCHAR(255) NOT NULL,
    kind VARCHAR(10) NOT NULL CHECK (kind IN ('md', 'html')),
    role VARCHAR(10) CHECK (role IN ('plan')),
    -- The version people with access see, and its text.
    shown_version INTEGER NOT NULL,
    content TEXT NOT NULL,
    -- The newest version the owner wrote. Above `shown_version` only while `pinned`.
    latest_version INTEGER NOT NULL,
    -- The SHA-256 of that version's text, kept while an older one is pinned. A write is refused
    -- unless it was made on this text: two of the owner's browsers can each hold a different
    -- text under the same version number.
    latest_hash VARCHAR(64) NOT NULL,
    pinned BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- When `content` last changed.
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- In the owner's trash: nobody else reads it, and the monitor purges it after 30 days.
    deleted_at TIMESTAMPTZ,
    UNIQUE (workspace_id, path),
    UNIQUE (workspace_id, artifact_id)
);

CREATE INDEX idx_ai_artifact_owner ON ai_artifact (workspace_id, email);
CREATE INDEX idx_ai_artifact_deleted_at ON ai_artifact (deleted_at) WHERE deleted_at IS NOT NULL;

GRANT ALL ON ai_artifact TO windmill_user;
GRANT ALL ON ai_artifact TO windmill_admin;

ALTER TABLE ai_artifact ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_artifact FOR ALL TO windmill_admin USING (true);

CREATE POLICY see_folder_extra_perms_user_select ON ai_artifact FOR SELECT TO windmill_user
USING (SPLIT_PART(ai_artifact.path, '/', 1) = 'f' AND SPLIT_PART(ai_artifact.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_read'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_insert ON ai_artifact FOR INSERT TO windmill_user
WITH CHECK (SPLIT_PART(ai_artifact.path, '/', 1) = 'f' AND SPLIT_PART(ai_artifact.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_update ON ai_artifact FOR UPDATE TO windmill_user
USING (SPLIT_PART(ai_artifact.path, '/', 1) = 'f' AND SPLIT_PART(ai_artifact.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));
CREATE POLICY see_folder_extra_perms_user_delete ON ai_artifact FOR DELETE TO windmill_user
USING (SPLIT_PART(ai_artifact.path, '/', 1) = 'f' AND SPLIT_PART(ai_artifact.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.folders_write'), ','))::text[]));

CREATE POLICY see_own ON ai_artifact FOR ALL TO windmill_user
USING (SPLIT_PART(ai_artifact.path, '/', 1) = 'u' AND SPLIT_PART(ai_artifact.path, '/', 2) = (select current_setting('session.user')));
CREATE POLICY see_member ON ai_artifact FOR ALL TO windmill_user
USING (SPLIT_PART(ai_artifact.path, '/', 1) = 'g' AND SPLIT_PART(ai_artifact.path, '/', 2) = any((select regexp_split_to_array(current_setting('session.groups'), ','))::text[]));

CREATE POLICY see_extra_perms_user_select ON ai_artifact FOR SELECT TO windmill_user
USING (extra_perms ? (select concat('u/', current_setting('session.user'))));
CREATE POLICY see_extra_perms_user_insert ON ai_artifact FOR INSERT TO windmill_user
WITH CHECK ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_update ON ai_artifact FOR UPDATE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);
CREATE POLICY see_extra_perms_user_delete ON ai_artifact FOR DELETE TO windmill_user
USING ((extra_perms ->> (select concat('u/', current_setting('session.user'))))::boolean);

CREATE POLICY see_extra_perms_groups_select ON ai_artifact FOR SELECT TO windmill_user
USING (extra_perms ?| (select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[]);
CREATE POLICY see_extra_perms_groups_insert ON ai_artifact FOR INSERT TO windmill_user
WITH CHECK (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_update ON ai_artifact FOR UPDATE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));
CREATE POLICY see_extra_perms_groups_delete ON ai_artifact FOR DELETE TO windmill_user
USING (exists(
    SELECT key, value FROM jsonb_each_text(extra_perms)
    WHERE SPLIT_PART(key, '/', 1) = 'g' AND key = ANY((select regexp_split_to_array(current_setting('session.pgroups'), ','))::text[])
    AND value::boolean));

-- Every share still here keeps its link and shows exactly what it showed: shared with the whole
-- workspace, pinned to its snapshot. A second share of the same name by the same user takes a
-- suffix from its id, which no name the user types produces. Shares past the instance's
-- AI_SHARED_ARTIFACT_RETENTION_SECS were already deleted by the monitor.
INSERT INTO ai_artifact (
    id, workspace_id, artifact_id, path, extra_perms, email, created_by, name, kind,
    shown_version, content, latest_version, latest_hash, pinned, created_at, updated_at
)
SELECT
    s.id, s.workspace_id, s.artifact_id,
    'u/' || s.created_by || '/' || s.slug
        || CASE WHEN s.rn > 1 THEN '_' || left(replace(s.id::text, '-', ''), 8) ELSE '' END,
    '{"g/all": false}'::jsonb, s.email, s.created_by, s.name, s.kind,
    s.version, s.content, s.version, encode(sha256(convert_to(s.content, 'UTF8')), 'hex'), true,
    s.shared_at, s.shared_at
FROM (
    SELECT a.*, row_number() OVER (
        PARTITION BY a.workspace_id, a.created_by, a.slug ORDER BY a.shared_at, a.id
    ) AS rn
    FROM (
        SELECT ai_shared_artifact.*,
            COALESCE(NULLIF(left(trim(BOTH '_' FROM regexp_replace(lower(name), '[^a-z0-9]+', '_', 'g')), 100), ''), 'artifact') AS slug
        FROM ai_shared_artifact
    ) a
) s
ON CONFLICT DO NOTHING;

DROP TABLE ai_shared_artifact;
