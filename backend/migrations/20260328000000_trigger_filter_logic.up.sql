ALTER TABLE kafka_trigger ADD COLUMN filter_logic VARCHAR(3) NOT NULL DEFAULT 'and';
ALTER TABLE websocket_trigger ADD COLUMN filter_logic VARCHAR(3) NOT NULL DEFAULT 'and';
