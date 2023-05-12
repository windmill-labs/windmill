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
        "containercomponent": {
            "container": {
                "style": "border: 1px solid #898989;"
            }
        }
    },
    "grid": [
        {
            "3": {
                "h": 1,
                "w": 3,
                "x": 0,
                "y": 0,
                "fixed": false,
                "id": "ab"
            },
            "12": {
                "h": 1,
                "w": 7,
                "x": 0,
                "y": 1,
                "id": "ab",
                "fixed": false
            },
            "id": "ab",
            "data": {
                "id": "ab",
                "type": "textcomponent",
                "customCss": {},
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Title"
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
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
                },
                "verticalAlignment": "top",
                "horizontalAlignment": "left"
            }
        },
        {
            "3": {
                "h": 8,
                "w": 3,
                "x": 0,
                "y": 1,
                "fixed": false,
                "id": "x"
            },
            "12": {
                "h": 7,
                "w": 7,
                "x": 0,
                "y": 3,
                "id": "x",
                "fixed": false
            },
            "id": "x",
            "data": {
                "id": "x",
                "type": "containercomponent",
                "customCss": {
                    "container": {
                        "class": "",
                        "style": "border: 1px solid #898989;"
                    }
                },
                "configuration": {},
                "numberOfSubgrids": 1
            }
        },
        {
            "3": {
                "h": 7,
                "w": 3,
                "x": 0,
                "y": 9,
                "fixed": true,
                "id": "z"
            },
            "12": {
                "h": 7,
                "w": 5,
                "x": 7,
                "y": 3,
                "id": "z",
                "fixed": true
            },
            "id": "z",
            "data": {
                "id": "z",
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
                "h": 3,
                "w": 3,
                "x": 0,
                "y": 16,
                "id": "u",
                "max": {
                    "h": 100,
                    "w": 3
                },
                "min": {
                    "h": 1,
                    "w": 1
                },
                "fixed": false,
                "draggable": true,
                "resizable": true,
                "customDragger": false,
                "customResizer": false
            },
            "12": {
                "h": 2,
                "w": 12,
                "x": 0,
                "y": 12,
                "id": "u",
                "max": {
                    "h": 100,
                    "w": 12
                },
                "min": {
                    "h": 1,
                    "w": 1
                },
                "fixed": false,
                "draggable": true,
                "resizable": true,
                "customDragger": false,
                "customResizer": false
            },
            "id": "u",
            "data": {
                "id": "u",
                "card": false,
                "type": "buttoncomponent",
                "softWrap": true,
                "configuration": {
                    "goto": {
                        "type": "static",
                        "value": "/user/logout",
                        "fieldType": "text",
                        "onlyStatic": true
                    },
                    "size": {
                        "type": "static",
                        "value": "lg",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "buttonSizeOptions"
                    },
                    "color": {
                        "type": "static",
                        "value": "blue",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "buttonColorOptions"
                    },
                    "label": {
                        "type": "static",
                        "value": "Setup instance",
                        "fieldType": "text"
                    },
                    "disabled": {
                        "expr": "g.result == \"\" || i.result == \"\"",
                        "type": "eval",
                        "fieldType": "boolean"
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
                            "setTab": {
                                "setTab": {
                                    "type": "static",
                                    "value": []
                                }
                            },
                            "gotoUrl": {
                                "url": {
                                    "type": "connected",
                                    "value": "",
                                    "connection": {
                                        "path": "result",
                                        "componentId": "bg_0"
                                    }
                                },
                                "newTab": {
                                    "type": "static",
                                    "value": false
                                }
                            },
                            "sendToast": {
                                "message": {
                                    "type": "static",
                                    "value": ""
                                }
                            }
                        }
                    },
                    "beforeIcon": {
                        "type": "static"
                    },
                    "fillContainer": {
                        "type": "static",
                        "value": true,
                        "fieldType": "boolean",
                        "onlyStatic": true
                    },
                    "triggerOnAppLoad": {
                        "type": "static"
                    }
                },
                "componentInput": {
                    "type": "runnable",
                    "fields": {
                        "cron_str": {
                            "type": "connected",
                            "value": null,
                            "fieldType": "string",
                            "connection": {
                                "path": "result",
                                "componentId": "p"
                            }
                        },
                        "newEmail": {
                            "type": "connected",
                            "value": null,
                            "format": "",
                            "fieldType": "string",
                            "connection": {
                                "path": "result",
                                "componentId": "g"
                            }
                        },
                        "oldEmail": {
                            "type": "connected",
                            "value": null,
                            "fieldType": "string",
                            "connection": {
                                "path": "email",
                                "componentId": "ctx"
                            }
                        },
                        "newPassword": {
                            "type": "connected",
                            "value": null,
                            "format": "",
                            "fieldType": "string",
                            "connection": {
                                "path": "result",
                                "componentId": "i"
                            }
                        },
                        "enable_hub_sync": {
                            "type": "connected",
                            "value": null,
                            "format": "",
                            "fieldType": "boolean",
                            "connection": {
                                "path": "result",
                                "componentId": "n"
                            }
                        }
                    },
                    "runnable": {
                        "name": "Change Account",
                        "type": "runnableByName",
                        "inlineScript": {
                            "path": "u/user/effortless_app/Change Account",
                            "schema": {
                                "type": "object",
                                "$schema": "https://json-schema.org/draft/2020-12/schema",
                                "required": [
                                    "oldEmail",
                                    "newEmail",
                                    "newPassword",
                                    "enable_hub_sync",
                                    "cron_str"
                                ],
                                "properties": {
                                    "cron_str": {
                                        "type": "string",
                                        "default": null,
                                        "description": ""
                                    },
                                    "newEmail": {
                                        "type": "string",
                                        "format": "",
                                        "default": null,
                                        "description": ""
                                    },
                                    "oldEmail": {
                                        "type": "string",
                                        "default": null,
                                        "description": ""
                                    },
                                    "newPassword": {
                                        "type": "string",
                                        "format": "",
                                        "default": null,
                                        "description": ""
                                    },
                                    "enable_hub_sync": {
                                        "type": "boolean",
                                        "format": "",
                                        "default": null,
                                        "description": ""
                                    }
                                }
                            },
                            "content": "import * as wmill from \"https://deno.land/x/windmill@v1.99.0/mod.ts\";\n\nexport async function main(\n  oldEmail: string,\n  newEmail: string,\n  newPassword: string,\n  enable_hub_sync: boolean,\n  cron_str: string,\n) {\n  try {\n    await wmill.UserService.createUserGlobally({\n      requestBody: {\n        email: newEmail,\n        password: newPassword,\n        super_admin: true,\n      },\n    });\n  } catch (e) {\n    throw Error(\"User already exist\")\n  }\n    const new_token = await wmill.UserService.login({\n      requestBody: {\n        email: newEmail,\n        password: newPassword,\n      },\n    });\n\n    wmill.setClient(new_token, Deno.env.get(\"BASE_INTERNAL_URL\")!);\n    await wmill.JobService.runScriptByPath({\n      workspace: \"admins\",\n      path: \"u/admin/hub_sync\",\n      requestBody: {},\n    });\n\n    if (enable_hub_sync) {\n      try {\n        await wmill.ScheduleService.createSchedule({\n          workspace: \"admins\",\n          requestBody: {\n            path: \"g/all/hub_sync\",\n            schedule: cron_str,\n            script_path: \"u/admin/hub_sync\",\n            is_flow: false,\n            args: {},\n            enabled: true,\n            timezone: \"Etc/UTC\",\n          },\n        });\n      } catch {\n        console.log(\"Schedule already exists\");\n      }\n    }\n\n    await wmill.UserService.globalUserDelete({ email: oldEmail });\n\n}\n",
                            "language": "deno"
                        }
                    },
                    "fieldType": "any",
                    "doNotRecomputeOnInputChanged": false
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        }
    ],
    "subgrids": {
        "x-0": [
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 0,
                    "id": "l",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 0,
                    "id": "l",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "l",
                "data": {
                    "id": "l",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": true,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Subtitle",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "Change superadmin account",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 0,
                    "y": 1,
                    "id": "c",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 4,
                    "x": 0,
                    "y": 1,
                    "id": "c",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "c",
                "data": {
                    "id": "c",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": true,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Label",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "Old Email",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 0,
                    "y": 2,
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false,
                    "id": "w"
                },
                "12": {
                    "h": 1,
                    "w": 6,
                    "x": 0,
                    "y": 2,
                    "id": "w",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "w",
                "data": {
                    "id": "w",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": false,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Body",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "${ctx.username}",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 0,
                    "y": 4,
                    "id": "d",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 5,
                    "x": 7,
                    "y": 1,
                    "id": "d",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "d",
                "data": {
                    "id": "d",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": true,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Label",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "New Email",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 2,
                    "y": 4,
                    "id": "e",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 5,
                    "x": 7,
                    "y": 4,
                    "id": "e",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "e",
                "data": {
                    "id": "e",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": true,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Label",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": ""
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "New Password",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 0,
                    "y": 5,
                    "id": "g",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 5,
                    "x": 7,
                    "y": 2,
                    "id": "g",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "g",
                "data": {
                    "id": "g",
                    "card": false,
                    "type": "textinputcomponent",
                    "softWrap": true,
                    "configuration": {
                        "placeholder": {
                            "type": "static",
                            "value": "good@corp.com",
                            "fieldType": "text",
                            "onlyStatic": true
                        },
                        "defaultValue": {
                            "type": "static",
                            "fieldType": "text"
                        }
                    },
                    "verticalAlignment": "center"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 2,
                    "y": 5,
                    "id": "i",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 5,
                    "x": 7,
                    "y": 5,
                    "id": "i",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": false,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "i",
                "data": {
                    "id": "i",
                    "card": false,
                    "type": "passwordinputcomponent",
                    "softWrap": true,
                    "configuration": {
                        "placeholder": {
                            "type": "static",
                            "value": "Password",
                            "fieldType": "text",
                            "onlyStatic": true
                        }
                    },
                    "verticalAlignment": "center"
                }
            }
        ],
        "z-0": [
            {
                "3": {
                    "h": 1,
                    "w": 3,
                    "x": 0,
                    "y": 0,
                    "id": "aa",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 12,
                    "x": 0,
                    "y": 0,
                    "id": "aa",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "aa",
                "data": {
                    "id": "aa",
                    "card": false,
                    "type": "textcomponent",
                    "softWrap": true,
                    "configuration": {
                        "style": {
                            "type": "static",
                            "value": "Subtitle",
                            "fieldType": "select",
                            "onlyStatic": true,
                            "optionValuesKey": "textStyleOptions"
                        },
                        "tooltip": {
                            "type": "static",
                            "value": "We recommend enabling hub sync below. This will create a schedule in the admin workspace on behalf of the new user once you press the button. This schedule will automatically synchronize resource types from the hub to your local instance."
                        },
                        "copyButton": {
                            "type": "static",
                            "value": false,
                            "fieldType": "boolean",
                            "onlyStatic": true
                        },
                        "extraStyle": {
                            "type": "static",
                            "value": "",
                            "tooltip": "CSS rules like \"color: blue;\"",
                            "fieldType": "text"
                        }
                    },
                    "componentInput": {
                        "eval": "Hub Sync",
                        "type": "template",
                        "value": "Hello ${ctx.username}",
                        "fieldType": "template"
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "left"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 2,
                    "x": 0,
                    "y": 3,
                    "id": "n",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 6,
                    "x": 0,
                    "y": 2,
                    "id": "n",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "n",
                "data": {
                    "id": "n",
                    "card": false,
                    "type": "checkboxcomponent",
                    "softWrap": true,
                    "configuration": {
                        "label": {
                            "type": "static",
                            "value": "Schedule",
                            "fieldType": "text"
                        },
                        "defaultValue": {
                            "type": "static",
                            "value": true,
                            "fieldType": "boolean"
                        }
                    },
                    "verticalAlignment": "center",
                    "horizontalAlignment": "center"
                }
            },
            {
                "3": {
                    "h": 1,
                    "w": 1,
                    "x": 2,
                    "y": 3,
                    "id": "p",
                    "max": {
                        "h": 100,
                        "w": 3
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "12": {
                    "h": 1,
                    "w": 6,
                    "x": 6,
                    "y": 2,
                    "id": "p",
                    "max": {
                        "h": 100,
                        "w": 12
                    },
                    "min": {
                        "h": 1,
                        "w": 1
                    },
                    "fixed": true,
                    "draggable": true,
                    "resizable": true,
                    "customDragger": false,
                    "customResizer": false
                },
                "id": "p",
                "data": {
                    "id": "p",
                    "card": false,
                    "type": "textinputcomponent",
                    "softWrap": true,
                    "configuration": {
                        "placeholder": {
                            "type": "static",
                            "value": "Type...",
                            "fieldType": "text",
                            "onlyStatic": true
                        },
                        "defaultValue": {
                            "type": "static",
                            "value": "0 0 0 * * *",
                            "fieldType": "text"
                        }
                    },
                    "verticalAlignment": "center"
                }
            }
        ]
    },
    "fullscreen": false,
    "norefreshbar": true,
    "hiddenInlineScripts": [
        {
            "name": "Background Script 0",
            "fields": {},
            "autoRefresh": true,
            "inlineScript": {
                "path": "u/user/effortless_app/Background Script 0",
                "content": "return ''/user/logout?rd='' + encodeURIComponent(''/user/login?email='' + g.result)",
                "language": "frontend",
                "refreshOn": [
                    {
                        "id": "g",
                        "key": "result"
                    }
                ]
            },
            "doNotRecomputeOnInputChanged": false
        }
    ],
    "unusedInlineScripts": []
}') RETURNING id)
UPDATE app SET versions = ARRAY((select id from _insert)), policy =  '{ "execution_mode": "viewer", "triggerables": {} }'
WHERE workspace_id = 'admins' AND path = 'g/all/setup_app';
