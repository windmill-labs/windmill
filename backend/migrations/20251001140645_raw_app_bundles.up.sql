-- Add up migration script here
CREATE TABLE raw_app_bundles (
    app_id BIGINT NOT NULL,
    w_id VARCHAR(255) NOT NULL,
    file_type VARCHAR(10) NOT NULL,
    data BYTEA NOT NULL,
    PRIMARY KEY (app_id, file_type)
);
