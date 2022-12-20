-- Add up migration script here
ALTER TABLE app_version
RENAME COLUMN flow_id TO app_id;

CREATE TABLE folder (
    name VARCHAR(255),
    workspace_id VARCHAR(50) NOT NULL,
    display_name VARCHAR(100) NOT NULL,
    owners VARCHAR(255)[] NOT NULL,
    extra_perms JSONB NOT NULL DEFAULT '{}',
    FOREIGN KEY (workspace_id) REFERENCES workspace(id) ON DELETE CASCADE,
    PRIMARY KEY (workspace_id, name)
);

CREATE INDEX folder_extra_perms ON folder USING GIN (extra_perms);
CREATE INDEX folder_owners ON folder USING GIN (owners);

GRANT ALL ON folder TO windmill_user;
GRANT ALL ON folder TO windmill_admin;
