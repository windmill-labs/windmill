INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
export function main(world: string) {
    const greet = `Hello ${world}!`;
    console.log(greet)
    return greet
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"default":"world","description":"","type":"string"}},"required":[],"type":"object"}',
'',
'',
'f/system/hello', 123412, 'deno', '');

INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace',
'',
'',
'f/system/hello_flow',
'{1443253234253453}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"default":"world","description":"","type":"string"}},"required":[],"type":"object"}',
'{"modules": [{"id": "a", "value": {"path": "f/system/hello", "type": "script", "input_transforms": {"world": {"expr": "flow_input.world", "type": "javascript"}}}}]}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
1443253234253453,
'test-workspace',
'f/system/hello_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"default":"world","description":"","type":"string"}},"required":[],"type":"object"}',
'{"modules": [{"id": "a", "value": {"path": "f/system/failing_script", "type": "script", "input_transforms": {"fail": {"expr": "flow_input.fail", "type": "javascript"}}}}]}',
'system'
);

INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace',
'',
'',
'f/system/hello_with_nodes_flow',
'{1443253234253454}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"default":"world","description":"","type":"string"}},"required":[],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"forloopflow","modules":[{"id":"b","value":{"type":"rawscript","content":"export function main(world: string) {\\n    const greet = `Hello ${world}!`;\\n    console.log(greet)\\n    return greet\\n}\\n","language":"deno","input_transforms":{"world":{"type":"javascript","expr":"flow_input.iter.value"}},"is_trigger":false}},{"id":"c","value":{"type":"rawscript","content":"export function main(hello: string) {\\n    const dareyou = `Did you just say \\"${hello}\\"??!`;\\n    console.log(dareyou)\\n    return dareyou\\n}","language":"deno","input_transforms":{"hello":{"type":"javascript","value":"${results.b}","expr":"`${results.b}`"}},"is_trigger":false}}],"iterator":{"type":"javascript","expr":"[\'foo\', \'bar\', \'baz\']"},"skip_failures":true,"parallel":false}}],"same_worker":false}',
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
1443253234253454,
'test-workspace',
'f/system/hello_with_nodes_flow',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"world":{"default":"world","description":"","type":"string"}},"required":[],"type":"object"}',
E'{"modules":[{"id":"a","value":{"type":"forloopflow","modules":[{"id":"b","value":{"type":"rawscript","content":"export function main(world: string) {\\n    const greet = `Hello ${world}!`;\\n    console.log(greet)\\n    return greet\\n}\\n","language":"deno","input_transforms":{"world":{"type":"javascript","expr":"flow_input.iter.value"}},"is_trigger":false}},{"id":"c","value":{"type":"rawscript","content":"export function main(hello: string) {\\n    const dareyou = `Did you just say \\"${hello}\\"??!`;\\n    console.log(dareyou)\\n    return dareyou\\n}","language":"deno","input_transforms":{"hello":{"type":"javascript","value":"${results.b}","expr":"`${results.b}`"}},"is_trigger":false}}],"iterator":{"type":"javascript","expr":"[\'foo\', \'bar\', \'baz\']"},"skip_failures":true,"parallel":false}}],"same_worker":false}',
'system'
);
