ALTER TABLE http_trigger ADD COLUMN is_static_website BOOLEAN NOT NULL DEFAULT FALSE;


CREATE OR REPLACE FUNCTION prevent_route_path_change()
RETURNS TRIGGER AS $$
BEGIN
    IF CURRENT_USER = 'windmill_user' AND NEW.route_path <> OLD.route_path THEN
        RAISE EXCEPTION 'Modification of route_path is only allowed by admins';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;