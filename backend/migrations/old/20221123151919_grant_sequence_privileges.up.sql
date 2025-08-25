-- Add up migration script here

GRANT ALL
ON ALL SEQUENCES IN SCHEMA public 
TO windmill_user;
GRANT ALL
ON ALL SEQUENCES IN SCHEMA public 
TO windmill_admin;