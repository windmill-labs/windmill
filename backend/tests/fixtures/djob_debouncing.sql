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

-- INSERT INTO public.app(id, workspace_id, path, versions, policy) VALUES (
-- 2,
-- 'test-workspace',
-- 'f/rel/root_app',
-- '{0}',
-- '{}'
-- );

-- INSERT INTO public.app_version(id, app_id, value, created_by) VALUES (
-- 0,
-- 2,
-- $tag${"grid":[{"3":{"h":2,"w":6,"x":0,"y":0,"fixed":true,"fullHeight":false},"12":{"h":2,"w":12,"x":0,"y":0,"fixed":true,"fullHeight":false},"id":"topbar","data":{"id":"topbar","type":"containercomponent","customCss":{"container":{"class":"!p-0","style":""}},"configuration":{},"numberOfSubgrids":1}},{"3":{"h":8,"w":2,"x":0,"y":2,"fixed":false,"fullHeight":false},"12":{"h":2,"w":6,"x":0,"y":2,"fixed":false,"fullHeight":false},"id":"a","data":{"id":"a","type":"containercomponent","customCss":{"container":{"class":"","style":""}},"configuration":{},"numberOfSubgrids":1}},{"3":{"h":1,"w":1,"x":2,"y":2,"fixed":false,"fullHeight":false},"12":{"h":1,"w":2,"x":6,"y":2,"fixed":false,"fullHeight":false},"id":"dontpressmeplz","data":{"id":"dontpressmeplz","type":"buttoncomponent","customCss":{"button":{"class":"","style":""},"container":{"class":"","style":""}},"recomputeIds":[],"configuration":{"size":{"type":"static","value":"xs"},"color":{"type":"static","value":"blue"},"label":{"type":"static","value":"Press me"},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"errorOverlay":{},"sendErrorToast":{"message":{"type":"static","value":"An error occured"},"appendError":{"type":"static","value":true}}}},"disabled":{"type":"static","value":false},"afterIcon":{"type":"static"},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"openModal":{"modalId":{"type":"static","value":""}},"sendToast":{"message":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}}}},"beforeIcon":{"type":"static"},"fillContainer":{"type":"static","value":false},"triggerOnAppLoad":{"type":"static","value":false},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fields":{"x":{"type":"static","value":null,"fieldType":"string"}},"runnable":{"name":"Inline Script","type":"runnableByName","inlineScript":{"path":"u/admin@windmill.dev/newapp/Inline_Script","schema":{"type":"object","$schema":"https://json-schema.org/draft/2020-12/schema","required":["x"],"properties":{"x":{"default":null,"description":"","originalType":"string","type":"string"}}},"content":"from f.rel.leaf_2 import main as lf_2;\n\ndef check():\n    return [lf_2()];\n    \ndef main(x: str):\n    return x","language":"python3"}},"fieldType":"any","autoRefresh":false,"recomputeOnInputChanged":false},"verticalAlignment":"center","horizontalAlignment":"center"}},{"3":{"h":1,"w":1,"x":2,"y":3,"fixed":false,"fullHeight":false},"12":{"h":1,"w":2,"x":8,"y":2,"fixed":false,"fullHeight":false},"id":"d","data":{"id":"d","type":"checkboxcomponent","customCss":{"text":{"class":"","style":""},"container":{"class":"","style":""}},"recomputeIds":[],"configuration":{"label":{"type":"static","value":"Label"},"disabled":{"type":"static","value":false},"defaultValue":{"type":"static","value":false}},"verticalAlignment":"center","horizontalAlignment":"center"}},{"3":{"h":1,"w":1,"x":2,"y":4,"fixed":false,"fullHeight":false},"12":{"h":1,"w":2,"x":6,"y":3,"fixed":false,"fullHeight":false},"id":"youcanpressme","data":{"id":"youcanpressme","type":"buttoncomponent","customCss":{"button":{"class":"","style":""},"container":{"class":"","style":""}},"recomputeIds":[],"configuration":{"size":{"type":"static","value":"xs"},"color":{"type":"static","value":"blue"},"label":{"type":"static","value":"Press me"},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"errorOverlay":{},"sendErrorToast":{"message":{"type":"static","value":"An error occured"},"appendError":{"type":"static","value":true}}}},"disabled":{"type":"static","value":false},"afterIcon":{"type":"static"},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"openModal":{"modalId":{"type":"static","value":""}},"sendToast":{"message":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}}}},"beforeIcon":{"type":"static"},"fillContainer":{"type":"static","value":false},"triggerOnAppLoad":{"type":"static","value":false},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fields":{"x":{"type":"static","value":null,"fieldType":"string"}},"runnable":{"name":"Inline Script","type":"runnableByName","inlineScript":{"path":"u/admin/easy_to_use_app/Inline_Script","schema":{"type":"object","$schema":"https://json-schema.org/draft/2020-12/schema","required":["x"],"properties":{"x":{"default":null,"description":"","originalType":"string","type":"string"}}},"content":"from f.rel.branch import main as br;\n\ndef check():\n    return [br()];\n\ndef main(x: str):\n    return x","language":"python3"}},"fieldType":"any","autoRefresh":false,"recomputeOnInputChanged":false},"verticalAlignment":"center","horizontalAlignment":"center"}}],"theme":{"path":"f/app_themes/theme_0","type":"path"},"subgrids":{"a-0":[{"3":{"h":1,"w":1,"x":0,"y":0,"fixed":false,"fullHeight":false},"12":{"h":2,"w":5,"x":0,"y":0,"fixed":false,"fullHeight":false},"id":"pressmeplz","data":{"id":"pressmeplz","type":"buttoncomponent","customCss":{"button":{"class":"","style":""},"container":{"class":"","style":""}},"recomputeIds":[],"configuration":{"size":{"type":"static","value":"xs"},"color":{"type":"static","value":"blue"},"label":{"type":"static","value":"Press me"},"onError":{"type":"oneOf","selected":"errorOverlay","configuration":{"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"errorOverlay":{},"sendErrorToast":{"message":{"type":"static","value":"An error occured"},"appendError":{"type":"static","value":true}}}},"disabled":{"type":"static","value":false},"afterIcon":{"type":"static"},"onSuccess":{"type":"oneOf","selected":"none","configuration":{"none":{},"open":{"id":{"type":"static","value":""}},"close":{"id":{"type":"static","value":""}},"setTab":{"setTab":{"type":"static","value":[]}},"gotoUrl":{"url":{"type":"static","value":""},"newTab":{"type":"static","value":true}},"openModal":{"modalId":{"type":"static","value":""}},"sendToast":{"message":{"type":"static","value":""}},"clearFiles":{"id":{"type":"static","value":""}},"closeModal":{"modalId":{"type":"static","value":""}}}},"beforeIcon":{"type":"static"},"fillContainer":{"type":"static","value":false},"triggerOnAppLoad":{"type":"static","value":false},"confirmationModal":{"type":"oneOf","selected":"none","configuration":{"none":{},"confirmationModal":{"title":{"type":"static","value":"Title"},"description":{"type":"static","value":"Are you sure?"},"confirmationText":{"type":"static","value":"Confirm"}}}}},"componentInput":{"type":"runnable","fields":{"x":{"type":"static","value":null,"fieldType":"string"}},"runnable":{"name":"Inline Script","type":"runnableByName","inlineScript":{"path":"u/admin@windmill.dev/newapp/Inline_Script","schema":{"type":"object","$schema":"https://json-schema.org/draft/2020-12/schema","required":["x"],"properties":{"x":{"default":null,"description":"","originalType":"string","type":"string"}}},"content":"from f.rel.branch import main as br;\nfrom f.rel.leaf_1 import main as lf_1;\nfrom ..leaf_1 import main as lf_12;\nfrom ...rel.leaf_2 import main as lf_2;\n\ndef check():\n    return [br(), lf_1(), lf_2(), lf_12()];\n\n\ndef main(x: str):\n    return x","language":"python3"}},"fieldType":"any","autoRefresh":false,"recomputeOnInputChanged":false},"verticalAlignment":"center","horizontalAlignment":"center"}}],"topbar-0":[{"3":{"h":1,"w":6,"x":0,"y":0,"fixed":false,"fullHeight":false},"12":{"h":1,"w":6,"x":0,"y":0,"fixed":false,"fullHeight":false},"id":"title","data":{"id":"title","type":"textcomponent","customCss":{"text":{"class":"text-xl font-semibold whitespace-nowrap truncate","style":""},"container":{"class":"","style":""}},"configuration":{"style":{"type":"static","value":"Body"},"tooltip":{"expr":"`Author: ${ctx.author}`","type":"evalv2","value":"","fieldType":"text","connections":[{"id":"author","componentId":"ctx"}]},"copyButton":{"type":"static","value":false},"disableNoText":{"type":"static","value":true,"fieldType":"boolean"}},"componentInput":{"eval":"${ctx.summary}","type":"templatev2","fieldType":"template","connections":[{"id":"summary","componentId":"ctx"}]},"verticalAlignment":"center","horizontalAlignment":"left"}},{"3":{"h":1,"w":3,"x":0,"y":1,"fixed":false,"fullHeight":false},"12":{"h":1,"w":6,"x":6,"y":0,"fixed":false,"fullHeight":false},"id":"recomputeall","data":{"id":"recomputeall","type":"recomputeallcomponent","customCss":{"container":{"class":"","style":""}},"menuItems":[],"configuration":{"defaultRefreshInterval":{"type":"static","value":"0"}},"verticalAlignment":"center","horizontalAlignment":"right"}}]},"fullscreen":false,"norefreshbar":false,"hideLegacyTopBar":true,"hiddenInlineScripts":[],"unusedInlineScripts":[],"mobileViewOnSmallerScreens":false}$tag$,
-- 'system'
-- );

