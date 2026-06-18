-- Add up migration script here

GRANT SELECT, UPDATE ON debounce_key TO windmill_user;
GRANT SELECT, UPDATE ON debounce_key TO windmill_admin;
