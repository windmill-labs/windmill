-- Add up migration script here
-- Emit a notify_event so every server evicts its cached `permissioned_as` -> address mapping
-- (windmill-common EMAIL_CACHE). That address is derived at job dispatch and feeds the
-- instance-superadmin check and `email_to_igroup`, so a replica serving a stale one runs jobs
-- with the wrong authorization until the TTL expires. SECURITY DEFINER so the INSERT runs as the
-- function owner rather than the invoking windmill_user/windmill_admin role, matching the other
-- notify_* triggers.
CREATE OR REPLACE FUNCTION notify_usr_email_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload)
    VALUES (
        'notify_user_email_change',
        COALESCE(NEW.workspace_id, OLD.workspace_id) || ':' || COALESCE(NEW.username, OLD.username)
    );
    -- A rename leaves the OLD username cached against this account's address; evict both keys.
    IF TG_OP = 'UPDATE' AND NEW.username IS DISTINCT FROM OLD.username THEN
        INSERT INTO notify_event (channel, payload)
        VALUES ('notify_user_email_change', OLD.workspace_id || ':' || OLD.username);
    END IF;
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

-- INSERT matters too: a lookup that resolved to nobody is cached as the synthetic
-- `{username}@unknown.windmill.dev`, so creating the row has to drop that entry.
CREATE TRIGGER usr_email_change_trigger
AFTER INSERT OR DELETE ON usr
FOR EACH ROW
EXECUTE FUNCTION notify_usr_email_change();

CREATE TRIGGER usr_email_update_trigger
AFTER UPDATE OF email, username ON usr
FOR EACH ROW
WHEN (OLD.email IS DISTINCT FROM NEW.email OR OLD.username IS DISTINCT FROM NEW.username)
EXECUTE FUNCTION notify_usr_email_change();

-- A superadmin acting outside their workspaces resolves through `password` instead, and that
-- row names no workspace, so there is no key to target: clear the whole cache. The empty payload
-- is the wildcard. Confined to superadmins because they are the only accounts the `usr` triggers
-- above cannot cover.
CREATE OR REPLACE FUNCTION notify_superadmin_email_change()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO notify_event (channel, payload) VALUES ('notify_user_email_change', '');
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

CREATE TRIGGER password_superadmin_email_change_trigger
AFTER UPDATE OF email, username ON password
FOR EACH ROW
WHEN (NEW.super_admin
      AND (OLD.email IS DISTINCT FROM NEW.email OR OLD.username IS DISTINCT FROM NEW.username))
EXECUTE FUNCTION notify_superadmin_email_change();
