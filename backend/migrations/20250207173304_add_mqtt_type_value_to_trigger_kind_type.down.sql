-- Add down migration script here
DROP TYPE TRIGGER_KIND;
CREATE TYPE TRIGGER_KIND AS ENUM ('webhook', 'http', 'websocket', 'kafka', 'email');