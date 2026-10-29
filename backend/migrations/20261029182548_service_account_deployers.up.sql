-- Who may deploy items that run as this service account: u/<user>, g/<group>, or f/<folder>
-- (whoever can write in that folder at deploy time). Admins and wm_deployers need no entry.
ALTER TABLE usr ADD COLUMN deployers TEXT[] NOT NULL DEFAULT '{}';

-- An entry hands over a whole account, so a user, group or folder created later under a freed
-- name must not inherit it. Triggers keep the lists in step on every path that deletes or
-- renames one, inside that path's own transaction.
CREATE FUNCTION usr_deployers_forget() RETURNS trigger AS $$
DECLARE
    entry TEXT;
BEGIN
    -- IF, not CASE: a CASE resolves every branch's field, and a usr row has no `name`.
    IF TG_TABLE_NAME = 'usr' THEN
        entry := 'u/' || OLD.username;
    ELSIF TG_TABLE_NAME = 'group_' THEN
        entry := 'g/' || OLD.name;
    ELSE
        entry := 'f/' || OLD.name;
    END IF;
    UPDATE usr SET deployers = array_remove(deployers, entry)
    WHERE workspace_id = OLD.workspace_id AND entry = ANY(deployers);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER usr_deployers_forget_user AFTER DELETE ON usr
    FOR EACH ROW EXECUTE FUNCTION usr_deployers_forget();
CREATE TRIGGER usr_deployers_forget_group AFTER DELETE ON group_
    FOR EACH ROW EXECUTE FUNCTION usr_deployers_forget();
CREATE TRIGGER usr_deployers_forget_folder AFTER DELETE ON folder
    FOR EACH ROW EXECUTE FUNCTION usr_deployers_forget();

CREATE FUNCTION usr_deployers_rename() RETURNS trigger AS $$
BEGIN
    UPDATE usr SET deployers = array_replace(deployers, 'u/' || OLD.username, 'u/' || NEW.username)
    WHERE workspace_id = NEW.workspace_id AND ('u/' || OLD.username) = ANY(deployers);
    RETURN NULL;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER usr_deployers_rename_user AFTER UPDATE OF username ON usr
    FOR EACH ROW WHEN (OLD.username IS DISTINCT FROM NEW.username)
    EXECUTE FUNCTION usr_deployers_rename();
