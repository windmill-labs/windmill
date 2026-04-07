-- Remove the //nodejs annotation that was prepended by the up migration.
-- Only removes it if it's at the very start of the content.
UPDATE script
SET content = REPLACE(content, '//nodejs
// dedicated workers were previously running in nodejs mode by default, remove this annotation to use bun
', '')
WHERE language = 'bun'
  AND dedicated_worker = true;
