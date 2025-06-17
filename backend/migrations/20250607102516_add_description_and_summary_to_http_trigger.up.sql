-- Add up migration script here
ALTER TABLE http_trigger 
ADD COLUMN summary VARCHAR(512) NULL,
ADD COLUMN description TEXT NULL;

