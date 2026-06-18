-- Bun dedicated workers were previously forced to run in nodejs mode at runtime.
-- Now that bun is the default again, add the //nodejs annotation to existing
-- bun dedicated scripts that don't already have it, so their behavior is preserved.
UPDATE script
SET content = '//nodejs
// dedicated workers were previously running in nodejs mode by default, remove this annotation to use bun
' || content
WHERE language = 'bun'
  AND dedicated_worker = true
  AND content !~ '^//\s*nodejs';
