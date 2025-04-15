INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
# requirements:
# microdot==2.2.0 

import pandas
import requests
import tiny # pin: tiny==0.1.2

def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system/requirements', 12346, 'python3', '');

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
# extra_requirements:
# bottle==0.13.2

import tiny

def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system/extra_requirements', 12347, 'python3', '');


INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import tiny # pin: bottle==0.13.2
import simplejson # pin: simplejson==3.19.3 

def main():
    return [test1(), test2(), test3(), test4()]
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/system/pins', 12348, 'python3', '');
