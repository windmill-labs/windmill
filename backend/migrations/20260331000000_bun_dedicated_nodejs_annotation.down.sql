-- Remove the //nodejs annotation that was prepended by the up migration.
-- Only removes it if it's at the very start of the content.
UPDATE script
SET content = SUBSTRING(content FROM 10)
WHERE language = 'bun'
  AND dedicated_worker = true
  AND content LIKE '//nodejs
%';
