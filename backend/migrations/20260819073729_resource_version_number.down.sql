-- Back to numbering versions by the table-wide identity sequence, so the function must stop
-- writing a column that is about to go.
CREATE OR REPLACE FUNCTION record_resource_version() RETURNS trigger AS $$
BEGIN
    INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by)
    VALUES (
        NEW.workspace_id, NEW.path, NEW.resource_type, NEW.value,
        COALESCE(NULLIF(current_setting('session.user', true), ''), NEW.created_by)
    );

    RETURN NEW;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT;

DROP INDEX IF EXISTS index_resource_version_number;

ALTER TABLE resource_version DROP COLUMN version;
