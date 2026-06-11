UPDATE app
SET policy = policy - 'legacy_unsandboxed'
WHERE policy ? 'legacy_unsandboxed';
