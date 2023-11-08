-- Add up migration script here

CREATE FUNCTION "notify_global_setting_delete" ()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM pg_notify('notify_global_setting_change', OLD.name::text);
    RETURN OLD;
END;
$$ LANGUAGE PLPGSQL;
  
CREATE OR REPLACE TRIGGER "notify_global_setting_delete"
 AFTER DELETE ON "global_settings"
    FOR EACH ROW
EXECUTE FUNCTION "notify_global_setting_delete" ();

