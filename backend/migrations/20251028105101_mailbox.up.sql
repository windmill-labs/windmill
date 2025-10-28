-- Add up migration script here
CREATE SEQUENCE IF NOT EXISTS mailbox_id_seq;

CREATE TYPE mailbox_type AS ENUM (
    'trigger',
    'debouncing_stale_data'
);

CREATE TABLE mailbox(
    message_id BIGINT DEFAULT nextval('mailbox_id_seq'),
    mailbox_id TEXT, 
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(), -- Indicates position in stack
    type mailbox_type NOT NULL, -- Type of mailbox
    payload JSONB NOT NULL, -- Payload of specific message
    PRIMARY KEY (message_id)
);
 

