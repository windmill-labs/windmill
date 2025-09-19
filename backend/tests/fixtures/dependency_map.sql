INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    return "f/rel/leaf_1"
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/rel/leaf_1', 333400, 'python3', '');
-- Padded Hex: 0000000000051658

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    return "f/rel/leaf_2"
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/rel/leaf_2', 333401, 'python3', '');
-- Padded Hex: 0000000000051659

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
from f.rel.leaf_1 import main as lf_1;

def main():
    return lf_1();
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/rel/branch', 333402, 'python3', '');
-- Hex: e03dae44922a8220

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
from f.rel.branch import main as br;
from f.rel.leaf_1 import main as lf_1;
from f.rel.leaf_2 import main as lf_2;

def main():
    return [br(), lf_1(), lf_2];
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/rel/root_script', 333403, 'python3', '');
-- Padded Hex: 000000000005165B
 
-- Prebuild dependency_map 
-- It would be done by Windmill, but this one is static.
INSERT INTO dependency_map(workspace_id, importer_path, importer_kind, imported_path) VALUES (
                        'test-workspace', 'f/rel/root_script', 'script', 'f/rel/branch');
INSERT INTO dependency_map(workspace_id, importer_path, importer_kind, imported_path) VALUES (
                        'test-workspace', 'f/rel/root_script', 'script', 'f/rel/leaf_1');
INSERT INTO dependency_map(workspace_id, importer_path, importer_kind, imported_path) VALUES (
                        'test-workspace', 'f/rel/root_script', 'script', 'f/rel/leaf_2');
INSERT INTO dependency_map(workspace_id, importer_path, importer_kind, imported_path) VALUES (
                        'test-workspace', 'f/rel/branch', 'script', 'f/rel/leaf_1');
