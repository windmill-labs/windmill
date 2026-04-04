-- Basic leaf scripts in different languages
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
export async function main() {
    return "TypeScript leaf";
}',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/leafs/ts', 500001, 'bun', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    return "Python leaf"',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/leafs/python', 500003, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
<?php
function main() {
    return "PHP leaf";
}
?>',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/leafs/php', 500004, 'php', '');

-- Link scripts to named workspace dependencies (name: "test")
INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id) VALUES
    ('test-workspace', 'f/leafs/ts', 'script', 'dependencies/test.package.json', ''),
    ('test-workspace', 'f/leafs/python', 'script', 'dependencies/test.requirements.in', ''),
    ('test-workspace', 'f/leafs/php', 'script', 'dependencies/test.composer.json', '');

