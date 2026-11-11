-- Items `/access/reads` follows as test-user-2: its own, one private to test-user, and a script
-- that runs as test-user.
INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin) VALUES
    (encode(sha256('READS_TOKEN'::bytea), 'hex'), 'READS_TOKE', 'READS_TOKEN', 'test2@windmill.dev',
     'reads', false);

INSERT INTO script (workspace_id, hash, path, content, language, created_by, created_at, schema,
                    summary, description, lock, archived, on_behalf_of) VALUES
    ('test-workspace', 1, 'u/test-user-2/versioned',
     'import * as wmill from "windmill-client"
export async function main() { return await wmill.getVariable("u/test-user-2/old_var") }',
     'bun', 'test-user-2', now() - interval '1 day', '{}', '', '', '', true, NULL),
    ('test-workspace', 2, 'u/test-user-2/versioned',
     'import * as wmill from "windmill-client"
export async function main() { return await wmill.getVariable("u/test-user-2/new_var") }',
     'bun', 'test-user-2', now(), '{}', '', '', '', false, NULL),
    ('test-workspace', 3, 'u/test-user-2/bot',
     'import * as wmill from "windmill-client"
export async function main(input: object) { return await wmill.getVariable("u/test-user-2/bot_token") }',
     'bun', 'test-user-2', now(), '{}', '', '', '', false, 'u/test-user'),
    ('test-workspace', 4, 'u/test-user/private', 'export async function main() {}',
     'bun', 'test-user', now(), '{}', '', '', '', false, NULL);

INSERT INTO resource (workspace_id, path, value, resource_type, extra_perms, created_by) VALUES
    ('test-workspace', 'u/test-user-2/bot_input', '{"password": "$var:u/test-user-2/linked"}',
     'object', '{}', 'test-user-2');

INSERT INTO flow (workspace_id, path, summary, description, value, edited_by, edited_at, schema,
                  extra_perms, versions) VALUES
    ('test-workspace', 'u/test-user-2/flow', '', '', '{}', 'test-user-2', now(), '{}', '{}', '{10}');

INSERT INTO flow_version (id, workspace_id, path, value, schema, created_by) VALUES
    (10, 'test-workspace', 'u/test-user-2/flow', '{"modules": [
        {"id": "a", "summary": "Pinned", "value": {"type": "script", "path": "u/test-user-2/versioned",
            "hash": "0000000000000001", "input_transforms": {}}},
        {"id": "b", "summary": "Inline", "value": {"type": "rawscript", "language": "bun",
            "content": "import * as wmill from \"windmill-client\"\nexport async function main() {\n  await wmill.setVariable(\"u/test-user-2/state\", \"x\")\n  await wmill.runScriptByPath(\"u/test-user-2/bot\", {})\n}",
            "input_transforms": {"token": {"type": "javascript", "expr": "variable(''u/test-user-2/expr_var'')"}}}},
        {"id": "c", "summary": "Bot", "value": {"type": "script", "path": "u/test-user-2/bot",
            "input_transforms": {"input": {"type": "static", "value": "$res:u/test-user-2/bot_input"}}}},
        {"id": "d", "value": {"type": "script", "path": "u/test-user/private", "input_transforms": {}}}
    ]}', '{}', 'test-user-2');

INSERT INTO app (id, workspace_id, path, summary, policy, versions, extra_perms) VALUES
    (20, 'test-workspace', 'u/test-user-2/app', '',
     '{"execution_mode": "publisher", "on_behalf_of": "u/test-user"}', '{20}', '{}');

INSERT INTO app_version (id, app_id, value, created_by, raw_app) VALUES
    (20, 20, '{
        "runnables": {
            "a": {"name": "a", "type": "inline", "inlineScript": {"language": "bun",
                "content": "import * as wmill from \"windmill-client\"\nexport async function main() { return await wmill.getVariable(\"u/test-user-2/runnable_var\") }"}},
            "b": {"name": "b", "type": "runnableByPath", "runType": "script", "path": "u/test-user-2/bot"}
        },
        "files": {
            "/index.tsx": "import * as wmill from \"windmill-client\"\nexport default function App() { wmill.getVariable(\"u/test-user-2/tsx_var\"); return <div /> }",
            "/Widget.svelte": "<script lang=\"ts\">import { getVariable } from \"windmill-client\"; const v = getVariable(\"u/test-user-2/svelte_var\")</script>\n<p>{v}</p>",
            "/notes.md": "Call getVariable(\"u/test-user-2/md_var\") to read it."
        }
    }', 'test-user-2', true);
