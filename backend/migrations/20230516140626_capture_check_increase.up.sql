-- Add up migration script here
alter table capture DROP constraint capture_payload_check;
alter table capture add constraint caputre_payload_too_big check  (length(payload::text) < 512 * 1024);
