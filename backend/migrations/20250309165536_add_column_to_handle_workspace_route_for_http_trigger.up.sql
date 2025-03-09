-- Add up migration script here
ALTER TABLE http_trigger
ADD COLUMN full_route_path_key VARCHAR(305) DEFAULT NULL,
ADD COLUMN workspaced_route BOOLEAN NOT NULL DEFAULT false;