INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'# py312
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/multipython/aliases', 2468135790, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'# py: >=3.9,!=3.12.2
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/multipython/script1', 2345678901, 'python3', '');

