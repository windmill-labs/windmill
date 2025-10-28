-- Add down migration script here
DROP TABLE IF EXISTS mailbox;
DROP TYPE IF EXISTS mailbox_type;
DROP SEQUENCE IF EXISTS mailbox_id_seq;
