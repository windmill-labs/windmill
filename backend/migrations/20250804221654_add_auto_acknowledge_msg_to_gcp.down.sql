-- Add down migration script here
ALTER table gcp_trigger DROP COLUMN auto_acknowledge_msg;
