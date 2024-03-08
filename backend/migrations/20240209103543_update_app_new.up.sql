-- Add up migration script here
-- Add up migration script here
-- Add up migration script here

with _insert as 
(
INSERT INTO
    app_version(app_id, created_by, created_at, value)
VALUES
    (
        (
            SELECT
                id
            FROM
                app
            WHERE
                workspace_id = 'admins'
                AND path = 'g/all/setup_app'
        ),
        'admin@windmill.dev',
        '2023-01-01 00:00:00.000000 +00:00',
        '{
    "css": {
        "htmlcomponent": {
            "container": {
                "class": "",
                "style": ""
            }
        },
        "textcomponent": {
            "text": {
                "class": "",
                "style": ""
            },
            "container": {
                "class": "",
                "style": ""
            }
        },
        "imagecomponent": {
            "image": {
                "class": "",
                "style": ""
            }
        },
        "containercomponent": {
            "container": {
                "class": "",
                "style": ""
            }
        },
        "textinputcomponent": {
            "input": {
                "class": "",
                "style": ""
            }
        },
        "schemaformcomponent": {
            "label": {
                "class": "",
                "style": ""
            },
            "container": {
                "class": "",
                "style": ""
            },
            "description": {
                "class": "",
                "style": ""
            }
        },
        "buttoncomponent": {
            "button": {
                "style": "",
                "class": ""
            },
            "container": {
                "style": "",
                "class": ""
            }
        }
    },
    "grid": [
        {
            "3": {
                "h": 2,
                "w": 3,
                "x": 0,
                "y": 0,
                "fixed": false
            },
            "12": {
                "h": 2,
                "w": 11,
                "x": 0,
                "y": 1,
                "fixed": false
            },
            "id": "a",
            "data": {
                "id": "a",
                "type": "textcomponent",
                "customCss": {
                    "text": {
                        "class": "text-3xl font-semibold",
                        "style": ""
                    },
                    "container": {
                        "class": "",
                        "style": ""
                    }
                },
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Body"
                    },
                    "tooltip": {
                        "type": "static",
                        "value": ""
                    },
                    "copyButton": {
                        "type": "static",
                        "value": false
                    }
                },
                "componentInput": {
                    "eval": "Setup your Windmill Instance",
                    "type": "templatev2",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template",
                    "connections": []
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "left"
            }
        },
        {
            "3": {
                "h": 3,
                "w": 3,
                "x": 0,
                "y": 2,
                "fixed": false
            },
            "12": {
                "h": 2,
                "w": 1,
                "x": 11,
                "y": 1,
                "fixed": false
            },
            "data": {
                "id": "b",
                "type": "imagecomponent",
                "customCss": {
                    "image": {
                        "class": "",
                        "style": ""
                    }
                },
                "configuration": {
                    "source": {
                        "type": "static",
                        "value": "/logo.svg"
                    },
                    "altText": {
                        "type": "static",
                        "value": ""
                    },
                    "imageFit": {
                        "type": "static",
                        "value": "contain"
                    },
                    "sourceKind": {
                        "type": "static",
                        "value": "url"
                    }
                }
            },
            "id": "b"
        },
        {
            "3": {
                "h": 11,
                "w": 3,
                "x": 0,
                "y": 6,
                "fixed": false
            },
            "12": {
                "h": 12,
                "w": 6,
                "x": 0,
                "y": 6,
                "fixed": false
            },
            "id": "d",
            "data": {
                "id": "d",
                "type": "containercomponent",
                "customCss": {
                    "container": {
                        "class": "shadow-lg rounded-lg border p-2 mr-2",
                        "style": ""
                    }
                },
                "configuration": {},
                "numberOfSubgrids": 1
            }
        },
        {
            "3": {
                "h": 9,
                "w": 3,
                "x": 0,
                "y": 17,
                "fixed": false
            },
            "12": {
                "h": 12,
                "w": 6,
                "x": 6,
                "y": 6,
                "fixed": false
            },
            "id": "f",
            "data": {
                "id": "f",
                "type": "containercomponent",
                "customCss": {
                    "container": {
                        "class": "shadow-lg rounded-lg border p-2 ml-2",
                        "style": ""
                    }
                },
                "configuration": {},
                "numberOfSubgrids": 1
            }
        },
        {
            "3": {
                "h": 2,
                "w": 3,
                "x": 0,
                "y": 26,
                "fixed": false
            },
            "12": {
                "h": 2,
                "w": 9,
                "x": 3,
                "y": 19,
                "fixed": false
            },
            "id": "i",
            "data": {
                "id": "i",
                "type": "buttoncomponent",
                "customCss": {
                    "button": {
                        "class": "",
                        "style": ""
                    },
                    "container": {
                        "class": "",
                        "style": ""
                    }
                },
                "recomputeIds": [],
                "configuration": {
                    "size": {
                        "type": "static",
                        "value": "lg"
                    },
                    "color": {
                        "type": "static",
                        "value": "dark"
                    },
                    "label": {
                        "type": "static",
                        "value": "Set admin account and hub sync"
                    },
                    "onError": {
                        "type": "oneOf",
                        "selected": "setTab",
                        "configuration": {
                            "open": {
                                "id": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "close": {
                                "id": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "setTab": {
                                "setTab": {
                                    "type": "static",
                                    "value": [
                                        {
                                            "id": "p",
                                            "index": 0
                                        }
                                    ]
                                }
                            },
                            "gotoUrl": {
                                "url": {
                                    "type": "static",
                                    "value": ""
                                },
                                "newTab": {
                                    "type": "static",
                                    "value": true
                                }
                            },
                            "errorOverlay": {},
                            "sendErrorToast": {
                                "message": {
                                    "type": "static",
                                    "value": ""
                                },
                                "appendError": {
                                    "type": "static",
                                    "value": true
                                }
                            }
                        }
                    },
                    "disabled": {
                        "expr": "c?.values?.new_email === '''' || c?.values?.password === ''''||c.valid === false ",
                        "type": "evalv2",
                        "value": false,
                        "connections": [
                            {
                                "id": "values",
                                "componentId": "c"
                            },
                            {
                                "id": "valid",
                                "componentId": "c"
                            }
                        ]
                    },
                    "afterIcon": {
                        "type": "static",
                        "value": "Zap"
                    },
                    "onSuccess": {
                        "type": "oneOf",
                        "selected": "gotoUrl",
                        "configuration": {
                            "none": {},
                            "open": {
                                "id": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "close": {
                                "id": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "setTab": {
                                "setTab": {
                                    "type": "static",
                                    "value": [
                                        {
                                            "id": "p",
                                            "index": 1
                                        }
                                    ]
                                }
                            },
                            "gotoUrl": {
                                "url": {
                                    "expr": "bg_1.result",
                                    "type": "evalv2",
                                    "value": "",
                                    "connections": [
                                        {
                                            "id": "result",
                                            "componentId": "bg_1"
                                        }
                                    ]
                                },
                                "newTab": {
                                    "type": "static",
                                    "value": false
                                }
                            },
                            "openModal": {
                                "modalId": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "sendToast": {
                                "message": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "clearFiles": {
                                "id": {
                                    "type": "static",
                                    "value": ""
                                }
                            },
                            "closeModal": {
                                "modalId": {
                                    "type": "static",
                                    "value": ""
                                }
                            }
                        }
                    },
                    "beforeIcon": {
                        "type": "static",
                        "value": ""
                    },
                    "fillContainer": {
                        "type": "static",
                        "value": false
                    },
                    "triggerOnAppLoad": {
                        "type": "static",
                        "value": false
                    }
                },
                "componentInput": {
                    "type": "runnable",
                    "fields": {
                        "oldEmail": {
                            "expr": "ctx.email",
                            "type": "evalv2",
                            "value": null,
                            "fieldType": "string",
                            "connections": [
                                {
                                    "id": "email",
                                    "componentId": "ctx"
                                }
                            ]
                        },
                        "newEmail": {
                            "expr": "c.values.new_email",
                            "type": "evalv2",
                            "value": null,
                            "fieldType": "string",
                            "connections": [
                                {
                                    "id": "values",
                                    "componentId": "c"
                                }
                            ]
                        },
                        "newPassword": {
                            "expr": "c.values.password",
                            "type": "evalv2",
                            "value": null,
                            "fieldType": "string",
                            "connections": [
                                {
                                    "id": "values",
                                    "componentId": "c"
                                }
                            ]
                        },
                        "enable_hub_sync": {
                            "expr": "h.result",
                            "type": "evalv2",
                            "value": null,
                            "fieldType": "boolean",
                            "connections": [
                                {
                                    "id": "result",
                                    "componentId": "h"
                                }
                            ]
                        },
                        "sync_now": {
                            "type": "evalv2",
                            "value": null,
                            "fieldType": "boolean",
                            "expr": "k.result",
                            "connections": [
                                {
                                    "componentId": "k",
                                    "id": "result"
                                }
                            ]
                        }
                    },
                    "runnable": {
                        "name": "Change Account",
                        "type": "runnableByName",
                        "inlineScript": {
                            "path": "g/all/setup_app/Change_Account",
                            "schema": {
                                "type": "object",
                                "$schema": "https://json-schema.org/draft/2020-12/schema",
                                "required": [
                                    "oldEmail",
                                    "newEmail",
                                    "newPassword",
                                    "enable_hub_sync",
                                    "sync_now"
                                ],
                                "properties": {
                                    "oldEmail": {
                                        "default": null,
                                        "description": "",
                                        "type": "string"
                                    },
                                    "newEmail": {
                                        "default": null,
                                        "description": "",
                                        "type": "string"
                                    },
                                    "newPassword": {
                                        "default": null,
                                        "description": "",
                                        "type": "string"
                                    },
                                    "enable_hub_sync": {
                                        "default": null,
                                        "description": "",
                                        "type": "boolean"
                                    },
                                    "sync_now": {
                                        "default": null,
                                        "description": "",
                                        "type": "boolean"
                                    }
                                }
                            },
                            "content": "import * as wmill from \"https://deno.land/x/windmill@v1.99.0/mod.ts\";\n\nexport async function main(\n  oldEmail: string,\n  newEmail: string,\n  newPassword: string,\n  enable_hub_sync: boolean,\n  sync_now: boolean,\n) {\n  try {\n    await wmill.UserService.createUserGlobally({\n      requestBody: {\n        email: newEmail,\n        password: newPassword,\n        super_admin: true,\n      },\n    });\n  } catch (e) {\n    throw Error(\"User already exists: \" + e.body);\n  }\n\n  let new_token;\n\n  try {\n    new_token = await wmill.UserService.login({\n      requestBody: {\n        email: newEmail,\n        password: newPassword,\n      },\n    });\n  } catch (e) {\n    throw Error(\"Login failed: \" + e.body);\n  }\n\n  wmill.setClient(new_token, Deno.env.get(\"BASE_INTERNAL_URL\")!);\n\n  if (sync_now) {\n    try {\n      await wmill.JobService.runScriptByPath({\n        workspace: \"admins\",\n        path: \"u/admin/hub_sync\",\n        requestBody: {},\n      });\n    } catch (e) {\n      throw Error(\"Hub sync failed:\" + e.body);\n    }\n  }\n\n  if (enable_hub_sync) {\n    try {\n      await wmill.ScheduleService.createSchedule({\n        workspace: \"admins\",\n        requestBody: {\n          path: \"g/all/hub_sync\",\n          schedule: \"0 0 0 * * *\",\n          script_path: \"u/admin/hub_sync\",\n          is_flow: false,\n          args: {},\n          enabled: true,\n          timezone: \"Etc/UTC\",\n        },\n      });\n    } catch (e) {\n      throw Error(\"Error creating schedule: \" + e.body);\n    }\n  }\n  try {\n    await wmill.UserService.globalUserDelete({ email: oldEmail });\n  } catch (e) {\n    throw Error(\"Deleting old account failed: \" + e.body);\n  }\n}\n",
                            "language": "deno"
                        }
                    },
                    "fieldType": "any",
                    "autoRefresh": false,
                    "recomputeOnInputChanged": false
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "right"
            }
        },
        {
            "3": {
                "h": 8,
                "w": 3,
                "x": 0,
                "y": 28,
                "fixed": false
            },
            "12": {
                "h": 7,
                "w": 12,
                "x": 0,
                "y": 21,
                "fixed": false
            },
            "id": "p",
            "data": {
                "id": "p",
                "type": "conditionalwrapper",
                "customCss": {
                    "container": {
                        "class": "",
                        "style": ""
                    }
                },
                "conditions": [
                    {
                        "expr": "i?.result.error",
                        "type": "evalv2",
                        "fieldType": "boolean",
                        "connections": [
                            {
                                "componentId": "i",
                                "id": "result"
                            }
                        ]
                    },
                    {
                        "expr": "true",
                        "type": "evalv2",
                        "fieldType": "boolean",
                        "connections": []
                    }
                ],
                "configuration": {},
                "numberOfSubgrids": 2
            }
        }
    ],
    "theme": {
        "path": "f/app_themes/theme_0",
        "type": "path"
    },
    "subgrids": {
        "d-0": [
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "id": "e",
                "data": {
                    "id": "e",
                    "type": "textcomponent",
                    "customCss": {
                        "text": {
                            "class": "text-xl font-semibold px-2",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Body"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false
                        }
                    },
                    "componentInput": {
                        "eval": "Setup a secure account",
                        "type": "templatev2",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template",
                        "connections": []
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 5,
                    "w": 3,
                    "x": 0,
                    "y": 2,
                    "fixed": false
                },
                "12": {
                    "h": 5,
                    "w": 12,
                    "x": 0,
                    "y": 2,
                    "fixed": false
                },
                "id": "c",
                "data": {
                    "id": "c",
                    "type": "schemaformcomponent",
                    "customCss": {
                        "label": {
                            "class": "",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        },
                        "description": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {
                        "largeGap": {
                            "type": "static",
                            "value": false
                        },
                        "displayType": {
                            "type": "static",
                            "value": false
                        },
                        "dynamicEnums": {
                            "type": "static",
                            "value": {}
                        },
                        "defaultValues": {
                            "type": "static",
                            "value": {}
                        }
                    },
                    "componentInput": {
                        "type": "static",
                        "value": {
                            "order": [
                                "new_email",
                                "password"
                            ],
                            "required": [],
                            "properties": {
                                "new_email": {
                                    "type": "string",
                                    "format": "email",
                                    "default": "",
                                    "pattern": "^[\\w-.]+@([\\w-]+\\.)+[\\w-]{2,4}$",
                                    "description": ""
                                },
                                "password": {
                                    "type": "string",
                                    "description": "",
                                    "pattern": "^..+$",
                                    "default": "",
                                    "customErrorMessage": "Must have at least 2 chars",
                                    "password": true
                                }
                            }
                        },
                        "fieldType": "schema"
                    }
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 9,
                    "fixed": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 10,
                    "fixed": false
                },
                "id": "l",
                "data": {
                    "id": "l",
                    "type": "textcomponent",
                    "customCss": {
                        "text": {
                            "class": "px-2",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Caption"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false
                        }
                    },
                    "componentInput": {
                        "eval": "Current email: ${ctx.email}",
                        "type": "templatev2",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template",
                        "connections": [
                            {
                                "id": "email",
                                "componentId": "ctx"
                            }
                        ]
                    },
                    "verticalAlignment": "top",
                    "horizontalAlignment": "left"
                }
            }
        ],
        "f-0": [
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "id": "g",
                "data": {
                    "id": "g",
                    "type": "textcomponent",
                    "customCss": {
                        "text": {
                            "class": "text-xl font-semibold px-2",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Body"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false
                        }
                    },
                    "componentInput": {
                        "eval": "Hub Sync",
                        "type": "templatev2",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template",
                        "connections": []
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 3,
                    "w": 3,
                    "x": 0,
                    "y": 2,
                    "fixed": false
                },
                "12": {
                    "h": 6,
                    "w": 12,
                    "x": 0,
                    "y": 2,
                    "fixed": false
                },
                "id": "m",
                "data": {
                    "id": "m",
                    "type": "containercomponent",
                    "customCss": {
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {},
                    "numberOfSubgrids": 1
                }
            },
            {
                "3": {
                    "h": 2,
                    "w": 3,
                    "x": 0,
                    "y": 6,
                    "fixed": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 10,
                    "fixed": false
                },
                "id": "n",
                "data": {
                    "id": "n",
                    "type": "textcomponent",
                    "customCss": {
                        "text": {
                            "class": "px-2 xs",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Caption"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false
                        }
                    },
                    "componentInput": {
                        "eval": "The schedule synchronizes resource types from the Hub every day.",
                        "type": "templatev2",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template",
                        "connections": []
                    },
                    "verticalAlignment": "top",
                    "horizontalAlignment": "left"
                }
            }
        ],
        "m-0": [
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "12": {
                    "h": 2,
                    "w": 12,
                    "x": 0,
                    "y": 0,
                    "fixed": false
                },
                "data": {
                    "id": "k",
                    "type": "checkboxcomponent",
                    "customCss": {
                        "text": {
                            "class": "",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "recomputeIds": [],
                    "configuration": {
                        "label": {
                            "type": "static",
                            "value": "Sync resource types now"
                        },
                        "disabled": {
                            "type": "static",
                            "value": false
                        },
                        "defaultValue": {
                            "type": "static",
                            "value": true
                        }
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                },
                "id": "k"
            },
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 1,
                    "fixed": false
                },
                "12": {
                    "h": 2,
                    "w": 12,
                    "x": 0,
                    "y": 2,
                    "fixed": false
                },
                "id": "h",
                "data": {
                    "id": "h",
                    "type": "checkboxcomponent",
                    "customCss": {
                        "text": {
                            "class": "",
                            "style": ""
                        },
                        "container": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "recomputeIds": [],
                    "configuration": {
                        "label": {
                            "type": "static",
                            "value": "Sync resource types  every day"
                        },
                        "disabled": {
                            "type": "static",
                            "value": false
                        },
                        "defaultValue": {
                            "type": "static",
                            "value": true
                        }
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            }
        ],
        "p-0": [
            {
                "3": {
                    "fixed": false,
                    "x": 0,
                    "y": 0,
                    "w": 2,
                    "h": 1
                },
                "12": {
                    "fixed": false,
                    "x": 0,
                    "y": 0,
                    "w": 12,
                    "h": 3
                },
                "data": {
                    "type": "alertcomponent",
                    "configuration": {
                        "type": {
                            "type": "static",
                            "value": "error"
                        },
                        "title": {
                            "type": "static",
                            "value": "There were an error with your setup:"
                        },
                        "description": {
                            "type": "evalv2",
                            "value": "Description",
                            "expr": "i?.result?.error?.message",
                            "connections": [
                                {
                                    "componentId": "i",
                                    "id": "result"
                                }
                            ]
                        },
                        "notRounded": {
                            "type": "static",
                            "value": false
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "size": {
                            "type": "static",
                            "value": "sm"
                        },
                        "collapsible": {
                            "type": "static",
                            "value": false
                        },
                        "initiallyCollapsed": {
                            "type": "static",
                            "value": false
                        }
                    },
                    "customCss": {
                        "container": {
                            "class": "",
                            "style": ""
                        },
                        "background": {
                            "class": "",
                            "style": ""
                        },
                        "icon": {
                            "class": "",
                            "style": ""
                        },
                        "title": {
                            "class": "",
                            "style": ""
                        },
                        "description": {
                            "class": "",
                            "style": ""
                        }
                    },
                    "verticalAlignment": "center",
                    "id": "j"
                },
                "id": "j"
            }
        ],
        "p-1": []
    },
    "fullscreen": false,
    "norefreshbar": true,
    "hiddenInlineScripts": [
        {
            "name": "Background Runnable 0",
            "type": "runnableByName",
            "fields": {},
            "hidden": true
        },
        {
            "name": "Compute URL",
            "type": "runnableByName",
            "fields": {},
            "autoRefresh": true,
            "inlineScript": {
                "path": "u/faton/captivating_app/Compute_URL",
                "content": "return ''/user/logout?rd='' + encodeURIComponent(''/user/login?email='' + c.values.new_email)",
                "language": "frontend",
                "refreshOn": [
                    {
                        "id": "c",
                        "key": "values"
                    }
                ],
                "suggestedRefreshOn": []
            },
            "recomputeIds": [],
            "recomputeOnInputChanged": true
        }
    ],
    "unusedInlineScripts": []
}') RETURNING id)
UPDATE app SET versions = ARRAY((select id from _insert)), policy =  '{ "execution_mode": "viewer", "triggerables": {} }'
WHERE workspace_id = 'admins' AND path = 'g/all/setup_app';
