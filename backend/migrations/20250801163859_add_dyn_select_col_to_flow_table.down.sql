-- Add down migration script here
ALTER TABLE flow DROP COLUMN dyn_select_code;
ALTER TABLE flow DROP COLUMN dyn_select_lang;
DROP TYPE IF EXISTS DYN_SELECT_LANG;