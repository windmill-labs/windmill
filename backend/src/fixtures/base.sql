-- used for backend automated testing
-- https://docs.rs/sqlx/latest/sqlx/attr.test.html

INSERT INTO workspace
            (id,               name,             owner,       domain)
     VALUES ('test-workspace', 'test-workspace', 'test-user', null);

CREATE FUNCTION "notify_insert_on_completed_job" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('insert on completed_job', NEW.id::text);
    RETURN new;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_insert_on_completed_job"
 AFTER INSERT ON "completed_job"
    FOR EACH ROW
EXECUTE FUNCTION "notify_insert_on_completed_job" ();
