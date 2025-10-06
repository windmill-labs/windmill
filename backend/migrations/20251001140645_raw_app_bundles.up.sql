-- Add up migration script here
CREATE TABLE app_bundles (
    app_version_id BIGINT NOT NULL,
    w_id VARCHAR(255) NOT NULL,
    file_type VARCHAR(10) NOT NULL,
    data BYTEA NOT NULL,
    PRIMARY KEY (app_version_id, file_type)
);
