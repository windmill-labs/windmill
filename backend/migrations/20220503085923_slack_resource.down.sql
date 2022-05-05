-- Add down migration script here
DELETE FROM resource_type WHERE name = 'slack' AND workspace_id = 'starter'; 
