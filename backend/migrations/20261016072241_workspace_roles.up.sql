-- A member's role by name. NULL keeps the legacy reading of `operator` (`operator` / `developer`),
-- so rows written by code that predates roles stay meaningful.
ALTER TABLE usr ADD COLUMN IF NOT EXISTS workspace_role VARCHAR(50);
ALTER TABLE workspace_invite ADD COLUMN IF NOT EXISTS workspace_role VARCHAR(50);

-- Per-workspace role overrides, keyed by role name: `{"<role>": {"label"?, "capabilities": {..}}}`.
ALTER TABLE workspace_settings ADD COLUMN IF NOT EXISTS role_overrides JSONB;

-- The operator settings a workspace changed from their default become its override of the
-- `operator` role. Keys left at their default stay unset, so they follow the instance definition.
-- A value that is not a boolean reads as unset: one hand-edited row must not abort the upgrade.
CREATE OR REPLACE FUNCTION wm_try_bool(v TEXT) RETURNS BOOLEAN AS $$
BEGIN
    RETURN v::boolean;
EXCEPTION WHEN invalid_text_representation THEN
    RETURN NULL;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

WITH caps AS (
    SELECT workspace_id, jsonb_strip_nulls(jsonb_build_object(
        'flows', CASE WHEN wm_try_bool(operator_settings->>'builder_flows') THEN 'compose' END,
        'apps', CASE WHEN wm_try_bool(operator_settings->>'builder_apps') THEN 'compose' END,
        'manage_schedules', CASE WHEN wm_try_bool(operator_settings->>'manage_schedules') IS false THEN false END,
        'manage_triggers', CASE WHEN wm_try_bool(operator_settings->>'manage_triggers') IS false THEN false END,
        'visibility', NULLIF(jsonb_strip_nulls(jsonb_build_object(
            'runs', CASE WHEN wm_try_bool(operator_settings->>'runs') IS false THEN false END,
            'schedules', CASE WHEN wm_try_bool(operator_settings->>'schedules') IS false THEN false END,
            'resources', CASE WHEN wm_try_bool(operator_settings->>'resources') IS false THEN false END,
            'variables', CASE WHEN wm_try_bool(operator_settings->>'variables') IS false THEN false END,
            'assets', CASE WHEN wm_try_bool(operator_settings->>'assets') IS false THEN false END,
            'triggers', CASE WHEN wm_try_bool(operator_settings->>'triggers') IS false THEN false END,
            'audit_logs', CASE WHEN wm_try_bool(operator_settings->>'audit_logs') IS false THEN false END,
            'groups', CASE WHEN wm_try_bool(operator_settings->>'groups') IS false THEN false END,
            'folders', CASE WHEN wm_try_bool(operator_settings->>'folders') IS false THEN false END,
            'workers', CASE WHEN wm_try_bool(operator_settings->>'workers') IS false THEN false END
        )), '{}'::jsonb)
    )) AS caps
    FROM workspace_settings
    WHERE operator_settings IS NOT NULL AND jsonb_typeof(operator_settings) = 'object'
)
UPDATE workspace_settings ws
SET role_overrides = jsonb_build_object('operator', jsonb_build_object('capabilities', caps.caps))
FROM caps
WHERE ws.workspace_id = caps.workspace_id AND caps.caps <> '{}'::jsonb;

DROP FUNCTION wm_try_bool(TEXT);

-- Whether a role costs a full (author) seat, materialized from the role configuration so billing
-- SQL needs no copy of the capability merge. `workspace_id = ''` holds the instance roles; a
-- workspace row exists only for a role that workspace overrides or adds.
CREATE TABLE IF NOT EXISTS role_seat (
    workspace_id VARCHAR(50) NOT NULL,
    role VARCHAR(50) NOT NULL,
    author BOOLEAN NOT NULL,
    PRIMARY KEY (workspace_id, role)
);
INSERT INTO role_seat (workspace_id, role, author)
VALUES ('', 'developer', true), ('', 'operator', false)
ON CONFLICT DO NOTHING;
INSERT INTO role_seat (workspace_id, role, author)
SELECT workspace_id,
       'operator',
       (role_overrides->'operator'->'capabilities'->>'flows') IS NOT NULL
        OR (role_overrides->'operator'->'capabilities'->>'apps') IS NOT NULL
FROM workspace_settings
WHERE role_overrides ? 'operator'
ON CONFLICT DO NOTHING;
