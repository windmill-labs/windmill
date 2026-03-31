-- Bun dedicated workers were previously forced to run in nodejs mode at runtime.
-- Now that bun is the default again, add the //nodejs annotation to existing
-- bun dedicated scripts that don't already have it (and aren't //native),
-- so their behavior is preserved.
UPDATE script
SET content = '//nodejs
' || content
WHERE language = 'bun'
  AND dedicated_worker = true
  AND content NOT LIKE '%//nodejs%'
  AND content NOT LIKE '%//native%';
