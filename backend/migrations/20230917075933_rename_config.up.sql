-- Add up migration script here
ALTER TABLE IF exists worker_group_config RENAME TO config;
UPDATE config SET name = 'worker__' || name;

CREATE FUNCTION "notify_config_change" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_config_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_config_change"
 AFTER INSERT OR UPDATE ON "config"
    FOR EACH ROW
EXECUTE FUNCTION "notify_config_change" ();

CREATE FUNCTION "notify_global_setting_change" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', NEW.name::text);
    RETURN NEW;
END;
$$ LANGUAGE PLPGSQL;

  CREATE TRIGGER "notify_global_setting_change"
 AFTER INSERT OR UPDATE ON "global_settings"
    FOR EACH ROW
EXECUTE FUNCTION "notify_global_setting_change" ();
