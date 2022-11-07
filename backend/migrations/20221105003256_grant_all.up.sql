-- Add up migration script here


GRANT ALL
ON ALL TABLES IN SCHEMA public 
TO windmill_user;


GRANT ALL
ON ALL TABLES IN SCHEMA public 
TO windmill_admin;

DELETE FROM workspace_invite WHERE workspace_id = 'demo' AND email = 'ruben@windmill.dev';