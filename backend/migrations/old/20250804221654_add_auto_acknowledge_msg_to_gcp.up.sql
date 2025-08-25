-- Add up migration script here
ALTER table gcp_trigger ADD COLUMN auto_acknowledge_msg BOOLEAN DEFAULT true;