-- Add up migration script here
alter table capture DROP constraint caputre_payload_too_big;
alter table capture add constraint capture_payload_too_big check (length(payload::text) < 512 * 1024);