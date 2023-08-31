-- used for backend automated testing
-- https://docs.rs/sqlx/latest/sqlx/attr.test.html

INSERT INTO workspace
            (id,               name,             owner)
     VALUES ('test-workspace', 'test-workspace', 'test-user');

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key');


INSERT INTO workspace_settings (workspace_id) VALUES
	('test-workspace');

insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true);

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

GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_admin;
GRANT ALL PRIVILEGES ON TABLE workspace_key TO windmill_user;

CREATE FUNCTION "notify_insert_on_completed_job" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('insert on completed_job', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_insert_on_completed_job"
 AFTER INSERT ON "completed_job"
    FOR EACH ROW
EXECUTE FUNCTION "notify_insert_on_completed_job" ();


CREATE FUNCTION "notify_queue" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('queue', NEW.id::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_queue_after_insert"
 AFTER INSERT ON "queue"
    FOR EACH ROW
EXECUTE FUNCTION "notify_queue" ();

  CREATE TRIGGER "notify_queue_after_flow_status_update"
 AFTER UPDATE ON "queue"
    FOR EACH ROW
            WHEN (NEW.flow_status IS DISTINCT FROM OLD.flow_status)
EXECUTE FUNCTION "notify_queue" ();