-- Add up migration script here
DO $$
DECLARE
    rec RECORD;
    view_definitions TEXT[];
    view_names TEXT[];
BEGIN
    -- Step 1: Store view definitions
    FOR rec IN (
        SELECT c.relname AS view_name, pg_get_viewdef(c.oid, true) AS view_sql
        FROM pg_class c JOIN pg_namespace n ON c.relnamespace = n.oid
        WHERE c.relkind = 'v' AND c.relname IN ('job', 'v2_as_queue', 'v2_as_completed_job')
    ) LOOP
        -- Save view names and definitions
        view_names := array_append(view_names, rec.view_name);
        view_definitions := array_append(view_definitions, rec.view_sql);
    END LOOP;

    -- Step 2: Drop the views
    FOR i IN 1..array_length(view_names, 1) LOOP
        EXECUTE format('DROP VIEW IF EXISTS %I', view_names[i]);
    END LOOP;

    -- Step 3: Alter the table column type
    ALTER TABLE v2_job ALTER COLUMN tag TYPE VARCHAR(255);
    ALTER TABLE flow ALTER COLUMN tag TYPE VARCHAR(255);
    ALTER TABLE schedule ALTER COLUMN tag TYPE VARCHAR(255);
    ALTER TABLE script ALTER COLUMN tag TYPE VARCHAR(255);

    -- Step 4: Recreate the views using stored definitions
    FOR i IN 1..array_length(view_names, 1) LOOP
        EXECUTE format('CREATE OR REPLACE VIEW %I AS %s', view_names[i], view_definitions[i]);
    END LOOP;
END $$;
