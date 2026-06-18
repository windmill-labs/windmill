ALTER TABLE kafka_trigger ADD COLUMN filters JSONB[] NOT NULL DEFAULT '{}';
