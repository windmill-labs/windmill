-- Add up migration script here
ALTER TABLE queue ADD COLUMN email VARCHAR(50) NOT NULL DEFAULT 'missing@email.xyz'; 
ALTER TABLE workspace_settings ADD COLUMN slack_email VARCHAR(50) NOT NULL DEFAULT 'missing@email.xyz';;
ALTER TABLE schedule ADD COLUMN email VARCHAR(50) NOT NULL DEFAULT 'missing@email.xyz';  
ALTER TABLE schedule ADD COLUMN error TEXT;  

