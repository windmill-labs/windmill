-- Add up migration script here
ALTER TYPE draft_kind ADD VALUE IF NOT EXISTS 'trigger_amqp';
