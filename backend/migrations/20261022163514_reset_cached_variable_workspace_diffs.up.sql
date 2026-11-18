-- Secrets compared by ciphertext were cached as changed even when both sides held the same value,
-- and a cached verdict is trusted until the next write. Re-evaluate every variable row once.
UPDATE workspace_diff SET has_changes = NULL WHERE kind = 'variable';
