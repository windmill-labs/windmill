-- Add up migration script here
UPDATE password SET username = 'admin' WHERE email = 'admin@windmill.dev';