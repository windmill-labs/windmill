ALTER TABLE kafka_trigger ADD COLUMN run_on_filter_error BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE websocket_trigger ADD COLUMN run_on_filter_error BOOLEAN NOT NULL DEFAULT false;
