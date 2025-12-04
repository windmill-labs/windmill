-- Add down migration script here
DROP TABLE IF EXISTS mailbox;
DROP TYPE IF EXISTS mailbox_type;
DROP SEQUENCE IF EXISTS mailbox_id_seq;
DROP INDEX IF EXISTS idx_mailbox_type_mailbox_id_message_id;
