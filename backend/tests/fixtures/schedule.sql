INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
export async function main(fail: boolean = true) {
  if (fail) {
    throw new Error("Failed")
  }

  return "OK"
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"fail":{"default":true,"description":"","type":"boolean"}},"required":[],"type":"object"}',
'',
'',
'f/system/failing_script', -28028598712388162, 'deno', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
export async function main() {
  return "Error handler";
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"path":{"default":null,"description":"","type":"string"},"schedule_path":{"default":null,"description":"","type":"string"},"error":{"default":null,"description":"","properties":{},"type":"object"}},"required":["path","schedule_path","error"],"type":"object"}',
'',
'',
'f/system/schedule_error_handler', -28028598712388161, 'deno', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'system',
'
export async function main() {
  return "Recovery handler";
}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"path":{"default":null,"description":"","type":"string"},"schedule_path":{"default":null,"description":"","type":"string"},"previous_job_error":{"default":null,"description":"","type":"string"},"result":{"default":null,"description":"","type":"string"}},"required":["path","schedule_path","previous_job_error","result"],"type":"object"}',
'',
'',
'f/system/schedule_recovery_handler', -28028598712388160, 'deno', '');

INSERT INTO public.flow(workspace_id, edited_by, value, schema, summary, description, path) VALUES (
'test-workspace',
'system',
'{"modules": [{"id": "a", "value": {"path": "f/system/failing_script", "type": "script", "input_transforms": {"fail": {"expr": "flow_input.fail", "type": "javascript"}}}}]}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"fail":{"default":true,"description":"","type":"boolean","format":""}},"required":[],"type":"object"}',
'',
'',
'f/system/failing_flow'
);