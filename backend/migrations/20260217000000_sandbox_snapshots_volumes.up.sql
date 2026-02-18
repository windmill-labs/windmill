-- Add sandbox_snapshot and sandbox_volume tables

CREATE TABLE IF NOT EXISTS sandbox_snapshot (
    workspace_id  VARCHAR(50) NOT NULL REFERENCES workspace(id),
    name          VARCHAR(255) NOT NULL,
    tag           VARCHAR(255) NOT NULL DEFAULT 'latest',
    s3_key        TEXT NOT NULL,
    content_hash  VARCHAR(64) NOT NULL DEFAULT '',
    docker_image  TEXT NOT NULL,
    setup_script  TEXT,
    size_bytes    BIGINT,
    status        VARCHAR(20) NOT NULL DEFAULT 'pending',
    build_error   TEXT,
    build_job_id  UUID,
    created_by    VARCHAR(255) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    extra_perms   JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (workspace_id, name, tag)
);

CREATE TABLE IF NOT EXISTS sandbox_volume (
    workspace_id  VARCHAR(50) NOT NULL REFERENCES workspace(id),
    name          VARCHAR(255) NOT NULL,
    s3_key        TEXT NOT NULL,
    size_bytes    BIGINT,
    created_by    VARCHAR(255) NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    extra_perms   JSONB NOT NULL DEFAULT '{}'::jsonb,
    PRIMARY KEY (workspace_id, name)
);
