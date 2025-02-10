ALTER TABLE http_trigger DROP COLUMN is_static_website;

CREATE OR REPLACE FUNCTION prevent_route_path_change()
RETURNS TRIGGER AS $$
BEGIN
    IF CURRENT_USER <> 'windmill_admin' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;