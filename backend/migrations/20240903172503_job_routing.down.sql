-- Add down migration script here
DROP TABLE trigger;
DROP TYPE trigger_kind;
DROP TYPE http_method;

ALTER TABLE script DROP COLUMN has_preprocessor;

DROP FUNCTION prevent_route_path_change();