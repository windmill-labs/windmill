-- Add up migration script here
CREATE TABLE metrics (
  id VARCHAR(255) NOT NULL,
  value JSONB NOT NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
);
CREATE INDEX metrics_key_idx ON metrics(id);
CREATE INDEX metrics_sort_idx ON metrics(created_at DESC);