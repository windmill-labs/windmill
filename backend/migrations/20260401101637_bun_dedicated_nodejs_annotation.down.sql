-- Remove the //nodejs annotation that was prepended by the up migration.
-- Only removes it if it's at the very start of the content.
UPDATE script
SET content = regexp_replace(content, '^//nodejs\n// dedicated workers were previously running in nodejs mode by default, remove this annotation to use bun\n', '')
WHERE language = 'bun'
  AND dedicated_worker = true;
