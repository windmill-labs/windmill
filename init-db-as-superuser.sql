CREATE ROLE windmill_user;

CREATE ROLE windmill_admin WITH BYPASSRLS;
GRANT windmill_user TO windmill_admin;

-- Since PostgreSQL 15, USAGE on public is no longer granted to PUBLIC. Without
-- it, SET LOCAL ROLE windmill_user/windmill_admin cannot resolve any table and
-- every query fails with "relation does not exist".
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
