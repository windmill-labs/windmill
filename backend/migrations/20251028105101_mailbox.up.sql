-- Add up migration script here
CREATE SEQUENCE IF NOT EXISTS mailbox_id_seq;

CREATE TYPE mailbox_type AS ENUM (
    'trigger',
    'debouncing_stale_data'
);

CREATE TABLE mailbox(
    message_id   BIGINT DEFAULT nextval('mailbox_id_seq') PRIMARY KEY, -- Also indicates position in stack
    mailbox_id   TEXT, -- Can be NULL 
    workspace_id character varying(50) NOT NULL,
    type         mailbox_type NOT NULL, -- Type of mailbox
    created_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(), 
    payload      JSONB NOT NULL -- Payload of specific message
);
 
CREATE INDEX idx_mailbox_type_mailbox_id_message_id 
ON mailbox(type, mailbox_id, message_id ASC);
