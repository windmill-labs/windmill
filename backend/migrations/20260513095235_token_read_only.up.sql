-- Add a flag to restrict a token to read-only HTTP endpoints.
-- Orthogonal to `scopes`: even if scopes grant write/run, this flag denies
-- mutating methods (POST/PUT/PATCH/DELETE) and Run actions.
ALTER TABLE token ADD COLUMN read_only BOOLEAN NOT NULL DEFAULT false;
