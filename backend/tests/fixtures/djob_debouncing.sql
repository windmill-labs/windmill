-- FLOWS --
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#bottle==0.13.2
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre/leaf_left', 333400, 'python3', '');
-- Padded Hex: 0000000000051658
 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#tiny==0.1.3
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre/leaf_right', 333403, 'python3', '');
-- Padded Hex: 000000000005165B
 
 
INSERT INTO public.flow(workspace_id, summary, description, path, versions, schema, value, edited_by) VALUES (
'test-workspace',
'',
'',
'f/dre/flow',
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
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "import f.dre.leaf_left\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf Left"
        },
        {
            "id": "b",
            "value": {
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "import f.dre.leaf_right\nimport f.dre.leaf_left\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf Left and Right"
        },
        {
            "id": "c",
            "value": {
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "import f.dre.leaf_right\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf RIght"
        }
    ]
}$tag$,
'system'
);

INSERT INTO public.flow_version(id, workspace_id, path, schema, value, created_by) VALUES (
1443253234253454,
'test-workspace',
'f/dre/flow',
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
                "content": "import f.dre.leaf_left\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf Left"
        },
        {
            "id": "b",
            "value": {
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "import f.dre.leaf_right\nimport f.dre.leaf_left\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf Left and Right"
        },
        {
            "id": "c",
            "value": {
                "lock": "# py: 3.11\n",
                "type": "rawscript",
                "assets": [],
                "content": "import f.dre.leaf_right\n\ndef main():\n    pass",
                "language": "python3",
                "input_transforms": {}
            },
            "summary": "leaf RIght"
        }
    ]
}$tag$,
'system'
);

-- APPS --
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#bottle==0.13.2
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre_app/leaf_left', 433400, 'python3', '');
-- Padded Hex: 0000000000069CF8
 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#tiny==0.1.3
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre_app/leaf_right', 433403, 'python3', '');
-- Padded Hex: 0000000000069CFB
 
INSERT INTO public.app(id, workspace_id, path, versions, policy) VALUES (
2,
'test-workspace',
'f/dre_app/app',
'{0}',
'{}'
);

