CREATE ROLE windmill_user;

CREATE ROLE windmill_admin WITH BYPASSRLS;
GRANT windmill_user TO windmill_admin;

-- USAGE is granted to PUBLIC on the public schema by default, so this is a
-- no-op on a stock database. It matters on hardened ones that revoke it: the
-- roles then cannot resolve any table and every query on a user_db transaction
-- fails with "relation does not exist" rather than a permission error.
GRANT USAGE ON SCHEMA public TO windmill_user, windmill_admin;

GRANT ALL
ON ALL TABLES IN SCHEMA public
TO windmill_user;

GRANT ALL PRIVILEGES
ON ALL SEQUENCES IN SCHEMA public
TO windmill_user;

ALTER DEFAULT PRIVILEGES
    IN SCHEMA public
    GRANT ALL ON TABLES TO windmill_user;

ALTER DEFAULT PRIVILEGES
    IN SCHEMA public
    GRANT ALL ON SEQUENCES TO windmill_user;
