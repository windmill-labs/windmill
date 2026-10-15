-- `account.created_by` grants the right to link the account to a variable, so it must name the
-- same person for as long as it is set: follow the user through a rename, and clear it when the
-- user leaves the workspace, so a later member given the same username does not inherit it.
-- A trigger rather than application code, because users are removed from several code paths.
CREATE OR REPLACE FUNCTION account_created_by_follows_user() RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        UPDATE account SET created_by = NULL
        WHERE workspace_id = OLD.workspace_id AND created_by = OLD.username;
    ELSIF NEW.username IS DISTINCT FROM OLD.username THEN
        UPDATE account SET created_by = NEW.username
        WHERE workspace_id = OLD.workspace_id AND created_by = OLD.username;
    END IF;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER account_created_by_follows_user_trigger
AFTER UPDATE OF username OR DELETE ON usr
FOR EACH ROW
EXECUTE FUNCTION account_created_by_follows_user();
