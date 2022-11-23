-- used for backend automated testing
-- https://docs.rs/sqlx/latest/sqlx/attr.test.html

INSERT INTO workspace
            (id,               name,             owner,       domain)
     VALUES ('test-workspace', 'test-workspace', 'test-user', null);

INSERT INTO usr(workspace_id, email, username, is_admin, role) VALUES
	('test-workspace', 'test@windmill.dev', 'test-user', true, 'Admin');

INSERT INTO workspace_key(workspace_id, kind, key) VALUES
	('test-workspace', 'cloud', 'test-key');

insert INTO token(token, email, label, super_admin) VALUES ('SECRET_TOKEN', 'test@windmill.dev', 'test token', true);

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