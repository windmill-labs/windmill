-- Add up migration script here
CREATE TYPE DYN_SELECT_LANG AS ENUM('bun', 'python3');
ALTER TABLE flow ADD COLUMN dyn_select_code TEXT DEFAULT NULL;
ALTER TABLE flow ADD COLUMN dyn_select_lang DYN_SELECT_LANG DEFAULT NULL;
