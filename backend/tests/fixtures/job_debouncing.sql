-- SCRIPTS -- 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'def main(x: str = "hey", b: int = 1):
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/scripts/script_1', 533400, 'python3', '');
-- Padded Hex: 0000000000082398
 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/scripts/script_2', 533403, 'python3', '');
-- Padded Hex: 000000000008239B

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/scripts/script_3', 533404, 'python3', '');


-- Padded Hex: 000000000008239C
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace',
'',
'',
'f/flows/flow',
'{1443253234253454}',
'{
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "properties": {},
    "required": [],
    "type": "object"
}',
$tag$
{
    "modules": [
        {
            "id": "a",
            "value": {
                "type": "rawscript",
                "assets": [],
                "content": "def main(x: str, y: str):\n    return x",
                "language": "python3",
                "debounce_delay_s": 2,
                "input_transforms": {
                    "x": {
                        "type": "static",
                        "value": ""
                    },
                    "y": {
                        "type": "static",
                        "value": ""
                    }
                }
            },
            "continue_on_error": false
        }
    ],
    "debounce_delay_s": 2
}$tag$,
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) 
SELECT versions[1], workspace_id, path, schema, value, edited_by FROM flow WHERE path = 'f/flows/flow';

-- No top level debouncing
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace',
'',
'',
'f/flows/flow_full',
'{123}',
'{
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "properties": {},
    "required": [],
    "type": "object"
}',
$tag$
{
    "modules": [
        {
            "id": "a",
            "value": {
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "def main(x: str, y: str):\n    return x",
                "language": "python3",
                "debounce_delay_s": 15,
                "input_transforms": {
                    "x": {
                        "type": "static",
                        "value": ""
                    },
                    "y": {
                        "type": "static",
                        "value": ""
                    }
                },
                "concurrency_time_window_s": 0
            },
            "continue_on_error": false
        },
        {
            "id": "b",
            "value": {
                "type": "branchall",
                "branches": [
                    {
                        "expr": "false",
                        "modules": [],
                        "summary": "",
                        "skip_failure": false
                    }
                ],
                "parallel": true
            },
            "summary": ""
        }
    ]
}$tag$,
'system'
);


INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) 
SELECT versions[1], workspace_id, path, schema, value, edited_by FROM flow WHERE path = 'f/flows/flow_full';
