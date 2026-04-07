-- Fixtures for dedicated worker E2E tests

-- A simple Bun script for testing dedicated Script steps in flows
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, dedicated_worker) VALUES (
'test-workspace',
'system',
E'export function main(x: number) { return x * 2; }',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'', '',
'f/system/dedicated_double', 300001, 'bun', E'{}\n//bun.lock\n<empty>', true);

-- Flow with a single RawScript inline bun step (the "Script not found" bug case)
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_rawscript_flow',
'{3000000000000001}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 10; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000001,
'test-workspace',
'f/system/dedicated_rawscript_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 10; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- Flow with a Script step referencing the external dedicated_double script
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_script_flow',
'{3000000000000002}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'{"modules":[{"id":"a","value":{"type":"script","path":"f/system/dedicated_double","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000002,
'test-workspace',
'f/system/dedicated_script_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'{"modules":[{"id":"a","value":{"type":"script","path":"f/system/dedicated_double","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

-- Flow with two inline RawScript steps (tests multi-step key uniqueness)
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_multi_step_flow',
'{3000000000000003}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 1; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}},{"id":"b","value":{"type":"rawscript","content":"export function main(x: number) { return x * 3; }","language":"bun","input_transforms":{"x":{"expr":"results.a","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000003,
'test-workspace',
'f/system/dedicated_multi_step_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 1; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}},{"id":"b","value":{"type":"rawscript","content":"export function main(x: number) { return x * 3; }","language":"bun","input_transforms":{"x":{"expr":"results.a","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- Two scripts sharing a workspace dependency for runner group testing.
-- Both reference the same external dep "f/system/dedicated_double" via extra_package_json annotation.
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, dedicated_worker) VALUES (
'test-workspace',
'system',
E'// extra_package_json: f/system/dedicated_double\nexport function main(x: number) { return x + 100; }',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'', '',
'f/system/rg_script_a', 300010, 'bun', E'{}\n//bun.lock\n<empty>', true);

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, dedicated_worker) VALUES (
'test-workspace',
'system',
E'// extra_package_json: f/system/dedicated_double\nexport function main(x: number) { return x + 200; }',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'', '',
'f/system/rg_script_b', 300011, 'bun', E'{}\n//bun.lock\n<empty>', true);

-- Flow with a Deno inline RawScript step
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_deno_flow',
'{3000000000000005}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 100; }","language":"deno","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000005,
'test-workspace',
'f/system/dedicated_deno_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 100; }","language":"deno","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

-- Flow with a Python inline RawScript step
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_python_flow',
'{3000000000000006}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"def main(x: int):\\n    return x + 100","language":"python3","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000006,
'test-workspace',
'f/system/dedicated_python_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"def main(x: int):\\n    return x + 100","language":"python3","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}}}}]}',
'system'
);

-- Flow with a Bunnative (//native) inline RawScript step
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_bunnative_flow',
'{3000000000000007}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"//native\\nexport function main(x: number) { return x + 100; }","language":"bunnative","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000007,
'test-workspace',
'f/system/dedicated_bunnative_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"//native\\nexport function main(x: number) { return x + 100; }","language":"bunnative","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- Flow with a Bun + //nodejs annotation inline RawScript step
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_nodejs_flow',
'{3000000000000008}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"//nodejs\\nexport function main(x: number) { return x + 100; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000008,
'test-workspace',
'f/system/dedicated_nodejs_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"//nodejs\\nexport function main(x: number) { return x + 100; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- Flow with a squashed for-loop for testing flow runners.
-- The for-loop iterates over [1, 2, 3], each iteration runs a simple bun rawscript.
-- squash=true triggers spawn_flow_module_runners to create dedicated subprocesses.
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/dedicated_flow_runners',
'{3000000000000004}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"forloopflow","iterator":{"type":"javascript","expr":"[1, 2, 3]"},"skip_failures":false,"parallel":false,"squash":true,"modules":[{"id":"b","value":{"type":"rawscript","content":"export function main(iter: {value: number, index: number}) { return iter.value * 10; }","language":"bun","input_transforms":{"iter":{"expr":"flow_input.iter","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000004,
'test-workspace',
'f/system/dedicated_flow_runners',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"forloopflow","iterator":{"type":"javascript","expr":"[1, 2, 3]"},"skip_failures":false,"parallel":false,"squash":true,"modules":[{"id":"b","value":{"type":"rawscript","content":"export function main(iter: {value: number, index: number}) { return iter.value * 10; }","language":"bun","input_transforms":{"iter":{"expr":"flow_input.iter","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}}]}',
'system'
);

-- Two flows with conflicting step IDs (both have module "a") but different RawScript content.
-- Tests that runnable_path-based lookup correctly disambiguates them.
-- Flow A: x + 1000
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/conflict_flow_a',
'{3000000000000009}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 1000; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000009,
'test-workspace',
'f/system/conflict_flow_a',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 1000; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- Flow B: x + 2000
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace', '', '',
'f/system/conflict_flow_b',
'{3000000000000010}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 2000; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
3000000000000010,
'test-workspace',
'f/system/conflict_flow_b',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"rawscript","content":"export function main(x: number) { return x + 2000; }","language":"bun","input_transforms":{"x":{"expr":"flow_input.x","type":"javascript"}},"lock":"{}\\n//bun.lock\\n<empty>"}}]}',
'system'
);

-- A dedicated worker script with a preprocessor function.
-- preprocessor doubles x, main adds 100.
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, dedicated_worker) VALUES (
'test-workspace',
'system',
E'export function preprocessor(x: number) { return { x: x * 2 }; }\nexport function main(x: number) { return x + 100; }',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'', '',
'f/system/preprocess_script', 300030, 'bun', E'{}\n//bun.lock\n<empty>', true);

-- Second workspace for cross-workspace isolation testing.
INSERT INTO workspace(id, name, owner) VALUES ('test-workspace-2', 'test-workspace-2', 'test-user');
INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace-2', 'test@windmill.dev', 'test-user', true, 'Admin');
INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace-2', 'cloud', 'test-key-2');
INSERT INTO workspace_settings (workspace_id) VALUES ('test-workspace-2');
INSERT INTO group_ (workspace_id, name, summary, extra_perms) VALUES
	('test-workspace-2', 'all', 'All users', '{}');

-- Same path as dedicated_double but in workspace 2, returns x * 3 instead of x * 2.
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, dedicated_worker) VALUES (
'test-workspace-2',
'system',
E'export function main(x: number) { return x * 3; }',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"x":{"type":"number","description":""}},"required":["x"],"type":"object"}',
'', '',
'f/system/dedicated_double', 300040, 'bun', E'{}\n//bun.lock\n<empty>', true);

