-- Fixture for the GHSA-8mv7-hmrg-96xv regression test (loaded after base.sql).
--
-- A folder `secret` owned only by test-user-2. A folder-scoped flow lives
-- inside it. test-user-3 (a non-admin workspace member with no folder grant)
-- must not be able to run that flow by EITHER its path or its version id.

INSERT INTO folder (name, workspace_id, display_name, owners, extra_perms)
VALUES ('secret', 'test-workspace', 'secret', ARRAY['u/test-user-2'],
        '{"u/test-user-2": true}'::jsonb);

INSERT INTO flow (workspace_id, summary, description, path, versions, schema, value, edited_by)
VALUES ('test-workspace', '', '', 'f/secret/admin_flow', '{9000001}',
        '{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{},"required":[]}',
        '{"modules":[{"id":"a","value":{"type":"rawscript","language":"bash","content":"echo hello","input_transforms":{}}}]}',
        'test-user-2');

INSERT INTO flow_version (id, workspace_id, path, schema, value, created_by)
VALUES (9000001, 'test-workspace', 'f/secret/admin_flow',
        '{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{},"required":[]}',
        '{"modules":[{"id":"a","value":{"type":"rawscript","language":"bash","content":"echo hello","input_transforms":{}}}]}',
        'test-user-2');
