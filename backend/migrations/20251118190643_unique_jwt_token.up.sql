-- Add up migration script here

CREATE TABLE IF NOT EXISTS unique_ext_jwt_token (
    jwt_hash BIGINT PRIMARY KEY NOT NULL,
    last_used_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_unique_ext_jwt_token_last_used_at ON unique_ext_jwt_token(last_used_at);

