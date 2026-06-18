ALTER TABLE kafka_trigger ADD COLUMN auto_offset_reset VARCHAR(10) NOT NULL DEFAULT 'latest';
ALTER TABLE kafka_trigger ADD COLUMN reset_offset BOOLEAN NOT NULL DEFAULT FALSE;
