-- Add down migration script here
ALTER TABLE http_trigger
DROP COLUMN full_route_path_key,
DROP COLUMN workspaced_route;