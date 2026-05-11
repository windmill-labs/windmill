-- Add-on fixture for batch_rerun.rs (combine with `base.sql` via
-- `#[sqlx::test(fixtures("base", "batch_rerun"))]`). Provides a deployed
-- script and a deployed flow so SingleStepFlow wrappers and rerun queries
-- find real runnable rows to join against.

INSERT INTO script (workspace_id, hash, path, content, language, kind, created_by, schema, summary, description, lock)
VALUES (
    'test-workspace', 1111111111, 'u/test-user/rerun_script',
    'export function main(name = "world") { return "hi " + name; }',
    'deno', 'script', 'test-user',
    '{"type":"object","properties":{"name":{"type":"string"}},"order":["name"],"required":[]}',
    '', '', ''
);

INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema, extra_perms, versions)
VALUES (
    'test-workspace', 'u/test-user/rerun_flow',
    '', '',
    '{"modules":[{"id":"a","value":{"type":"rawscript","language":"deno","content":"export function main(name = \"world\"){ return \"flow:\" + name; }","input_transforms":{"name":{"type":"javascript","expr":"flow_input.name"}}}}]}',
    'test-user', NOW(),
    '{"type":"object","properties":{"name":{"type":"string"}},"order":["name"],"required":[]}',
    '{}',
    ARRAY[2222222222::bigint]
);

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by, created_at)
VALUES (
    2222222222, 'test-workspace', 'u/test-user/rerun_flow',
    '{"modules":[{"id":"a","value":{"type":"rawscript","language":"deno","content":"export function main(name = \"world\"){ return \"flow:\" + name; }","input_transforms":{"name":{"type":"javascript","expr":"flow_input.name"}}}}]}',
    '{"type":"object","properties":{"name":{"type":"string"}},"order":["name"],"required":[]}',
    'test-user', NOW()
);
