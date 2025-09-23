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
