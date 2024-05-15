-- Add up migration script here
UPDATE config SET name = 'worker__default_tmp' WHERE name = 'worker__default';