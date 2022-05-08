-- Add up migration script here
DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'app') THEN
        CREATE ROLE app LOGIN PASSWORD 'changeme';
    END IF;
END
$do$;

DO
$do$
BEGIN
    IF NOT EXISTS (
        SELECT
        FROM   pg_catalog.pg_roles
        WHERE  rolname = 'admin') THEN
        CREATE ROLE admin LOGIN PASSWORD 'changeme';
    END IF;
END
$do$;
