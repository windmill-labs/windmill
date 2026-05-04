-- Python WAC script with both a @workflow-decorated main and a preprocessor.
-- Used by test_python_wac_v2_with_preprocessor (preprocessor invocation)
-- and test_scripts_list_includes_wac (auto_kind = 'wac' + scripts/list filter).
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock, auto_kind) VALUES (
'test-workspace',
'system',
'
from wmill import task, workflow

@task()
def shout(msg: str) -> str:
    return msg.upper()

def preprocessor(who: str, count: int):
    return {"name": who, "qty": count}

@workflow
async def main(name: str, qty: int):
    s = await shout(name)
    return {"greeting": s, "qty": qty}
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{"name":{"default":null,"description":"","originalType":"string","type":"string"},"qty":{"default":null,"description":"","originalType":"integer","type":"integer"}},"required":["name","qty"],"type":"object"}',
'',
'',
'f/system/wac_with_preprocessor', 123419, 'python3', NULL, 'wac');
