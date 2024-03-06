-- Add up migration script here
CREATE TABLE custom_concurrency_key_ended (
  key VARCHAR(255) NOT NULL,
  ended_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
  PRIMARY KEY (key, ended_at)
);
