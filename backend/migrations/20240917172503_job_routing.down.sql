-- Add down migration script here
DROP TABLE http_trigger;
DROP TYPE http_method;

ALTER TABLE script DROP COLUMN has_preprocessor;

DROP FUNCTION prevent_route_path_change(); 