INSERT INTO public.app_version(id, app_id, value, created_by) VALUES (
0,
2,
$tag${"grid":[{"3":{"fixed":true,"x":0,"y":0,"fullHeight":false,"w":6,"h":2},"12":{"fixed":true,"x":0,"y":0,"fullHeight":false,"w":12,"h":2},"data":{"type":"containercomponent","configuration":{},"customCss":{"container":{"class":"!p-0","style":""}},"numberOfSubgrids":1,"id":"topbar"},"id":"topbar"},{"3":{"fixed":false,"x":0,"y":2,"fullHeight":false,"w":1,"h":1},"12":{"fixed":false,"x":0,"y":2,"fullHeight":false,"w":2,"h":1},"data":{"type":"buttoncomponent","configuration":{"label":{"type":"static","value":"A"},"color":{"type":"static","value":"blue"},"size":{"type":"static","value":"xs"},"fillContainer":{"type":"static","value":false},"disabled":{"type":"static","value":false},"beforeIcon":{"type":"static"},"afterIcon":{"type":"static"},"tooltip":{"type":"static","value":""},"triggerOnAppLoad":{"type":"static","value":false},"runInBackground":{"type":"static","value":false},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendToast":{"message":{"type":"static","value":""}},"openModal":{"modalId":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}}}},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"errorOverlay":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendErrorToast":{"message":{"type":"static","value":"An error occurred"},"appendError":{"type":"static","value":true}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}}}},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fieldType":"any","fields":{},"runnable":{"type":"runnableByName","name":"Inline Script","inlineScript":{"content":"import f.dre_app.leaf_left\n\ndef main():\n    pass\n","language":"python3","schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"},"path":"f/dre_app/app/Inline_Script"}},"autoRefresh":false,"recomputeOnInputChanged":false},"customCss":{"button":{"style":"","class":""},"container":{"style":"","class":""}},"recomputeIds":[],"horizontalAlignment":"center","verticalAlignment":"center","id":"a"},"id":"a"},{"3":{"fixed":false,"x":1,"y":2,"fullHeight":false,"w":1,"h":1},"12":{"fixed":false,"x":2,"y":2,"fullHeight":false,"w":2,"h":1},"data":{"type":"buttoncomponent","configuration":{"label":{"type":"static","value":"B"},"color":{"type":"static","value":"blue"},"size":{"type":"static","value":"xs"},"fillContainer":{"type":"static","value":false},"disabled":{"type":"static","value":false},"beforeIcon":{"type":"static"},"afterIcon":{"type":"static"},"tooltip":{"type":"static","value":""},"triggerOnAppLoad":{"type":"static","value":false},"runInBackground":{"type":"static","value":false},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendToast":{"message":{"type":"static","value":""}},"openModal":{"modalId":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}}}},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"errorOverlay":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendErrorToast":{"message":{"type":"static","value":"An error occurred"},"appendError":{"type":"static","value":true}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}}}},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fieldType":"any","fields":{},"runnable":{"type":"runnableByName","name":"Inline Script","inlineScript":{"content":"import f.dre_app.leaf_left\nimport f.dre_app.leaf_right\n\ndef main():\n    pass\n","language":"python3","schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"},"path":"f/dre_app/app/Inline_Script"}},"autoRefresh":false,"recomputeOnInputChanged":false},"customCss":{"button":{"style":"","class":""},"container":{"style":"","class":""}},"recomputeIds":[],"horizontalAlignment":"center","verticalAlignment":"center","id":"b"},"id":"b"},{"3":{"fixed":false,"x":2,"y":2,"fullHeight":false,"w":1,"h":1},"12":{"fixed":false,"x":4,"y":2,"fullHeight":false,"w":2,"h":1},"data":{"type":"buttoncomponent","configuration":{"label":{"type":"static","value":"C"},"color":{"type":"static","value":"blue"},"size":{"type":"static","value":"xs"},"fillContainer":{"type":"static","value":false},"disabled":{"type":"static","value":false},"beforeIcon":{"type":"static"},"afterIcon":{"type":"static"},"tooltip":{"type":"static","value":""},"triggerOnAppLoad":{"type":"static","value":false},"runInBackground":{"type":"static","value":false},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendToast":{"message":{"type":"static","value":""}},"openModal":{"modalId":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}}}},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"errorOverlay":{},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"setTab":{"setTab":{"type":"static","value":[]}},"sendErrorToast":{"message":{"type":"static","value":"An error occurred"},"appendError":{"type":"static","value":true}},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}}}},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fieldType":"any","fields":{},"runnable":{"type":"runnableByName","name":"Inline Script","inlineScript":{"content":"import f.dre_app.leaf_right\n\ndef main():\n    pass\n","language":"python3","schema":{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"},"path":"f/dre_app/app/Inline_Script"}},"autoRefresh":false,"recomputeOnInputChanged":false},"customCss":{"button":{"style":"","class":""},"container":{"style":"","class":""}},"recomputeIds":[],"horizontalAlignment":"center","verticalAlignment":"center","id":"c"},"id":"c"}],"fullscreen":false,"unusedInlineScripts":[],"hiddenInlineScripts":[],"theme":{"type":"path","path":"f/app_themes/theme_0"},"subgrids":{"topbar-0":[{"3":{"fixed":false,"x":0,"y":0,"fullHeight":false,"w":6,"h":1},"12":{"fixed":false,"x":0,"y":0,"fullHeight":false,"w":6,"h":1},"data":{"type":"textcomponent","configuration":{"style":{"type":"static","value":"Body"},"copyButton":{"type":"static","value":false},"tooltip":{"type":"evalv2","value":"","fieldType":"text","expr":"`Author: ${ctx.author}`","connections":[{"componentId":"ctx","id":"author"}]},"disableNoText":{"type":"static","value":true,"fieldType":"boolean"}},"componentInput":{"type":"templatev2","fieldType":"template","eval":"${ctx.summary}","connections":[{"id":"summary","componentId":"ctx"}]},"customCss":{"text":{"class":"text-xl font-semibold whitespace-nowrap truncate","style":""},"container":{"class":"","style":""}},"horizontalAlignment":"left","verticalAlignment":"center","id":"title"},"id":"title"},{"3":{"fixed":false,"x":0,"y":1,"fullHeight":false,"w":3,"h":1},"12":{"fixed":false,"x":6,"y":0,"fullHeight":false,"w":6,"h":1},"data":{"type":"recomputeallcomponent","configuration":{"defaultRefreshInterval":{"type":"static","value":"0"}},"customCss":{"container":{"style":"","class":""}},"menuItems":[],"horizontalAlignment":"right","verticalAlignment":"center","id":"recomputeall"},"id":"recomputeall"}]},"hideLegacyTopBar":true,"mobileViewOnSmallerScreens":false}$tag$,
'system'
);

-- SCRIPTS -- 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#bottle==0.13.2
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre_script/leaf_left', 533400, 'python3', '');
-- Padded Hex: 0000000000082398
 
INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'#requirements:
#tiny==0.1.3
def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre_script/leaf_right', 533403, 'python3', '');
-- Padded Hex: 000000000008239B

INSERT INTO public.script(workspace_id, created_by, content, schema, summary, description, path, hash, language, lock) VALUES (
'test-workspace',
'test-user',
'
import f.dre_script.leaf_left
import f.dre_script.leaf_right

def main():
    pass
',
'{"$schema":"https://json-schema.org/draft/2020-12/schema","properties":{},"required":[],"type":"object"}',
'',
'',
'f/dre_script/script', 533404, 'python3', '');
-- Padded Hex: 000000000008239C

-- Create dependency map
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre/leaf_left', 'flow', 'f/dre/flow', 'a');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre/leaf_left', 'flow', 'f/dre/flow', 'b');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre/leaf_right', 'flow', 'f/dre/flow', 'b');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre/leaf_right', 'flow', 'f/dre/flow', 'c');

INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_app/leaf_left', 'app', 'f/dre_app/app', 'a');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_app/leaf_left', 'app', 'f/dre_app/app', 'b');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_app/leaf_right', 'app', 'f/dre_app/app', 'b');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_app/leaf_right', 'app', 'f/dre_app/app', 'c');

INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_script/leaf_left', 'script', 'f/dre_script/script', '');
INSERT INTO dependency_map (workspace_id, imported_path, importer_kind, importer_path, importer_node_id) VALUES ('test-workspace', 'f/dre_script/leaf_right', 'script', 'f/dre_script/script', '');
