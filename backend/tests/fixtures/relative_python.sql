INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    return "f/system/same_folder_script"
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system/same_folder_script', 12346, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    return "f/system_relative/different_folder_script"
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system_relative/different_folder_script', 12347, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
from f.system.same_folder_script import main as test1
from ..system.same_folder_script import main as test2
from f.system_relative.different_folder_script import main as test3
from .different_folder_script import main as test4

def main():
    return [test1(), test2(), test3(), test4()]
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system_relative/nested_script', 12348, 'python3', '');

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
'f/rel/s_root', 333403, 'python3', '');
-- Hex: 3d0bb0c62a6811ec
