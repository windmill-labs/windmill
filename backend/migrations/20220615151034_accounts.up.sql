-- Add up migration script here

CREATE TABLE account (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    id SERIAL NOT NULL,
    expires_at TIMESTAMP,
    refresh_token VARCHAR(255),
    PRIMARY KEY (workspace_id, id)
);

ALTER TABLE resource ADD COLUMN account INTEGER;
ALTER TABLE variable ADD COLUMN account INTEGER;
ALTER TABLE password ALTER COLUMN login_type TYPE VARCHAR(50);
