
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
    "grid": [
        {
            "3": {
                "h": 1,
                "w": 3,
                "x": 0,
                "y": 0,
                "id": "a",
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
                "h": 2,
                "w": 12,
                "x": 0,
                "y": 0,
                "id": "a",
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
            "id": "a",
            "data": {
                "id": "a",
                "card": false,
                "type": "textcomponent",
                "softWrap": true,
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Title",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "textStyleOptions"
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
                    "eval": "Setup your Windmill instance",
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        },
        {
            "3": {
                "h": 1,
                "w": 1,
                "x": 0,
                "y": 6,
                "id": "c",
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
                "w": 4,
                "x": 1,
                "y": 6,
                "id": "c",
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
                "x": 2,
                "y": 6,
                "id": "d",
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
                "w": 4,
                "x": 7,
                "y": 6,
                "id": "d",
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
                "y": 8,
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
                "w": 4,
                "x": 7,
                "y": 8,
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
                "x": 2,
                "y": 7,
                "id": "g",
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
                "w": 4,
                "x": 7,
                "y": 7,
                "id": "g",
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
            "id": "g",
            "data": {
                "id": "g",
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
                "y": 9,
                "id": "i",
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
                "w": 4,
                "x": 7,
                "y": 9,
                "id": "i",
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
        },
        {
            "3": {
                "h": 1,
                "w": 3,
                "x": 0,
                "y": 1,
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
                "y": 2,
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
                    "eval": "Setup a secure account",
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        },
        {
            "3": {
                "h": 3,
                "w": 3,
                "x": 0,
                "y": 2,
                "id": "m",
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
                "h": 3,
                "w": 10,
                "x": 1,
                "y": 3,
                "id": "m",
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
            "id": "m",
            "data": {
                "id": "m",
                "card": false,
                "type": "textcomponent",
                "softWrap": true,
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Body",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "textStyleOptions"
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
                    "eval": "The below button will replace your account with a new one.\nYou''ll have to log in again.",
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        },
        {
            "3": {
                "h": 1,
                "w": 2,
                "x": 0,
                "y": 17,
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
                "w": 4,
                "x": 2,
                "y": 17,
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
                        "value": "Enable Hub Sync",
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
                "h": 3,
                "w": 3,
                "x": 0,
                "y": 14,
                "id": "o",
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
                "h": 3,
                "w": 8,
                "x": 2,
                "y": 14,
                "id": "o",
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
            "id": "o",
            "data": {
                "id": "o",
                "card": false,
                "type": "textcomponent",
                "softWrap": true,
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Body",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "textStyleOptions"
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
                    "eval": "We recommend enabling hub sync below. This will create a schedule in the admin workspace on behalf of the new user once you press the button. This schedule will automatically syncronize resource types from the hub to your local instance.",
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
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
                "y": 17,
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
                "w": 4,
                "x": 6,
                "y": 17,
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
        },
        {
            "3": {
                "h": 3,
                "w": 3,
                "x": 0,
                "y": 19,
                "id": "u",
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
                "h": 2,
                "w": 8,
                "x": 2,
                "y": 19,
                "id": "u",
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
            "id": "u",
            "data": {
                "id": "u",
                "card": false,
                "type": "buttoncomponent",
                "softWrap": true,
                "configuration": {
                    "size": {
                        "type": "static",
                        "value": "lg",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "buttonSizeOptions"
                    },
                    "color": {
                        "type": "static",
                        "value": "red",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "buttonColorOptions"
                    },
                    "label": {
                        "type": "static",
                        "value": "Change account and set hub sync - re-login necessary",
                        "fieldType": "text"
                    },
                    "disabled": {
                        "expr": "g.result == \"\" || i.result == \"\"",
                        "type": "eval",
                        "fieldType": "boolean"
                    },
                    "fillContainer": {
                        "type": "static",
                        "value": true,
                        "fieldType": "boolean",
                        "onlyStatic": true
                    },
                    "goto": {
                        "type": "static",
                        "value": "/user/logout",
                        "fieldType": "text",
                        "onlyStatic": true
                    }
                },
                "componentInput": {
                    "type": "runnable",
                    "fields": {
                        "oldEmail": {
                            "type": "connected",
                            "value": null,
                            "fieldType": "string",
                            "connection": {
                                "componentId": "ctx",
                                "path": "email"
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
                        },
                        "cron_str": {
                            "type": "connected",
                            "value": null,
                            "fieldType": "string",
                            "connection": {
                                "path": "result",
                                "componentId": "p"
                            }
                        }
                    },
                    "runnable": {
                        "name": "Change Account",
                        "type": "runnableByName",
                        "inlineScript": {
                            "path": "/inline-script/Inline Script 0",
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
                                    "oldEmail": {
                                        "description": "",
                                        "type": "string",
                                        "default": null
                                    },
                                    "newEmail": {
                                        "type": "string",
                                        "format": "",
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
                                    },
                                    "cron_str": {
                                        "type": "string",
                                        "default": null,
                                        "description": ""
                                    }
                                }
                            },
                            "content": "import * as wmill from \"https://deno.land/x/windmill@v1.69.3/mod.ts\";\n\nexport async function main(\n  oldEmail: string,\n  newEmail: string,\n  newPassword: string,\n  enable_hub_sync: boolean,\n  cron_str: string,\n) {\n  await wmill.UserService.createUserGlobally({\n    requestBody: {\n      email: newEmail,\n      password: newPassword,\n      super_admin: true,\n    },\n  });\n\n  const new_token = await wmill.UserService.login({\n    requestBody: {\n      email: newEmail,\n      password: newPassword,\n    },\n  });\n\n  wmill.setClient(new_token, Deno.env.get(\"BASE_INTERNAL_URL\")!);\n  await wmill.JobService.runScriptByPath({\n    workspace: \"admins\",\n    path: \"u/admin/hub_sync\",\n    requestBody: {},\n  });\n  \n  if (enable_hub_sync) {\n    try {\n      await wmill.ScheduleService.createSchedule({\n        workspace: \"admins\",\n        requestBody: {\n          path: \"g/all/hub_sync\",\n          schedule: cron_str,\n          script_path: \"u/admin/hub_sync\",\n          is_flow: false,\n          args: {},\n          enabled: true,\n          offset: 0,\n        },\n      });\n    } catch {\n      console.log(\"Schedule already exists\");\n    }\n  }\n\n  await wmill.UserService.globalUserDelete({ email: oldEmail });\n}\n",
                            "language": "deno"
                        }
                    },
                    "fieldType": "any"
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        },
        {
            "3": {
                "h": 1,
                "w": 3,
                "x": 0,
                "y": 13,
                "id": "v",
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
                "y": 13,
                "id": "v",
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
            "id": "v",
            "data": {
                "id": "v",
                "card": false,
                "type": "textcomponent",
                "softWrap": false,
                "configuration": {
                    "style": {
                        "type": "static",
                        "value": "Subtitle",
                        "fieldType": "select",
                        "onlyStatic": true,
                        "optionValuesKey": "textStyleOptions"
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
                    "eval": "Setup Hub Sync",
                    "type": "template",
                    "value": "Hello ${ctx.username}",
                    "fieldType": "template"
                },
                "verticalAlignment": "center",
                "horizontalAlignment": "center"
            }
        },
        {
            "3": {
                "fixed": false,
                "resizable": true,
                "draggable": true,
                "customDragger": false,
                "customResizer": false,
                "x": 0,
                "y": 5,
                "min": {
                    "w": 1,
                    "h": 1
                },
                "max": {
                    "w": 3,
                    "h": 100
                },
                "w": 1,
                "h": 1
            },
            "12": {
                "fixed": false,
                "resizable": true,
                "draggable": true,
                "customDragger": false,
                "customResizer": false,
                "x": 1,
                "y": 7,
                "min": {
                    "w": 1,
                    "h": 1
                },
                "max": {
                    "w": 12,
                    "h": 100
                },
                "w": 4,
                "h": 1,
                "id": "w"
            },
            "data": {
                "softWrap": false,
                "horizontalAlignment": "left",
                "verticalAlignment": "center",
                "id": "w",
                "type": "textcomponent",
                "componentInput": {
                    "type": "template",
                    "fieldType": "template",
                    "value": "Hello ${ctx.username}",
                    "eval": "${ctx.username}"
                },
                "configuration": {
                    "style": {
                        "fieldType": "select",
                        "type": "static",
                        "onlyStatic": true,
                        "optionValuesKey": "textStyleOptions",
                        "value": "Body"
                    },
                    "extraStyle": {
                        "type": "static",
                        "fieldType": "text",
                        "value": "",
                        "tooltip": "CSS rules like \"color: blue;\""
                    },
                    "copyButton": {
                        "type": "static",
                        "value": false,
                        "fieldType": "boolean",
                        "onlyStatic": true
                    }
                },
                "card": false
            },
            "id": "w"
        }
    ],
    "fullscreen": false,
    "hiddenInlineScripts": [],
    "unusedInlineScripts": []
}') RETURNING id)
UPDATE app SET versions = ARRAY((select id from _insert)), policy =  '{ "execution_mode": "viewer", "triggerables": {} }'
 WHERE workspace_id = 'admins' AND path = 'g/all/setup_app';
