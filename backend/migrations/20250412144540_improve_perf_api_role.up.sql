-- Add up migration script here
 CREATE OR REPLACE FUNCTION set_session_context(
    admin BOOLEAN,
    username TEXT,
    groups TEXT,
    pgroups TEXT,
    folders_read TEXT,
    folders_write TEXT
) RETURNS void AS $$
BEGIN
    IF admin THEN
        SET LOCAL ROLE windmill_admin;
    ELSE
        SET LOCAL ROLE windmill_user;
    END IF;
    PERFORM set_config('session.user', username, true);
    PERFORM set_config('session.groups', groups, true);
    PERFORM set_config('session.pgroups', pgroups, true);
    PERFORM set_config('session.folders_read', folders_read, true);
    PERFORM set_config('session.folders_write', folders_write, true);
END;
$$ LANGUAGE plpgsql;