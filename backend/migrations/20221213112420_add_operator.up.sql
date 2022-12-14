-- Add up migration script here
ALTER TABLE workspace_invite ADD COLUMN operator BOOLEAN NOT NULL DEFAULT false;
ALTER TABLE workspace_settings ADD COLUMN auto_invite_operator BOOLEAN DEFAULT false;
ALTER TABLE completed_job ADD COLUMN email VARCHAR(50) NOT NULL DEFAULT 'missing@email.xyz'; 
