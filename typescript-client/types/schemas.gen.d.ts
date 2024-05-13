export declare const $Script: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly hash: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly parent_hashes: {
            readonly type: "array";
            readonly description: "The first element is the direct parent of the script, the second is the parent of the first, etc\n";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly content: {
            readonly type: "string";
        };
        readonly created_by: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly archived: {
            readonly type: "boolean";
        };
        readonly schema: {
            readonly type: "object";
        };
        readonly deleted: {
            readonly type: "boolean";
        };
        readonly is_template: {
            readonly type: "boolean";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly lock: {
            readonly type: "string";
        };
        readonly lock_error_logs: {
            readonly type: "string";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
        readonly kind: {
            readonly type: "string";
            readonly enum: readonly ["script", "failure", "trigger", "command", "approval"];
        };
        readonly starred: {
            readonly type: "boolean";
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly has_draft: {
            readonly type: "boolean";
        };
        readonly draft_only: {
            readonly type: "boolean";
        };
        readonly envs: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly concurrent_limit: {
            readonly type: "integer";
        };
        readonly concurrency_time_window_s: {
            readonly type: "integer";
        };
        readonly cache_ttl: {
            readonly type: "number";
        };
        readonly dedicated_worker: {
            readonly type: "boolean";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly priority: {
            readonly type: "integer";
        };
        readonly restart_unless_cancelled: {
            readonly type: "boolean";
        };
        readonly timeout: {
            readonly type: "integer";
        };
        readonly delete_after_use: {
            readonly type: "boolean";
        };
        readonly visible_to_runner_only: {
            readonly type: "boolean";
        };
        readonly no_main_func: {
            readonly type: "boolean";
        };
        readonly codebase: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["hash", "path", "summary", "description", "content", "created_by", "created_at", "archived", "deleted", "is_template", "extra_perms", "language", "kind", "starred", "no_main_func"];
};
export declare const $NewScript: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly parent_hash: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly content: {
            readonly type: "string";
        };
        readonly schema: {
            readonly type: "object";
        };
        readonly is_template: {
            readonly type: "boolean";
        };
        readonly lock: {
            readonly type: "string";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
        readonly kind: {
            readonly type: "string";
            readonly enum: readonly ["script", "failure", "trigger", "command", "approval"];
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly draft_only: {
            readonly type: "boolean";
        };
        readonly envs: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly concurrent_limit: {
            readonly type: "integer";
        };
        readonly concurrency_time_window_s: {
            readonly type: "integer";
        };
        readonly cache_ttl: {
            readonly type: "number";
        };
        readonly dedicated_worker: {
            readonly type: "boolean";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly priority: {
            readonly type: "integer";
        };
        readonly restart_unless_cancelled: {
            readonly type: "boolean";
        };
        readonly timeout: {
            readonly type: "integer";
        };
        readonly delete_after_use: {
            readonly type: "boolean";
        };
        readonly deployment_message: {
            readonly type: "string";
        };
        readonly concurrency_key: {
            readonly type: "string";
        };
        readonly visible_to_runner_only: {
            readonly type: "boolean";
        };
        readonly no_main_func: {
            readonly type: "boolean";
        };
        readonly codebase: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["path", "summary", "description", "content", "language"];
};
export declare const $NewScriptWithDraft: {
    readonly allOf: readonly [{
        readonly $ref: "#/components/schemas/NewScript";
    }, {
        readonly type: "object";
        readonly properties: {
            readonly draft: {
                readonly $ref: "#/components/schemas/NewScript";
            };
            readonly hash: {
                readonly type: "string";
            };
        };
        readonly required: readonly ["hash"];
    }];
};
export declare const $ScriptHistory: {
    readonly type: "object";
    readonly properties: {
        readonly script_hash: {
            readonly type: "string";
        };
        readonly deployment_msg: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["script_hash"];
};
export declare const $ScriptArgs: {
    readonly type: "object";
    readonly additionalProperties: {};
};
export declare const $Input: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly args: {
            readonly type: "object";
        };
        readonly created_by: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly is_public: {
            readonly type: "boolean";
        };
        readonly success: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["id", "name", "args", "created_by", "created_at", "is_public"];
};
export declare const $CreateInput: {
    readonly type: "object";
    readonly properties: {
        readonly name: {
            readonly type: "string";
        };
        readonly args: {
            readonly type: "object";
        };
    };
    readonly required: readonly ["name", "args", "created_by"];
};
export declare const $UpdateInput: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly is_public: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["id", "name", "is_public"];
};
export declare const $RunnableType: {
    readonly type: "string";
    readonly enum: readonly ["ScriptHash", "ScriptPath", "FlowPath"];
};
export declare const $QueuedJob: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly id: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly parent_job: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly created_by: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly started_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly scheduled_for: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly running: {
            readonly type: "boolean";
        };
        readonly script_path: {
            readonly type: "string";
        };
        readonly script_hash: {
            readonly type: "string";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly logs: {
            readonly type: "string";
        };
        readonly raw_code: {
            readonly type: "string";
        };
        readonly canceled: {
            readonly type: "boolean";
        };
        readonly canceled_by: {
            readonly type: "string";
        };
        readonly canceled_reason: {
            readonly type: "string";
        };
        readonly last_ping: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly job_kind: {
            readonly type: "string";
            readonly enum: readonly ["script", "preview", "dependencies", "flowdependencies", "appdependencies", "flow", "flowpreview", "script_hub", "identity", "deploymentcallback", "singlescriptflow"];
        };
        readonly schedule_path: {
            readonly type: "string";
        };
        readonly permissioned_as: {
            readonly type: "string";
            readonly description: "The user (u/userfoo) or group (g/groupfoo) whom \nthe execution of this script will be permissioned_as and by extension its DT_TOKEN.\n";
        };
        readonly flow_status: {
            readonly $ref: "#/components/schemas/FlowStatus";
        };
        readonly raw_flow: {
            readonly $ref: "#/components/schemas/FlowValue";
        };
        readonly is_flow_step: {
            readonly type: "boolean";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
        readonly email: {
            readonly type: "string";
        };
        readonly visible_to_owner: {
            readonly type: "boolean";
        };
        readonly mem_peak: {
            readonly type: "integer";
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly priority: {
            readonly type: "integer";
        };
    };
    readonly required: readonly ["id", "running", "canceled", "job_kind", "permissioned_as", "is_flow_step", "email", "visible_to_owner", "tag"];
};
export declare const $CompletedJob: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly id: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly parent_job: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly created_by: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly started_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly duration_ms: {
            readonly type: "integer";
        };
        readonly success: {
            readonly type: "boolean";
        };
        readonly script_path: {
            readonly type: "string";
        };
        readonly script_hash: {
            readonly type: "string";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly result: {};
        readonly logs: {
            readonly type: "string";
        };
        readonly deleted: {
            readonly type: "boolean";
        };
        readonly raw_code: {
            readonly type: "string";
        };
        readonly canceled: {
            readonly type: "boolean";
        };
        readonly canceled_by: {
            readonly type: "string";
        };
        readonly canceled_reason: {
            readonly type: "string";
        };
        readonly job_kind: {
            readonly type: "string";
            readonly enum: readonly ["script", "preview", "dependencies", "flow", "flowdependencies", "appdependencies", "flowpreview", "script_hub", "identity", "deploymentcallback", "singlescriptflow"];
        };
        readonly schedule_path: {
            readonly type: "string";
        };
        readonly permissioned_as: {
            readonly type: "string";
            readonly description: "The user (u/userfoo) or group (g/groupfoo) whom \nthe execution of this script will be permissioned_as and by extension its DT_TOKEN.\n";
        };
        readonly flow_status: {
            readonly $ref: "#/components/schemas/FlowStatus";
        };
        readonly raw_flow: {
            readonly $ref: "#/components/schemas/FlowValue";
        };
        readonly is_flow_step: {
            readonly type: "boolean";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
        readonly is_skipped: {
            readonly type: "boolean";
        };
        readonly email: {
            readonly type: "string";
        };
        readonly visible_to_owner: {
            readonly type: "boolean";
        };
        readonly mem_peak: {
            readonly type: "integer";
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly priority: {
            readonly type: "integer";
        };
        readonly labels: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["id", "created_by", "duration_ms", "created_at", "started_at", "success", "canceled", "job_kind", "permissioned_as", "is_flow_step", "is_skipped", "email", "visible_to_owner", "tag"];
};
export declare const $Job: {
    readonly oneOf: readonly [{
        readonly allOf: readonly [{
            readonly $ref: "#/components/schemas/CompletedJob";
        }, {
            readonly type: "object";
            readonly properties: {
                readonly type: {
                    readonly type: "string";
                    readonly enum: readonly ["CompletedJob"];
                };
            };
        }];
    }, {
        readonly allOf: readonly [{
            readonly $ref: "#/components/schemas/QueuedJob";
        }, {
            readonly type: "object";
            readonly properties: {
                readonly type: {
                    readonly type: "string";
                    readonly enum: readonly ["QueuedJob"];
                };
            };
        }];
    }];
    readonly discriminator: {
        readonly propertyName: "type";
    };
};
export declare const $User: {
    readonly type: "object";
    readonly properties: {
        readonly email: {
            readonly type: "string";
        };
        readonly username: {
            readonly type: "string";
        };
        readonly is_admin: {
            readonly type: "boolean";
        };
        readonly is_super_admin: {
            readonly type: "boolean";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly operator: {
            readonly type: "boolean";
        };
        readonly disabled: {
            readonly type: "boolean";
        };
        readonly groups: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly folders: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly folders_owners: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["email", "username", "is_admin", "is_super_admin", "created_at", "operator", "disabled", "folders", "folders_owners"];
};
export declare const $UserUsage: {
    readonly type: "object";
    readonly properties: {
        readonly email: {
            readonly type: "string";
        };
        readonly executions: {
            readonly type: "number";
        };
    };
};
export declare const $Login: {
    readonly type: "object";
    readonly properties: {
        readonly email: {
            readonly type: "string";
        };
        readonly password: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["email", "password"];
};
export declare const $EditWorkspaceUser: {
    readonly type: "object";
    readonly properties: {
        readonly is_admin: {
            readonly type: "boolean";
        };
        readonly operator: {
            readonly type: "boolean";
        };
        readonly disabled: {
            readonly type: "boolean";
        };
    };
};
export declare const $TruncatedToken: {
    readonly type: "object";
    readonly properties: {
        readonly label: {
            readonly type: "string";
        };
        readonly expiration: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly token_prefix: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly last_used_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly scopes: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["token_prefix", "created_at", "last_used_at"];
};
export declare const $NewToken: {
    readonly type: "object";
    readonly properties: {
        readonly label: {
            readonly type: "string";
        };
        readonly expiration: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly scopes: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
};
export declare const $NewTokenImpersonate: {
    readonly type: "object";
    readonly properties: {
        readonly label: {
            readonly type: "string";
        };
        readonly expiration: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly impersonate_email: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["impersonate_email"];
};
export declare const $ListableVariable: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly value: {
            readonly type: "string";
        };
        readonly is_secret: {
            readonly type: "boolean";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly account: {
            readonly type: "integer";
        };
        readonly is_oauth: {
            readonly type: "boolean";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly is_expired: {
            readonly type: "boolean";
        };
        readonly refresh_error: {
            readonly type: "string";
        };
        readonly is_linked: {
            readonly type: "boolean";
        };
        readonly is_refreshed: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["workspace_id", "path", "is_secret", "extra_perms"];
};
export declare const $ContextualVariable: {
    readonly type: "object";
    readonly properties: {
        readonly name: {
            readonly type: "string";
        };
        readonly value: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly is_custom: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["name", "value", "description", "is_custom"];
};
export declare const $CreateVariable: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly value: {
            readonly type: "string";
        };
        readonly is_secret: {
            readonly type: "boolean";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly account: {
            readonly type: "integer";
        };
        readonly is_oauth: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["path", "value", "is_secret", "description"];
};
export declare const $EditVariable: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly value: {
            readonly type: "string";
        };
        readonly is_secret: {
            readonly type: "boolean";
        };
        readonly description: {
            readonly type: "string";
        };
    };
};
export declare const $AuditLog: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "integer";
        };
        readonly timestamp: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly username: {
            readonly type: "string";
        };
        readonly operation: {
            readonly type: "string";
            readonly enum: readonly ["jobs.run", "jobs.run.script", "jobs.run.preview", "jobs.run.flow", "jobs.run.flow_preview", "jobs.run.script_hub", "jobs.run.dependencies", "jobs.run.identity", "jobs.run.noop", "jobs.flow_dependencies", "jobs", "jobs.cancel", "jobs.force_cancel", "jobs.disapproval", "jobs.delete", "account.delete", "openai.request", "resources.create", "resources.update", "resources.delete", "resource_types.create", "resource_types.update", "resource_types.delete", "schedule.create", "schedule.setenabled", "schedule.edit", "schedule.delete", "scripts.create", "scripts.update", "scripts.archive", "scripts.delete", "users.create", "users.delete", "users.update", "users.login", "users.logout", "users.accept_invite", "users.decline_invite", "users.token.create", "users.token.delete", "users.add_to_workspace", "users.add_global", "users.setpassword", "users.impersonate", "users.leave_workspace", "oauth.login", "oauth.signup", "variables.create", "variables.delete", "variables.update", "flows.create", "flows.update", "flows.delete", "flows.archive", "apps.create", "apps.update", "apps.delete", "folder.create", "folder.update", "folder.delete", "folder.add_owner", "folder.remove_owner", "group.create", "group.delete", "group.edit", "group.adduser", "group.removeuser", "igroup.create", "igroup.delete", "igroup.adduser", "igroup.removeuser", "variables.decrypt_secret", "workspaces.edit_command_script", "workspaces.edit_deploy_to", "workspaces.edit_auto_invite_domain", "workspaces.edit_webhook", "workspaces.edit_copilot_config", "workspaces.edit_error_handler", "workspaces.create", "workspaces.update", "workspaces.archive", "workspaces.unarchive", "workspaces.delete"];
        };
        readonly action_kind: {
            readonly type: "string";
            readonly enum: readonly ["Created", "Updated", "Delete", "Execute"];
        };
        readonly resource: {
            readonly type: "string";
        };
        readonly parameters: {
            readonly type: "object";
        };
    };
    readonly required: readonly ["id", "timestamp", "username", "operation", "action_kind"];
};
export declare const $MainArgSignature: {
    readonly type: "object";
    readonly properties: {
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["Valid", "Invalid"];
        };
        readonly error: {
            readonly type: "string";
        };
        readonly star_args: {
            readonly type: "boolean";
        };
        readonly star_kwargs: {
            readonly type: "boolean";
        };
        readonly args: {
            readonly type: "array";
            readonly items: {
                readonly type: "object";
                readonly properties: {
                    readonly name: {
                        readonly type: "string";
                    };
                    readonly typ: {
                        readonly oneOf: readonly [{
                            readonly type: "string";
                            readonly enum: readonly ["float", "int", "bool", "email", "unknown", "bytes", "dict", "datetime", "sql"];
                        }, {
                            readonly type: "object";
                            readonly properties: {
                                readonly resource: {
                                    readonly type: "string";
                                    readonly nullable: true;
                                };
                            };
                            readonly required: readonly ["resource"];
                        }, {
                            readonly type: "object";
                            readonly properties: {
                                readonly str: {
                                    readonly type: "array";
                                    readonly items: {
                                        readonly type: "string";
                                    };
                                    readonly nullable: true;
                                };
                            };
                            readonly required: readonly ["str"];
                        }, {
                            readonly type: "object";
                            readonly properties: {
                                readonly object: {
                                    readonly type: "array";
                                    readonly items: {
                                        readonly type: "object";
                                        readonly properties: {
                                            readonly key: {
                                                readonly type: "string";
                                            };
                                            readonly typ: {
                                                readonly oneOf: readonly [{
                                                    readonly type: "string";
                                                    readonly enum: readonly ["float", "int", "bool", "email", "unknown", "bytes", "dict", "datetime", "sql"];
                                                }, {
                                                    readonly type: "object";
                                                    readonly properties: {
                                                        readonly str: {};
                                                    };
                                                    readonly required: readonly ["str"];
                                                }];
                                            };
                                        };
                                        readonly required: readonly ["key", "typ"];
                                    };
                                };
                            };
                            readonly required: readonly ["object"];
                        }, {
                            readonly type: "object";
                            readonly properties: {
                                readonly list: {
                                    readonly oneOf: readonly [{
                                        readonly type: "string";
                                        readonly enum: readonly ["float", "int", "bool", "email", "unknown", "bytes", "dict", "datetime", "sql"];
                                    }, {
                                        readonly type: "object";
                                        readonly properties: {
                                            readonly str: {};
                                        };
                                        readonly required: readonly ["str"];
                                    }];
                                    readonly nullable: true;
                                };
                            };
                            readonly required: readonly ["list"];
                        }];
                    };
                    readonly has_default: {
                        readonly type: "boolean";
                    };
                    readonly default: {};
                };
                readonly required: readonly ["name", "typ"];
            };
        };
    };
    readonly required: readonly ["star_args", "start_kwargs", "args", "type", "error"];
};
export declare const $Preview: {
    readonly type: "object";
    readonly properties: {
        readonly content: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly kind: {
            readonly type: "string";
            readonly enum: readonly ["code", "identity", "http"];
        };
        readonly dedicated_worker: {
            readonly type: "boolean";
        };
        readonly lock: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["args"];
};
export declare const $WorkflowTask: {
    readonly type: "object";
    readonly properties: {
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
    };
    readonly required: readonly ["args"];
};
export declare const $WorkflowStatusRecord: {
    readonly type: "object";
    readonly additionalProperties: {
        readonly $ref: "#/components/schemas/WorkflowStatus";
    };
};
export declare const $WorkflowStatus: {
    readonly type: "object";
    readonly properties: {
        readonly scheduled_for: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly started_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly duration_ms: {
            readonly type: "number";
        };
        readonly name: {
            readonly type: "string";
        };
    };
};
export declare const $CreateResource: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly value: {};
        readonly description: {
            readonly type: "string";
        };
        readonly resource_type: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["path", "value", "resource_type"];
};
export declare const $EditResource: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly value: {};
    };
};
export declare const $Resource: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly resource_type: {
            readonly type: "string";
        };
        readonly value: {};
        readonly is_oauth: {
            readonly type: "boolean";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
    };
    readonly required: readonly ["path", "resource_type", "is_oauth"];
};
export declare const $ListableResource: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly resource_type: {
            readonly type: "string";
        };
        readonly value: {};
        readonly is_oauth: {
            readonly type: "boolean";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly is_expired: {
            readonly type: "boolean";
        };
        readonly refresh_error: {
            readonly type: "string";
        };
        readonly is_linked: {
            readonly type: "boolean";
        };
        readonly is_refreshed: {
            readonly type: "boolean";
        };
        readonly account: {
            readonly type: "number";
        };
    };
    readonly required: readonly ["path", "resource_type", "is_oauth", "is_linked", "is_refreshed"];
};
export declare const $ResourceType: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly schema: {};
        readonly description: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["name"];
};
export declare const $EditResourceType: {
    readonly type: "object";
    readonly properties: {
        readonly schema: {};
        readonly description: {
            readonly type: "string";
        };
    };
};
export declare const $Schedule: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly edited_by: {
            readonly type: "string";
        };
        readonly edited_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly schedule: {
            readonly type: "string";
        };
        readonly timezone: {
            readonly type: "string";
        };
        readonly enabled: {
            readonly type: "boolean";
        };
        readonly script_path: {
            readonly type: "string";
        };
        readonly is_flow: {
            readonly type: "boolean";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly email: {
            readonly type: "string";
        };
        readonly error: {
            readonly type: "string";
        };
        readonly on_failure: {
            readonly type: "string";
        };
        readonly on_failure_times: {
            readonly type: "number";
        };
        readonly on_failure_exact: {
            readonly type: "boolean";
        };
        readonly on_failure_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly on_recovery: {
            readonly type: "string";
        };
        readonly on_recovery_times: {
            readonly type: "number";
        };
        readonly on_recovery_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly retry: {
            readonly $ref: "#/components/schemas/Retry";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly no_flow_overlap: {
            readonly type: "boolean";
        };
        readonly tag: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["path", "edited_by", "edited_at", "schedule", "script_path", "timezone", "extra_perms", "is_flow", "enabled", "email"];
};
export declare const $ScheduleWJobs: {
    readonly allOf: readonly [{
        readonly $ref: "#/components/schemas/Schedule";
    }, {
        readonly type: "object";
        readonly properties: {
            readonly jobs: {
                readonly type: "array";
                readonly items: {
                    readonly type: "object";
                    readonly properties: {
                        readonly id: {
                            readonly type: "string";
                        };
                        readonly success: {
                            readonly type: "boolean";
                        };
                        readonly duration_ms: {
                            readonly type: "number";
                        };
                    };
                    readonly required: readonly ["id", "success", "duration_ms"];
                };
            };
        };
    }];
};
export declare const $NewSchedule: {
    readonly type: "object";
    readonly properties: {
        readonly path: {
            readonly type: "string";
        };
        readonly schedule: {
            readonly type: "string";
        };
        readonly timezone: {
            readonly type: "string";
        };
        readonly script_path: {
            readonly type: "string";
        };
        readonly is_flow: {
            readonly type: "boolean";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly enabled: {
            readonly type: "boolean";
        };
        readonly on_failure: {
            readonly type: "string";
        };
        readonly on_failure_times: {
            readonly type: "number";
        };
        readonly on_failure_exact: {
            readonly type: "boolean";
        };
        readonly on_failure_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly on_recovery: {
            readonly type: "string";
        };
        readonly on_recovery_times: {
            readonly type: "number";
        };
        readonly on_recovery_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly retry: {
            readonly $ref: "#/components/schemas/Retry";
        };
        readonly no_flow_overlap: {
            readonly type: "boolean";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly tag: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["path", "schedule", "timezone", "script_path", "is_flow", "args"];
};
export declare const $EditSchedule: {
    readonly type: "object";
    readonly properties: {
        readonly schedule: {
            readonly type: "string";
        };
        readonly timezone: {
            readonly type: "string";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly on_failure: {
            readonly type: "string";
        };
        readonly on_failure_times: {
            readonly type: "number";
        };
        readonly on_failure_exact: {
            readonly type: "boolean";
        };
        readonly on_failure_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly on_recovery: {
            readonly type: "string";
        };
        readonly on_recovery_times: {
            readonly type: "number";
        };
        readonly on_recovery_extra_args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly retry: {
            readonly $ref: "#/components/schemas/Retry";
        };
        readonly no_flow_overlap: {
            readonly type: "boolean";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly tag: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["schedule", "timezone", "script_path", "is_flow", "args"];
};
export declare const $Group: {
    readonly type: "object";
    readonly properties: {
        readonly name: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly members: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
    };
    readonly required: readonly ["name"];
};
export declare const $InstanceGroup: {
    readonly type: "object";
    readonly properties: {
        readonly name: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly emails: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["name"];
};
export declare const $Folder: {
    readonly type: "object";
    readonly properties: {
        readonly name: {
            readonly type: "string";
        };
        readonly owners: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
    };
    readonly required: readonly ["name", "owners", "extra_perms"];
};
export declare const $WorkerPing: {
    readonly type: "object";
    readonly properties: {
        readonly worker: {
            readonly type: "string";
        };
        readonly worker_instance: {
            readonly type: "string";
        };
        readonly last_ping: {
            readonly type: "number";
        };
        readonly started_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly ip: {
            readonly type: "string";
        };
        readonly jobs_executed: {
            readonly type: "integer";
        };
        readonly custom_tags: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly worker_group: {
            readonly type: "string";
        };
        readonly wm_version: {
            readonly type: "string";
        };
        readonly current_job_id: {
            readonly type: "string";
        };
        readonly current_job_workspace_id: {
            readonly type: "string";
        };
        readonly occupancy_rate: {
            readonly type: "number";
        };
    };
    readonly required: readonly ["worker", "worker_instance", "ping_at", "started_at", "ip", "jobs_executed", "worker_group", "wm_version"];
};
export declare const $UserWorkspaceList: {
    readonly type: "object";
    readonly properties: {
        readonly email: {
            readonly type: "string";
        };
        readonly workspaces: {
            readonly type: "array";
            readonly items: {
                readonly type: "object";
                readonly properties: {
                    readonly id: {
                        readonly type: "string";
                    };
                    readonly name: {
                        readonly type: "string";
                    };
                    readonly username: {
                        readonly type: "string";
                    };
                };
                readonly required: readonly ["id", "name", "username"];
            };
        };
    };
    readonly required: readonly ["email", "workspaces"];
};
export declare const $CreateWorkspace: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly username: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["id", "name"];
};
export declare const $Workspace: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly owner: {
            readonly type: "string";
        };
        readonly domain: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["id", "name", "owner"];
};
export declare const $WorkspaceInvite: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly email: {
            readonly type: "string";
        };
        readonly is_admin: {
            readonly type: "boolean";
        };
        readonly operator: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["workspace_id", "email", "is_admin", "operator"];
};
export declare const $GlobalUserInfo: {
    readonly type: "object";
    readonly properties: {
        readonly email: {
            readonly type: "string";
        };
        readonly login_type: {
            readonly type: "string";
            readonly enum: readonly ["password", "github"];
        };
        readonly super_admin: {
            readonly type: "boolean";
        };
        readonly verified: {
            readonly type: "boolean";
        };
        readonly name: {
            readonly type: "string";
        };
        readonly company: {
            readonly type: "string";
        };
        readonly username: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["email", "login_type", "super_admin", "verified"];
};
export declare const $Flow: {
    readonly allOf: readonly [{
        readonly $ref: "#/components/schemas/OpenFlow";
    }, {
        readonly $ref: "#/components/schemas/FlowMetadata";
    }];
};
export declare const $ExtraPerms: {
    readonly type: "object";
    readonly additionalProperties: {
        readonly type: "boolean";
    };
};
export declare const $FlowMetadata: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly edited_by: {
            readonly type: "string";
        };
        readonly edited_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly archived: {
            readonly type: "boolean";
        };
        readonly extra_perms: {
            readonly $ref: "#/components/schemas/ExtraPerms";
        };
        readonly starred: {
            readonly type: "boolean";
        };
        readonly draft_only: {
            readonly type: "boolean";
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly ws_error_handler_muted: {
            readonly type: "boolean";
        };
        readonly priority: {
            readonly type: "integer";
        };
        readonly dedicated_worker: {
            readonly type: "boolean";
        };
        readonly timeout: {
            readonly type: "number";
        };
        readonly visible_to_runner_only: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["path", "edited_by", "edited_at", "archived", "extra_perms"];
};
export declare const $OpenFlowWPath: {
    readonly allOf: readonly [{
        readonly $ref: "#/components/schemas/OpenFlow";
    }, {
        readonly type: "object";
        readonly properties: {
            readonly path: {
                readonly type: "string";
            };
            readonly tag: {
                readonly type: "string";
            };
            readonly ws_error_handler_muted: {
                readonly type: "boolean";
            };
            readonly priority: {
                readonly type: "integer";
            };
            readonly dedicated_worker: {
                readonly type: "boolean";
            };
            readonly timeout: {
                readonly type: "number";
            };
            readonly visible_to_runner_only: {
                readonly type: "boolean";
            };
        };
        readonly required: readonly ["path"];
    }];
};
export declare const $FlowPreview: {
    readonly type: "object";
    readonly properties: {
        readonly value: {
            readonly $ref: "#/components/schemas/FlowValue";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly args: {
            readonly $ref: "#/components/schemas/ScriptArgs";
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly restarted_from: {
            readonly $ref: "#/components/schemas/RestartedFrom";
        };
    };
    readonly required: readonly ["value", "content", "args"];
};
export declare const $RestartedFrom: {
    readonly type: "object";
    readonly properties: {
        readonly flow_job_id: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly step_id: {
            readonly type: "string";
        };
        readonly branch_or_iteration_n: {
            readonly type: "integer";
        };
    };
};
export declare const $Policy: {
    readonly type: "object";
    readonly properties: {
        readonly triggerables: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "object";
            };
        };
        readonly triggerables_v2: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "object";
            };
        };
        readonly execution_mode: {
            readonly type: "string";
            readonly enum: readonly ["viewer", "publisher", "anonymous"];
        };
        readonly on_behalf_of: {
            readonly type: "string";
        };
        readonly on_behalf_of_email: {
            readonly type: "string";
        };
    };
};
export declare const $ListableApp: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "integer";
        };
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly version: {
            readonly type: "integer";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly starred: {
            readonly type: "boolean";
        };
        readonly edited_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly execution_mode: {
            readonly type: "string";
            readonly enum: readonly ["viewer", "publisher", "anonymous"];
        };
    };
    readonly required: readonly ["id", "workspace_id", "path", "summary", "version", "extra_perms", "edited_at", "execution_mode"];
};
export declare const $ListableRawApp: {
    readonly type: "object";
    readonly properties: {
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
        readonly starred: {
            readonly type: "boolean";
        };
        readonly version: {
            readonly type: "number";
        };
        readonly edited_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
    };
    readonly required: readonly ["workspace_id", "path", "summary", "extra_perms", "version", "edited_at"];
};
export declare const $AppWithLastVersion: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "integer";
        };
        readonly workspace_id: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly versions: {
            readonly type: "array";
            readonly items: {
                readonly type: "integer";
            };
        };
        readonly created_by: {
            readonly type: "string";
        };
        readonly created_at: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly value: {
            readonly type: "object";
        };
        readonly policy: {
            readonly $ref: "#/components/schemas/Policy";
        };
        readonly execution_mode: {
            readonly type: "string";
            readonly enum: readonly ["viewer", "publisher", "anonymous"];
        };
        readonly extra_perms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly type: "boolean";
            };
        };
    };
    readonly required: readonly ["id", "workspace_id", "path", "summary", "versions", "created_by", "created_at", "value", "policy", "execution_mode", "extra_perms"];
};
export declare const $AppWithLastVersionWDraft: {
    readonly allOf: readonly [{
        readonly $ref: "#/components/schemas/AppWithLastVersion";
    }, {
        readonly type: "object";
        readonly properties: {
            readonly draft_only: {
                readonly type: "boolean";
            };
            readonly draft: {};
        };
    }];
};
export declare const $AppHistory: {
    readonly type: "object";
    readonly properties: {
        readonly version: {
            readonly type: "integer";
        };
        readonly deployment_msg: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["version"];
};
export declare const $SlackToken: {
    readonly type: "object";
    readonly properties: {
        readonly access_token: {
            readonly type: "string";
        };
        readonly team_id: {
            readonly type: "string";
        };
        readonly team_name: {
            readonly type: "string";
        };
        readonly bot: {
            readonly type: "object";
            readonly properties: {
                readonly bot_access_token: {
                    readonly type: "string";
                };
            };
        };
    };
    readonly required: readonly ["access_token", "team_id", "team_name", "bot"];
};
export declare const $TokenResponse: {
    readonly type: "object";
    readonly properties: {
        readonly access_token: {
            readonly type: "string";
        };
        readonly expires_in: {
            readonly type: "integer";
        };
        readonly refresh_token: {
            readonly type: "string";
        };
        readonly scope: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["access_token"];
};
export declare const $HubScriptKind: {
    readonly name: "kind";
    readonly schema: {
        readonly type: "string";
        readonly enum: readonly ["script", "failure", "trigger", "approval"];
    };
};
export declare const $PolarsClientKwargs: {
    readonly type: "object";
    readonly properties: {
        readonly region_name: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["region_name"];
};
export declare const $LargeFileStorage: {
    readonly type: "object";
    readonly properties: {
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["S3Storage", "AzureBlobStorage", "AzureWorkloadIdentity", "S3AwsOidc"];
        };
        readonly s3_resource_path: {
            readonly type: "string";
        };
        readonly azure_blob_resource_path: {
            readonly type: "string";
        };
        readonly public_resource: {
            readonly type: "boolean";
        };
    };
};
export declare const $WindmillLargeFile: {
    readonly type: "object";
    readonly properties: {
        readonly s3: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["s3"];
};
export declare const $WindmillFileMetadata: {
    readonly type: "object";
    readonly properties: {
        readonly mime_type: {
            readonly type: "string";
        };
        readonly size_in_bytes: {
            readonly type: "integer";
        };
        readonly last_modified: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly expires: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly version_id: {
            readonly type: "string";
        };
    };
};
export declare const $WindmillFilePreview: {
    readonly type: "object";
    readonly properties: {
        readonly msg: {
            readonly type: "string";
        };
        readonly content: {
            readonly type: "string";
        };
        readonly content_type: {
            readonly type: "string";
            readonly enum: readonly ["RawText", "Csv", "Parquet", "Unknown"];
        };
    };
    readonly required: readonly ["content_type"];
};
export declare const $S3Resource: {
    readonly type: "object";
    readonly properties: {
        readonly bucket: {
            readonly type: "string";
        };
        readonly region: {
            readonly type: "string";
        };
        readonly endPoint: {
            readonly type: "string";
        };
        readonly useSSL: {
            readonly type: "boolean";
        };
        readonly accessKey: {
            readonly type: "string";
        };
        readonly secretKey: {
            readonly type: "string";
        };
        readonly pathStyle: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["bucket", "region", "endPoint", "useSSL", "pathStyle"];
};
export declare const $WorkspaceGitSyncSettings: {
    readonly type: "object";
    readonly properties: {
        readonly include_path: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly include_type: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
                readonly enum: readonly ["script", "flow", "app", "folder", "resource", "variable", "secret", "resourcetype", "schedule", "user", "group"];
            };
        };
        readonly repositories: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/GitRepositorySettings";
            };
        };
    };
};
export declare const $WorkspaceDefaultScripts: {
    readonly type: "object";
    readonly properties: {
        readonly order: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly hidden: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly default_script_content: {
            readonly additionalProperties: {
                readonly type: "string";
            };
        };
    };
};
export declare const $GitRepositorySettings: {
    readonly type: "object";
    readonly properties: {
        readonly script_path: {
            readonly type: "string";
        };
        readonly git_repo_resource_path: {
            readonly type: "string";
        };
        readonly use_individual_branch: {
            readonly type: "boolean";
        };
        readonly group_by_folder: {
            readonly type: "boolean";
        };
        readonly exclude_types_override: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
                readonly enum: readonly ["script", "flow", "app", "folder", "resource", "variable", "secret", "resourcetype", "schedule", "user", "group"];
            };
        };
    };
    readonly required: readonly ["script_path", "git_repo_resource_path"];
};
export declare const $UploadFilePart: {
    readonly type: "object";
    readonly properties: {
        readonly part_number: {
            readonly type: "integer";
        };
        readonly tag: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["part_number", "tag"];
};
export declare const $MetricMetadata: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly name: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["id"];
};
export declare const $ScalarMetric: {
    readonly type: "object";
    readonly properties: {
        readonly metric_id: {
            readonly type: "string";
        };
        readonly value: {
            readonly type: "number";
        };
    };
    readonly required: readonly ["id", "value"];
};
export declare const $TimeseriesMetric: {
    readonly type: "object";
    readonly properties: {
        readonly metric_id: {
            readonly type: "string";
        };
        readonly values: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/MetricDataPoint";
            };
        };
    };
    readonly required: readonly ["id", "values"];
};
export declare const $MetricDataPoint: {
    readonly type: "object";
    readonly properties: {
        readonly timestamp: {
            readonly type: "string";
            readonly format: "date-time";
        };
        readonly value: {
            readonly type: "number";
        };
    };
    readonly required: readonly ["timestamp", "value"];
};
export declare const $RawScriptForDependencies: {
    readonly type: "object";
    readonly properties: {
        readonly raw_code: {
            readonly type: "string";
        };
        readonly path: {
            readonly type: "string";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["python3", "deno", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets", "bun"];
        };
    };
    readonly required: readonly ["raw_code", "path", "language"];
};
export declare const $ConcurrencyGroup: {
    readonly type: "object";
    readonly properties: {
        readonly concurrency_id: {
            readonly type: "string";
        };
        readonly job_uuids: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
    };
    readonly required: readonly ["concurrency_id", "job_uuids"];
};
export declare const $OpenFlow: {
    readonly type: "object";
    readonly properties: {
        readonly summary: {
            readonly type: "string";
        };
        readonly description: {
            readonly type: "string";
        };
        readonly value: {
            readonly $ref: "#/components/schemas/FlowValue";
        };
        readonly schema: {
            readonly type: "object";
        };
    };
    readonly required: readonly ["summary", "value"];
};
export declare const $FlowValue: {
    readonly type: "object";
    readonly properties: {
        readonly modules: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/FlowModule";
            };
        };
        readonly failure_module: {
            readonly $ref: "#/components/schemas/FlowModule";
        };
        readonly same_worker: {
            readonly type: "boolean";
        };
        readonly concurrent_limit: {
            readonly type: "number";
        };
        readonly concurrency_key: {
            readonly type: "string";
        };
        readonly concurrency_time_window_s: {
            readonly type: "number";
        };
        readonly skip_expr: {
            readonly type: "string";
        };
        readonly cache_ttl: {
            readonly type: "number";
        };
        readonly priority: {
            readonly type: "number";
        };
        readonly early_return: {
            readonly type: "string";
        };
    };
    readonly required: readonly ["modules"];
};
export declare const $Retry: {
    readonly type: "object";
    readonly properties: {
        readonly constant: {
            readonly type: "object";
            readonly properties: {
                readonly attempts: {
                    readonly type: "integer";
                };
                readonly seconds: {
                    readonly type: "integer";
                };
            };
        };
        readonly exponential: {
            readonly type: "object";
            readonly properties: {
                readonly attempts: {
                    readonly type: "integer";
                };
                readonly multiplier: {
                    readonly type: "integer";
                };
                readonly seconds: {
                    readonly type: "integer";
                };
                readonly random_factor: {
                    readonly type: "integer";
                    readonly minimum: 0;
                    readonly maximum: 100;
                };
            };
        };
    };
};
export declare const $FlowModule: {
    readonly type: "object";
    readonly properties: {
        readonly id: {
            readonly type: "string";
        };
        readonly value: {
            readonly $ref: "#/components/schemas/FlowModuleValue";
        };
        readonly stop_after_if: {
            readonly type: "object";
            readonly properties: {
                readonly skip_if_stopped: {
                    readonly type: "boolean";
                };
                readonly expr: {
                    readonly type: "string";
                };
            };
            readonly required: readonly ["expr"];
        };
        readonly sleep: {
            readonly $ref: "#/components/schemas/InputTransform";
        };
        readonly cache_ttl: {
            readonly type: "number";
        };
        readonly timeout: {
            readonly type: "number";
        };
        readonly delete_after_use: {
            readonly type: "boolean";
        };
        readonly summary: {
            readonly type: "string";
        };
        readonly mock: {
            readonly type: "object";
            readonly properties: {
                readonly enabled: {
                    readonly type: "boolean";
                };
                readonly return_value: {};
            };
        };
        readonly suspend: {
            readonly type: "object";
            readonly properties: {
                readonly required_events: {
                    readonly type: "integer";
                };
                readonly timeout: {
                    readonly type: "integer";
                };
                readonly resume_form: {
                    readonly type: "object";
                    readonly properties: {
                        readonly schema: {
                            readonly type: "object";
                        };
                    };
                };
                readonly user_auth_required: {
                    readonly type: "boolean";
                };
                readonly user_groups_required: {
                    readonly $ref: "#/components/schemas/InputTransform";
                };
                readonly self_approval_disabled: {
                    readonly type: "boolean";
                };
                readonly hide_cancel: {
                    readonly type: "boolean";
                };
            };
        };
        readonly priority: {
            readonly type: "number";
        };
        readonly continue_on_error: {
            readonly type: "boolean";
        };
        readonly retry: {
            readonly $ref: "#/components/schemas/Retry";
        };
    };
    readonly required: readonly ["value", "id"];
};
export declare const $InputTransform: {
    readonly oneOf: readonly [{
        readonly $ref: "#/components/schemas/StaticTransform";
    }, {
        readonly $ref: "#/components/schemas/JavascriptTransform";
    }];
    readonly discriminator: {
        readonly propertyName: "type";
        readonly mapping: {
            readonly static: "#/components/schemas/StaticTransform";
            readonly javascript: "#/components/schemas/JavascriptTransform";
        };
    };
};
export declare const $StaticTransform: {
    readonly type: "object";
    readonly properties: {
        readonly value: {};
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["javascript"];
        };
    };
    readonly required: readonly ["expr", "type"];
};
export declare const $JavascriptTransform: {
    readonly type: "object";
    readonly properties: {
        readonly expr: {
            readonly type: "string";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["javascript"];
        };
    };
    readonly required: readonly ["expr", "type"];
};
export declare const $FlowModuleValue: {
    readonly oneOf: readonly [{
        readonly $ref: "#/components/schemas/RawScript";
    }, {
        readonly $ref: "#/components/schemas/PathScript";
    }, {
        readonly $ref: "#/components/schemas/PathFlow";
    }, {
        readonly $ref: "#/components/schemas/ForloopFlow";
    }, {
        readonly $ref: "#/components/schemas/WhileloopFlow";
    }, {
        readonly $ref: "#/components/schemas/BranchOne";
    }, {
        readonly $ref: "#/components/schemas/BranchAll";
    }, {
        readonly $ref: "#/components/schemas/Identity";
    }];
    readonly discriminator: {
        readonly propertyName: "type";
        readonly mapping: {
            readonly rawscript: "#/components/schemas/RawScript";
            readonly script: "#/components/schemas/PathScript";
            readonly flow: "#/components/schemas/PathFlow";
            readonly forloopflow: "#/components/schemas/ForloopFlow";
            readonly whileloopflow: "#/components/schemas/WhileloopFlow";
            readonly branchone: "#/components/schemas/BranchOne";
            readonly branchall: "#/components/schemas/BranchAll";
            readonly identity: "#/components/schemas/Identity";
        };
    };
};
export declare const $RawScript: {
    readonly type: "object";
    readonly properties: {
        readonly input_transforms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly $ref: "#/components/schemas/InputTransform";
            };
        };
        readonly content: {
            readonly type: "string";
        };
        readonly language: {
            readonly type: "string";
            readonly enum: readonly ["deno", "bun", "python3", "go", "bash", "powershell", "postgresql", "mysql", "bigquery", "snowflake", "mssql", "graphql", "nativets"];
        };
        readonly path: {
            readonly type: "string";
        };
        readonly lock: {
            readonly type: "string";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["rawscript"];
        };
        readonly tag: {
            readonly type: "string";
        };
        readonly concurrent_limit: {
            readonly type: "number";
        };
        readonly concurrency_time_window_s: {
            readonly type: "number";
        };
    };
    readonly required: readonly ["type", "content", "language", "input_transforms"];
};
export declare const $PathScript: {
    readonly type: "object";
    readonly properties: {
        readonly input_transforms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly $ref: "#/components/schemas/InputTransform";
            };
        };
        readonly path: {
            readonly type: "string";
        };
        readonly hash: {
            readonly type: "string";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["script"];
        };
    };
    readonly required: readonly ["type", "path", "input_transforms"];
};
export declare const $PathFlow: {
    readonly type: "object";
    readonly properties: {
        readonly input_transforms: {
            readonly type: "object";
            readonly additionalProperties: {
                readonly $ref: "#/components/schemas/InputTransform";
            };
        };
        readonly path: {
            readonly type: "string";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["flow"];
        };
    };
    readonly required: readonly ["type", "path", "input_transforms"];
};
export declare const $ForloopFlow: {
    readonly type: "object";
    readonly properties: {
        readonly modules: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/FlowModule";
            };
        };
        readonly iterator: {
            readonly $ref: "#/components/schemas/InputTransform";
        };
        readonly skip_failures: {
            readonly type: "boolean";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["forloopflow"];
        };
        readonly parallel: {
            readonly type: "boolean";
        };
        readonly parallelism: {
            readonly type: "integer";
        };
    };
    readonly required: readonly ["modules", "iterator", "skip_failures", "type"];
};
export declare const $WhileloopFlow: {
    readonly type: "object";
    readonly properties: {
        readonly modules: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/FlowModule";
            };
        };
        readonly skip_failures: {
            readonly type: "boolean";
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["forloopflow"];
        };
        readonly parallel: {
            readonly type: "boolean";
        };
        readonly parallelism: {
            readonly type: "integer";
        };
    };
    readonly required: readonly ["modules", "skip_failures", "type"];
};
export declare const $BranchOne: {
    readonly type: "object";
    readonly properties: {
        readonly branches: {
            readonly type: "array";
            readonly items: {
                readonly type: "object";
                readonly properties: {
                    readonly summary: {
                        readonly type: "string";
                    };
                    readonly expr: {
                        readonly type: "string";
                    };
                    readonly modules: {
                        readonly type: "array";
                        readonly items: {
                            readonly $ref: "#/components/schemas/FlowModule";
                        };
                    };
                };
                readonly required: readonly ["modules", "expr"];
            };
        };
        readonly default: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/FlowModule";
            };
            readonly required: readonly ["modules"];
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["branchone"];
        };
    };
    readonly required: readonly ["branches", "default", "type"];
};
export declare const $BranchAll: {
    readonly type: "object";
    readonly properties: {
        readonly branches: {
            readonly type: "array";
            readonly items: {
                readonly type: "object";
                readonly properties: {
                    readonly summary: {
                        readonly type: "string";
                    };
                    readonly skip_failure: {
                        readonly type: "boolean";
                    };
                    readonly modules: {
                        readonly type: "array";
                        readonly items: {
                            readonly $ref: "#/components/schemas/FlowModule";
                        };
                    };
                };
                readonly required: readonly ["modules", "expr"];
            };
        };
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["branchall"];
        };
        readonly parallel: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["branches", "type"];
};
export declare const $Identity: {
    readonly type: "object";
    readonly properties: {
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["identity"];
        };
        readonly flow: {
            readonly type: "boolean";
        };
    };
    readonly required: readonly ["type"];
};
export declare const $FlowStatus: {
    readonly type: "object";
    readonly properties: {
        readonly step: {
            readonly type: "integer";
        };
        readonly modules: {
            readonly type: "array";
            readonly items: {
                readonly $ref: "#/components/schemas/FlowStatusModule";
            };
        };
        readonly user_states: {
            readonly additionalProperties: true;
        };
        readonly failure_module: {
            readonly allOf: readonly [{
                readonly $ref: "#/components/schemas/FlowStatusModule";
            }, {
                readonly type: "object";
                readonly properties: {
                    readonly parent_module: {
                        readonly type: "string";
                    };
                };
            }];
        };
        readonly retry: {
            readonly type: "object";
            readonly properties: {
                readonly fail_count: {
                    readonly type: "integer";
                };
                readonly failed_jobs: {
                    readonly type: "array";
                    readonly items: {
                        readonly type: "string";
                        readonly format: "uuid";
                    };
                };
            };
        };
    };
    readonly required: readonly ["step", "modules", "failure_module"];
};
export declare const $FlowStatusModule: {
    readonly type: "object";
    readonly properties: {
        readonly type: {
            readonly type: "string";
            readonly enum: readonly ["WaitingForPriorSteps", "WaitingForEvents", "WaitingForExecutor", "InProgress", "Success", "Failure"];
        };
        readonly id: {
            readonly type: "string";
        };
        readonly job: {
            readonly type: "string";
            readonly format: "uuid";
        };
        readonly count: {
            readonly type: "integer";
        };
        readonly iterator: {
            readonly type: "object";
            readonly properties: {
                readonly index: {
                    readonly type: "integer";
                };
                readonly itered: {
                    readonly type: "array";
                    readonly items: {};
                };
                readonly args: {};
            };
        };
        readonly flow_jobs: {
            readonly type: "array";
            readonly items: {
                readonly type: "string";
            };
        };
        readonly branch_chosen: {
            readonly type: "object";
            readonly properties: {
                readonly type: {
                    readonly type: "string";
                    readonly enum: readonly ["branch", "default"];
                };
                readonly branch: {
                    readonly type: "integer";
                };
            };
            readonly required: readonly ["type"];
        };
        readonly branchall: {
            readonly type: "object";
            readonly properties: {
                readonly branch: {
                    readonly type: "integer";
                };
                readonly len: {
                    readonly type: "integer";
                };
            };
            readonly required: readonly ["branch", "len"];
        };
        readonly approvers: {
            readonly type: "array";
            readonly items: {
                readonly type: "object";
                readonly properties: {
                    readonly resume_id: {
                        readonly type: "integer";
                    };
                    readonly approver: {
                        readonly type: "string";
                    };
                };
                readonly required: readonly ["resume_id", "approver"];
            };
        };
    };
    readonly required: readonly ["type"];
};
