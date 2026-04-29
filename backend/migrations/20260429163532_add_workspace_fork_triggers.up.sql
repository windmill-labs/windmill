-- Tracks whether triggers and schedules were cloned (always disabled) at fork
-- creation time. Read by the create_fork handler; reflected in the UI so users
-- can tell which forks carry the parent's trigger/schedule definitions.
ALTER TABLE workspace ADD COLUMN fork_triggers BOOLEAN NOT NULL DEFAULT FALSE;
