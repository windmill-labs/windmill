INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import inspect
import sys

def greet(name: str) -> str:
    # Verify that __file__ is set on this module (same check typeguard does)
    mod = sys.modules[__name__]
    source_file = inspect.getfile(mod)
    return f"Hello, {name}! from {source_file}"

def main():
    return greet("World")
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system/typechecked_helper', 12349, 'python3', '');
