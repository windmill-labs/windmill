-- Add up migration script here
INSERT INTO group_ SELECT id, 'all', 'The group that always contains all users of this workspace' FROM workspace ON CONFLICT DO NOTHING;
