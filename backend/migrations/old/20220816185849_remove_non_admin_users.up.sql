-- Add up migration script here
DELETE FROM password WHERE email = 'user@windmill.dev' OR email = 'ruben@windmill.dev';
