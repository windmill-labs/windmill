// src/core/ApiError.ts
var ApiError = class extends Error {
  constructor(request2, response, message) {
    super(message);
    this.name = "ApiError";
    this.url = response.url;
    this.status = response.status;
    this.statusText = response.statusText;
    this.body = response.body;
    this.request = request2;
  }
};

// src/core/CancelablePromise.ts
var CancelError = class extends Error {
  constructor(message) {
    super(message);
    this.name = "CancelError";
  }
  get isCancelled() {
    return true;
  }
};
var CancelablePromise = class {
  constructor(executor) {
    this._isResolved = false;
    this._isRejected = false;
    this._isCancelled = false;
    this.cancelHandlers = [];
    this.promise = new Promise((resolve2, reject) => {
      this._resolve = resolve2;
      this._reject = reject;
      const onResolve = (value) => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this._isResolved = true;
        if (this._resolve) this._resolve(value);
      };
      const onReject = (reason) => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this._isRejected = true;
        if (this._reject) this._reject(reason);
      };
      const onCancel = (cancelHandler) => {
        if (this._isResolved || this._isRejected || this._isCancelled) {
          return;
        }
        this.cancelHandlers.push(cancelHandler);
      };
      Object.defineProperty(onCancel, "isResolved", {
        get: () => this._isResolved,
      });
      Object.defineProperty(onCancel, "isRejected", {
        get: () => this._isRejected,
      });
      Object.defineProperty(onCancel, "isCancelled", {
        get: () => this._isCancelled,
      });
      return executor(onResolve, onReject, onCancel);
    });
  }
  get [Symbol.toStringTag]() {
    return "Cancellable Promise";
  }
  then(onFulfilled, onRejected) {
    return this.promise.then(onFulfilled, onRejected);
  }
  catch(onRejected) {
    return this.promise.catch(onRejected);
  }
  finally(onFinally) {
    return this.promise.finally(onFinally);
  }
  cancel() {
    if (this._isResolved || this._isRejected || this._isCancelled) {
      return;
    }
    this._isCancelled = true;
    if (this.cancelHandlers.length) {
      try {
        for (const cancelHandler of this.cancelHandlers) {
          cancelHandler();
        }
      } catch (error) {
        console.warn("Cancellation threw an error", error);
        return;
      }
    }
    this.cancelHandlers.length = 0;
    if (this._reject) this._reject(new CancelError("Request aborted"));
  }
  get isCancelled() {
    return this._isCancelled;
  }
};

// src/core/OpenAPI.ts
var Interceptors = class {
  constructor() {
    this._fns = [];
  }
  eject(fn) {
    const index = this._fns.indexOf(fn);
    if (index !== -1) {
      this._fns = [...this._fns.slice(0, index), ...this._fns.slice(index + 1)];
    }
  }
  use(fn) {
    this._fns = [...this._fns, fn];
  }
};
var OpenAPI = {
  BASE: "/api",
  CREDENTIALS: "include",
  ENCODE_PATH: void 0,
  HEADERS: void 0,
  PASSWORD: void 0,
  TOKEN: void 0,
  USERNAME: void 0,
  VERSION: "1.326.0",
  WITH_CREDENTIALS: false,
  interceptors: {
    request: new Interceptors(),
    response: new Interceptors(),
  },
};

// src/schemas.gen.ts
var $Script = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    hash: {
      type: "string",
    },
    path: {
      type: "string",
    },
    parent_hashes: {
      type: "array",
      description: `The first element is the direct parent of the script, the second is the parent of the first, etc
`,
      items: {
        type: "string",
      },
    },
    summary: {
      type: "string",
    },
    description: {
      type: "string",
    },
    content: {
      type: "string",
    },
    created_by: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    archived: {
      type: "boolean",
    },
    schema: {
      type: "object",
    },
    deleted: {
      type: "boolean",
    },
    is_template: {
      type: "boolean",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    lock: {
      type: "string",
    },
    lock_error_logs: {
      type: "string",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
    kind: {
      type: "string",
      enum: ["script", "failure", "trigger", "command", "approval"],
    },
    starred: {
      type: "boolean",
    },
    tag: {
      type: "string",
    },
    has_draft: {
      type: "boolean",
    },
    draft_only: {
      type: "boolean",
    },
    envs: {
      type: "array",
      items: {
        type: "string",
      },
    },
    concurrent_limit: {
      type: "integer",
    },
    concurrency_time_window_s: {
      type: "integer",
    },
    cache_ttl: {
      type: "number",
    },
    dedicated_worker: {
      type: "boolean",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    priority: {
      type: "integer",
    },
    restart_unless_cancelled: {
      type: "boolean",
    },
    timeout: {
      type: "integer",
    },
    delete_after_use: {
      type: "boolean",
    },
    visible_to_runner_only: {
      type: "boolean",
    },
    no_main_func: {
      type: "boolean",
    },
    codebase: {
      type: "string",
    },
  },
  required: [
    "hash",
    "path",
    "summary",
    "description",
    "content",
    "created_by",
    "created_at",
    "archived",
    "deleted",
    "is_template",
    "extra_perms",
    "language",
    "kind",
    "starred",
    "no_main_func",
  ],
};
var $NewScript = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    parent_hash: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    description: {
      type: "string",
    },
    content: {
      type: "string",
    },
    schema: {
      type: "object",
    },
    is_template: {
      type: "boolean",
    },
    lock: {
      type: "string",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
    kind: {
      type: "string",
      enum: ["script", "failure", "trigger", "command", "approval"],
    },
    tag: {
      type: "string",
    },
    draft_only: {
      type: "boolean",
    },
    envs: {
      type: "array",
      items: {
        type: "string",
      },
    },
    concurrent_limit: {
      type: "integer",
    },
    concurrency_time_window_s: {
      type: "integer",
    },
    cache_ttl: {
      type: "number",
    },
    dedicated_worker: {
      type: "boolean",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    priority: {
      type: "integer",
    },
    restart_unless_cancelled: {
      type: "boolean",
    },
    timeout: {
      type: "integer",
    },
    delete_after_use: {
      type: "boolean",
    },
    deployment_message: {
      type: "string",
    },
    concurrency_key: {
      type: "string",
    },
    visible_to_runner_only: {
      type: "boolean",
    },
    no_main_func: {
      type: "boolean",
    },
    codebase: {
      type: "string",
    },
  },
  required: ["path", "summary", "description", "content", "language"],
};
var $NewScriptWithDraft = {
  allOf: [
    {
      $ref: "#/components/schemas/NewScript",
    },
    {
      type: "object",
      properties: {
        draft: {
          $ref: "#/components/schemas/NewScript",
        },
        hash: {
          type: "string",
        },
      },
      required: ["hash"],
    },
  ],
};
var $ScriptHistory = {
  type: "object",
  properties: {
    script_hash: {
      type: "string",
    },
    deployment_msg: {
      type: "string",
    },
  },
  required: ["script_hash"],
};
var $ScriptArgs = {
  type: "object",
  additionalProperties: {},
};
var $Input = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    name: {
      type: "string",
    },
    args: {
      type: "object",
    },
    created_by: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    is_public: {
      type: "boolean",
    },
    success: {
      type: "boolean",
    },
  },
  required: ["id", "name", "args", "created_by", "created_at", "is_public"],
};
var $CreateInput = {
  type: "object",
  properties: {
    name: {
      type: "string",
    },
    args: {
      type: "object",
    },
  },
  required: ["name", "args", "created_by"],
};
var $UpdateInput = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    name: {
      type: "string",
    },
    is_public: {
      type: "boolean",
    },
  },
  required: ["id", "name", "is_public"],
};
var $RunnableType = {
  type: "string",
  enum: ["ScriptHash", "ScriptPath", "FlowPath"],
};
var $QueuedJob = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    id: {
      type: "string",
      format: "uuid",
    },
    parent_job: {
      type: "string",
      format: "uuid",
    },
    created_by: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    started_at: {
      type: "string",
      format: "date-time",
    },
    scheduled_for: {
      type: "string",
      format: "date-time",
    },
    running: {
      type: "boolean",
    },
    script_path: {
      type: "string",
    },
    script_hash: {
      type: "string",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    logs: {
      type: "string",
    },
    raw_code: {
      type: "string",
    },
    canceled: {
      type: "boolean",
    },
    canceled_by: {
      type: "string",
    },
    canceled_reason: {
      type: "string",
    },
    last_ping: {
      type: "string",
      format: "date-time",
    },
    job_kind: {
      type: "string",
      enum: [
        "script",
        "preview",
        "dependencies",
        "flowdependencies",
        "appdependencies",
        "flow",
        "flowpreview",
        "script_hub",
        "identity",
        "deploymentcallback",
        "singlescriptflow",
        "flowscript",
        "flownode",
        "appscript",
      ],
    },
    schedule_path: {
      type: "string",
    },
    permissioned_as: {
      type: "string",
      description: `The user (u/userfoo) or group (g/groupfoo) whom 
the execution of this script will be permissioned_as and by extension its DT_TOKEN.
`,
    },
    flow_status: {
      $ref: "#/components/schemas/FlowStatus",
    },
    raw_flow: {
      $ref: "#/components/schemas/FlowValue",
    },
    is_flow_step: {
      type: "boolean",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
    email: {
      type: "string",
    },
    visible_to_owner: {
      type: "boolean",
    },
    mem_peak: {
      type: "integer",
    },
    tag: {
      type: "string",
    },
    priority: {
      type: "integer",
    },
  },
  required: [
    "id",
    "running",
    "canceled",
    "job_kind",
    "permissioned_as",
    "is_flow_step",
    "email",
    "visible_to_owner",
    "tag",
  ],
};
var $CompletedJob = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    id: {
      type: "string",
      format: "uuid",
    },
    parent_job: {
      type: "string",
      format: "uuid",
    },
    created_by: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    started_at: {
      type: "string",
      format: "date-time",
    },
    duration_ms: {
      type: "integer",
    },
    success: {
      type: "boolean",
    },
    script_path: {
      type: "string",
    },
    script_hash: {
      type: "string",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    result: {},
    logs: {
      type: "string",
    },
    deleted: {
      type: "boolean",
    },
    raw_code: {
      type: "string",
    },
    canceled: {
      type: "boolean",
    },
    canceled_by: {
      type: "string",
    },
    canceled_reason: {
      type: "string",
    },
    job_kind: {
      type: "string",
      enum: [
        "script",
        "preview",
        "dependencies",
        "flow",
        "flowdependencies",
        "appdependencies",
        "flowpreview",
        "script_hub",
        "identity",
        "deploymentcallback",
        "singlescriptflow",
        "flowscript",
        "flownode",
        "appscript",
      ],
    },
    schedule_path: {
      type: "string",
    },
    permissioned_as: {
      type: "string",
      description: `The user (u/userfoo) or group (g/groupfoo) whom 
the execution of this script will be permissioned_as and by extension its DT_TOKEN.
`,
    },
    flow_status: {
      $ref: "#/components/schemas/FlowStatus",
    },
    raw_flow: {
      $ref: "#/components/schemas/FlowValue",
    },
    is_flow_step: {
      type: "boolean",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
    is_skipped: {
      type: "boolean",
    },
    email: {
      type: "string",
    },
    visible_to_owner: {
      type: "boolean",
    },
    mem_peak: {
      type: "integer",
    },
    tag: {
      type: "string",
    },
    priority: {
      type: "integer",
    },
    labels: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: [
    "id",
    "created_by",
    "duration_ms",
    "created_at",
    "started_at",
    "success",
    "canceled",
    "job_kind",
    "permissioned_as",
    "is_flow_step",
    "is_skipped",
    "email",
    "visible_to_owner",
    "tag",
  ],
};
var $Job = {
  oneOf: [
    {
      allOf: [
        {
          $ref: "#/components/schemas/CompletedJob",
        },
        {
          type: "object",
          properties: {
            type: {
              type: "string",
              enum: ["CompletedJob"],
            },
          },
        },
      ],
    },
    {
      allOf: [
        {
          $ref: "#/components/schemas/QueuedJob",
        },
        {
          type: "object",
          properties: {
            type: {
              type: "string",
              enum: ["QueuedJob"],
            },
          },
        },
      ],
    },
  ],
  discriminator: {
    propertyName: "type",
  },
};
var $User = {
  type: "object",
  properties: {
    email: {
      type: "string",
    },
    username: {
      type: "string",
    },
    is_admin: {
      type: "boolean",
    },
    is_super_admin: {
      type: "boolean",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    operator: {
      type: "boolean",
    },
    disabled: {
      type: "boolean",
    },
    groups: {
      type: "array",
      items: {
        type: "string",
      },
    },
    folders: {
      type: "array",
      items: {
        type: "string",
      },
    },
    folders_owners: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: [
    "email",
    "username",
    "is_admin",
    "is_super_admin",
    "created_at",
    "operator",
    "disabled",
    "folders",
    "folders_owners",
  ],
};
var $UserUsage = {
  type: "object",
  properties: {
    email: {
      type: "string",
    },
    executions: {
      type: "number",
    },
  },
};
var $Login = {
  type: "object",
  properties: {
    email: {
      type: "string",
    },
    password: {
      type: "string",
    },
  },
  required: ["email", "password"],
};
var $EditWorkspaceUser = {
  type: "object",
  properties: {
    is_admin: {
      type: "boolean",
    },
    operator: {
      type: "boolean",
    },
    disabled: {
      type: "boolean",
    },
  },
};
var $TruncatedToken = {
  type: "object",
  properties: {
    label: {
      type: "string",
    },
    expiration: {
      type: "string",
      format: "date-time",
    },
    token_prefix: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    last_used_at: {
      type: "string",
      format: "date-time",
    },
    scopes: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: ["token_prefix", "created_at", "last_used_at"],
};
var $NewToken = {
  type: "object",
  properties: {
    label: {
      type: "string",
    },
    expiration: {
      type: "string",
      format: "date-time",
    },
    scopes: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
};
var $NewTokenImpersonate = {
  type: "object",
  properties: {
    label: {
      type: "string",
    },
    expiration: {
      type: "string",
      format: "date-time",
    },
    impersonate_email: {
      type: "string",
    },
  },
  required: ["impersonate_email"],
};
var $ListableVariable = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    value: {
      type: "string",
    },
    is_secret: {
      type: "boolean",
    },
    description: {
      type: "string",
    },
    account: {
      type: "integer",
    },
    is_oauth: {
      type: "boolean",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    is_expired: {
      type: "boolean",
    },
    refresh_error: {
      type: "string",
    },
    is_linked: {
      type: "boolean",
    },
    is_refreshed: {
      type: "boolean",
    },
  },
  required: ["workspace_id", "path", "is_secret", "extra_perms"],
};
var $ContextualVariable = {
  type: "object",
  properties: {
    name: {
      type: "string",
    },
    value: {
      type: "string",
    },
    description: {
      type: "string",
    },
    is_custom: {
      type: "boolean",
    },
  },
  required: ["name", "value", "description", "is_custom"],
};
var $CreateVariable = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    value: {
      type: "string",
    },
    is_secret: {
      type: "boolean",
    },
    description: {
      type: "string",
    },
    account: {
      type: "integer",
    },
    is_oauth: {
      type: "boolean",
    },
  },
  required: ["path", "value", "is_secret", "description"],
};
var $EditVariable = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    value: {
      type: "string",
    },
    is_secret: {
      type: "boolean",
    },
    description: {
      type: "string",
    },
  },
};
var $AuditLog = {
  type: "object",
  properties: {
    id: {
      type: "integer",
    },
    timestamp: {
      type: "string",
      format: "date-time",
    },
    username: {
      type: "string",
    },
    operation: {
      type: "string",
      enum: [
        "jobs.run",
        "jobs.run.script",
        "jobs.run.preview",
        "jobs.run.flow",
        "jobs.run.flow_preview",
        "jobs.run.script_hub",
        "jobs.run.dependencies",
        "jobs.run.identity",
        "jobs.run.noop",
        "jobs.flow_dependencies",
        "jobs",
        "jobs.cancel",
        "jobs.force_cancel",
        "jobs.disapproval",
        "jobs.delete",
        "account.delete",
        "openai.request",
        "resources.create",
        "resources.update",
        "resources.delete",
        "resource_types.create",
        "resource_types.update",
        "resource_types.delete",
        "schedule.create",
        "schedule.setenabled",
        "schedule.edit",
        "schedule.delete",
        "scripts.create",
        "scripts.update",
        "scripts.archive",
        "scripts.delete",
        "users.create",
        "users.delete",
        "users.update",
        "users.login",
        "users.logout",
        "users.accept_invite",
        "users.decline_invite",
        "users.token.create",
        "users.token.delete",
        "users.add_to_workspace",
        "users.add_global",
        "users.setpassword",
        "users.impersonate",
        "users.leave_workspace",
        "oauth.login",
        "oauth.signup",
        "variables.create",
        "variables.delete",
        "variables.update",
        "flows.create",
        "flows.update",
        "flows.delete",
        "flows.archive",
        "apps.create",
        "apps.update",
        "apps.delete",
        "folder.create",
        "folder.update",
        "folder.delete",
        "folder.add_owner",
        "folder.remove_owner",
        "group.create",
        "group.delete",
        "group.edit",
        "group.adduser",
        "group.removeuser",
        "igroup.create",
        "igroup.delete",
        "igroup.adduser",
        "igroup.removeuser",
        "variables.decrypt_secret",
        "workspaces.edit_command_script",
        "workspaces.edit_deploy_to",
        "workspaces.edit_auto_invite_domain",
        "workspaces.edit_webhook",
        "workspaces.edit_copilot_config",
        "workspaces.edit_error_handler",
        "workspaces.create",
        "workspaces.update",
        "workspaces.archive",
        "workspaces.unarchive",
        "workspaces.delete",
      ],
    },
    action_kind: {
      type: "string",
      enum: ["Created", "Updated", "Delete", "Execute"],
    },
    resource: {
      type: "string",
    },
    parameters: {
      type: "object",
    },
  },
  required: ["id", "timestamp", "username", "operation", "action_kind"],
};
var $MainArgSignature = {
  type: "object",
  properties: {
    type: {
      type: "string",
      enum: ["Valid", "Invalid"],
    },
    error: {
      type: "string",
    },
    star_args: {
      type: "boolean",
    },
    star_kwargs: {
      type: "boolean",
    },
    args: {
      type: "array",
      items: {
        type: "object",
        properties: {
          name: {
            type: "string",
          },
          typ: {
            oneOf: [
              {
                type: "string",
                enum: [
                  "float",
                  "int",
                  "bool",
                  "email",
                  "unknown",
                  "bytes",
                  "dict",
                  "datetime",
                  "sql",
                ],
              },
              {
                type: "object",
                properties: {
                  resource: {
                    type: "string",
                    nullable: true,
                  },
                },
                required: ["resource"],
              },
              {
                type: "object",
                properties: {
                  str: {
                    type: "array",
                    items: {
                      type: "string",
                    },
                    nullable: true,
                  },
                },
                required: ["str"],
              },
              {
                type: "object",
                properties: {
                  object: {
                    type: "array",
                    items: {
                      type: "object",
                      properties: {
                        key: {
                          type: "string",
                        },
                        typ: {
                          oneOf: [
                            {
                              type: "string",
                              enum: [
                                "float",
                                "int",
                                "bool",
                                "email",
                                "unknown",
                                "bytes",
                                "dict",
                                "datetime",
                                "sql",
                              ],
                            },
                            {
                              type: "object",
                              properties: {
                                str: {},
                              },
                              required: ["str"],
                            },
                          ],
                        },
                      },
                      required: ["key", "typ"],
                    },
                  },
                },
                required: ["object"],
              },
              {
                type: "object",
                properties: {
                  list: {
                    oneOf: [
                      {
                        type: "string",
                        enum: [
                          "float",
                          "int",
                          "bool",
                          "email",
                          "unknown",
                          "bytes",
                          "dict",
                          "datetime",
                          "sql",
                        ],
                      },
                      {
                        type: "object",
                        properties: {
                          str: {},
                        },
                        required: ["str"],
                      },
                    ],
                    nullable: true,
                  },
                },
                required: ["list"],
              },
            ],
          },
          has_default: {
            type: "boolean",
          },
          default: {},
        },
        required: ["name", "typ"],
      },
    },
  },
  required: ["star_args", "start_kwargs", "args", "type", "error"],
};
var $Preview = {
  type: "object",
  properties: {
    content: {
      type: "string",
    },
    path: {
      type: "string",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
    tag: {
      type: "string",
    },
    kind: {
      type: "string",
      enum: ["code", "identity", "http"],
    },
    dedicated_worker: {
      type: "boolean",
    },
    lock: {
      type: "string",
    },
  },
  required: ["args"],
};
var $WorkflowTask = {
  type: "object",
  properties: {
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
  },
  required: ["args"],
};
var $WorkflowStatusRecord = {
  type: "object",
  additionalProperties: {
    $ref: "#/components/schemas/WorkflowStatus",
  },
};
var $WorkflowStatus = {
  type: "object",
  properties: {
    scheduled_for: {
      type: "string",
      format: "date-time",
    },
    started_at: {
      type: "string",
      format: "date-time",
    },
    duration_ms: {
      type: "number",
    },
    name: {
      type: "string",
    },
  },
};
var $CreateResource = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    value: {},
    description: {
      type: "string",
    },
    resource_type: {
      type: "string",
    },
  },
  required: ["path", "value", "resource_type"],
};
var $EditResource = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    description: {
      type: "string",
    },
    value: {},
  },
};
var $Resource = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    description: {
      type: "string",
    },
    resource_type: {
      type: "string",
    },
    value: {},
    is_oauth: {
      type: "boolean",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
  },
  required: ["path", "resource_type", "is_oauth"],
};
var $ListableResource = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    description: {
      type: "string",
    },
    resource_type: {
      type: "string",
    },
    value: {},
    is_oauth: {
      type: "boolean",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    is_expired: {
      type: "boolean",
    },
    refresh_error: {
      type: "string",
    },
    is_linked: {
      type: "boolean",
    },
    is_refreshed: {
      type: "boolean",
    },
    account: {
      type: "number",
    },
  },
  required: ["path", "resource_type", "is_oauth", "is_linked", "is_refreshed"],
};
var $ResourceType = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    name: {
      type: "string",
    },
    schema: {},
    description: {
      type: "string",
    },
  },
  required: ["name"],
};
var $EditResourceType = {
  type: "object",
  properties: {
    schema: {},
    description: {
      type: "string",
    },
  },
};
var $Schedule = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    edited_by: {
      type: "string",
    },
    edited_at: {
      type: "string",
      format: "date-time",
    },
    schedule: {
      type: "string",
    },
    timezone: {
      type: "string",
    },
    enabled: {
      type: "boolean",
    },
    script_path: {
      type: "string",
    },
    is_flow: {
      type: "boolean",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    email: {
      type: "string",
    },
    error: {
      type: "string",
    },
    on_failure: {
      type: "string",
    },
    on_failure_times: {
      type: "number",
    },
    on_failure_exact: {
      type: "boolean",
    },
    on_failure_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    on_recovery: {
      type: "string",
    },
    on_recovery_times: {
      type: "number",
    },
    on_recovery_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    retry: {
      $ref: "#/components/schemas/Retry",
    },
    summary: {
      type: "string",
    },
    no_flow_overlap: {
      type: "boolean",
    },
    tag: {
      type: "string",
    },
  },
  required: [
    "path",
    "edited_by",
    "edited_at",
    "schedule",
    "script_path",
    "timezone",
    "extra_perms",
    "is_flow",
    "enabled",
    "email",
  ],
};
var $ScheduleWJobs = {
  allOf: [
    {
      $ref: "#/components/schemas/Schedule",
    },
    {
      type: "object",
      properties: {
        jobs: {
          type: "array",
          items: {
            type: "object",
            properties: {
              id: {
                type: "string",
              },
              success: {
                type: "boolean",
              },
              duration_ms: {
                type: "number",
              },
            },
            required: ["id", "success", "duration_ms"],
          },
        },
      },
    },
  ],
};
var $NewSchedule = {
  type: "object",
  properties: {
    path: {
      type: "string",
    },
    schedule: {
      type: "string",
    },
    timezone: {
      type: "string",
    },
    script_path: {
      type: "string",
    },
    is_flow: {
      type: "boolean",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    enabled: {
      type: "boolean",
    },
    on_failure: {
      type: "string",
    },
    on_failure_times: {
      type: "number",
    },
    on_failure_exact: {
      type: "boolean",
    },
    on_failure_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    on_recovery: {
      type: "string",
    },
    on_recovery_times: {
      type: "number",
    },
    on_recovery_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    retry: {
      $ref: "#/components/schemas/Retry",
    },
    no_flow_overlap: {
      type: "boolean",
    },
    summary: {
      type: "string",
    },
    tag: {
      type: "string",
    },
  },
  required: ["path", "schedule", "timezone", "script_path", "is_flow", "args"],
};
var $EditSchedule = {
  type: "object",
  properties: {
    schedule: {
      type: "string",
    },
    timezone: {
      type: "string",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    on_failure: {
      type: "string",
    },
    on_failure_times: {
      type: "number",
    },
    on_failure_exact: {
      type: "boolean",
    },
    on_failure_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    on_recovery: {
      type: "string",
    },
    on_recovery_times: {
      type: "number",
    },
    on_recovery_extra_args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    retry: {
      $ref: "#/components/schemas/Retry",
    },
    no_flow_overlap: {
      type: "boolean",
    },
    summary: {
      type: "string",
    },
    tag: {
      type: "string",
    },
  },
  required: ["schedule", "timezone", "script_path", "is_flow", "args"],
};
var $Group = {
  type: "object",
  properties: {
    name: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    members: {
      type: "array",
      items: {
        type: "string",
      },
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
  },
  required: ["name"],
};
var $InstanceGroup = {
  type: "object",
  properties: {
    name: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    emails: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: ["name"],
};
var $Folder = {
  type: "object",
  properties: {
    name: {
      type: "string",
    },
    owners: {
      type: "array",
      items: {
        type: "string",
      },
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
  },
  required: ["name", "owners", "extra_perms"],
};
var $WorkerPing = {
  type: "object",
  properties: {
    worker: {
      type: "string",
    },
    worker_instance: {
      type: "string",
    },
    last_ping: {
      type: "number",
    },
    started_at: {
      type: "string",
      format: "date-time",
    },
    ip: {
      type: "string",
    },
    jobs_executed: {
      type: "integer",
    },
    custom_tags: {
      type: "array",
      items: {
        type: "string",
      },
    },
    worker_group: {
      type: "string",
    },
    wm_version: {
      type: "string",
    },
    current_job_id: {
      type: "string",
    },
    current_job_workspace_id: {
      type: "string",
    },
    occupancy_rate: {
      type: "number",
    },
  },
  required: [
    "worker",
    "worker_instance",
    "ping_at",
    "started_at",
    "ip",
    "jobs_executed",
    "worker_group",
    "wm_version",
  ],
};
var $UserWorkspaceList = {
  type: "object",
  properties: {
    email: {
      type: "string",
    },
    workspaces: {
      type: "array",
      items: {
        type: "object",
        properties: {
          id: {
            type: "string",
          },
          name: {
            type: "string",
          },
          username: {
            type: "string",
          },
        },
        required: ["id", "name", "username"],
      },
    },
  },
  required: ["email", "workspaces"],
};
var $CreateWorkspace = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    name: {
      type: "string",
    },
    username: {
      type: "string",
    },
  },
  required: ["id", "name"],
};
var $Workspace = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    name: {
      type: "string",
    },
    owner: {
      type: "string",
    },
    domain: {
      type: "string",
    },
  },
  required: ["id", "name", "owner"],
};
var $WorkspaceInvite = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    email: {
      type: "string",
    },
    is_admin: {
      type: "boolean",
    },
    operator: {
      type: "boolean",
    },
  },
  required: ["workspace_id", "email", "is_admin", "operator"],
};
var $GlobalUserInfo = {
  type: "object",
  properties: {
    email: {
      type: "string",
    },
    login_type: {
      type: "string",
      enum: ["password", "github"],
    },
    super_admin: {
      type: "boolean",
    },
    verified: {
      type: "boolean",
    },
    name: {
      type: "string",
    },
    company: {
      type: "string",
    },
    username: {
      type: "string",
    },
  },
  required: ["email", "login_type", "super_admin", "verified"],
};
var $Flow = {
  allOf: [
    {
      $ref: "#/components/schemas/OpenFlow",
    },
    {
      $ref: "#/components/schemas/FlowMetadata",
    },
  ],
};
var $ExtraPerms = {
  type: "object",
  additionalProperties: {
    type: "boolean",
  },
};
var $FlowMetadata = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    edited_by: {
      type: "string",
    },
    edited_at: {
      type: "string",
      format: "date-time",
    },
    archived: {
      type: "boolean",
    },
    extra_perms: {
      $ref: "#/components/schemas/ExtraPerms",
    },
    starred: {
      type: "boolean",
    },
    draft_only: {
      type: "boolean",
    },
    tag: {
      type: "string",
    },
    ws_error_handler_muted: {
      type: "boolean",
    },
    priority: {
      type: "integer",
    },
    dedicated_worker: {
      type: "boolean",
    },
    timeout: {
      type: "number",
    },
    visible_to_runner_only: {
      type: "boolean",
    },
  },
  required: ["path", "edited_by", "edited_at", "archived", "extra_perms"],
};
var $OpenFlowWPath = {
  allOf: [
    {
      $ref: "#/components/schemas/OpenFlow",
    },
    {
      type: "object",
      properties: {
        path: {
          type: "string",
        },
        tag: {
          type: "string",
        },
        ws_error_handler_muted: {
          type: "boolean",
        },
        priority: {
          type: "integer",
        },
        dedicated_worker: {
          type: "boolean",
        },
        timeout: {
          type: "number",
        },
        visible_to_runner_only: {
          type: "boolean",
        },
      },
      required: ["path"],
    },
  ],
};
var $FlowPreview = {
  type: "object",
  properties: {
    value: {
      $ref: "#/components/schemas/FlowValue",
    },
    path: {
      type: "string",
    },
    args: {
      $ref: "#/components/schemas/ScriptArgs",
    },
    tag: {
      type: "string",
    },
    restarted_from: {
      $ref: "#/components/schemas/RestartedFrom",
    },
  },
  required: ["value", "content", "args"],
};
var $RestartedFrom = {
  type: "object",
  properties: {
    flow_job_id: {
      type: "string",
      format: "uuid",
    },
    step_id: {
      type: "string",
    },
    branch_or_iteration_n: {
      type: "integer",
    },
  },
};
var $Policy = {
  type: "object",
  properties: {
    triggerables: {
      type: "object",
      additionalProperties: {
        type: "object",
      },
    },
    triggerables_v2: {
      type: "object",
      additionalProperties: {
        type: "object",
      },
    },
    execution_mode: {
      type: "string",
      enum: ["viewer", "publisher", "anonymous"],
    },
    on_behalf_of: {
      type: "string",
    },
    on_behalf_of_email: {
      type: "string",
    },
  },
};
var $ListableApp = {
  type: "object",
  properties: {
    id: {
      type: "integer",
    },
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    version: {
      type: "integer",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    starred: {
      type: "boolean",
    },
    edited_at: {
      type: "string",
      format: "date-time",
    },
    execution_mode: {
      type: "string",
      enum: ["viewer", "publisher", "anonymous"],
    },
  },
  required: [
    "id",
    "workspace_id",
    "path",
    "summary",
    "version",
    "extra_perms",
    "edited_at",
    "execution_mode",
  ],
};
var $ListableRawApp = {
  type: "object",
  properties: {
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
    starred: {
      type: "boolean",
    },
    version: {
      type: "number",
    },
    edited_at: {
      type: "string",
      format: "date-time",
    },
  },
  required: [
    "workspace_id",
    "path",
    "summary",
    "extra_perms",
    "version",
    "edited_at",
  ],
};
var $AppWithLastVersion = {
  type: "object",
  properties: {
    id: {
      type: "integer",
    },
    workspace_id: {
      type: "string",
    },
    path: {
      type: "string",
    },
    summary: {
      type: "string",
    },
    versions: {
      type: "array",
      items: {
        type: "integer",
      },
    },
    created_by: {
      type: "string",
    },
    created_at: {
      type: "string",
      format: "date-time",
    },
    value: {
      type: "object",
    },
    policy: {
      $ref: "#/components/schemas/Policy",
    },
    execution_mode: {
      type: "string",
      enum: ["viewer", "publisher", "anonymous"],
    },
    extra_perms: {
      type: "object",
      additionalProperties: {
        type: "boolean",
      },
    },
  },
  required: [
    "id",
    "workspace_id",
    "path",
    "summary",
    "versions",
    "created_by",
    "created_at",
    "value",
    "policy",
    "execution_mode",
    "extra_perms",
  ],
};
var $AppWithLastVersionWDraft = {
  allOf: [
    {
      $ref: "#/components/schemas/AppWithLastVersion",
    },
    {
      type: "object",
      properties: {
        draft_only: {
          type: "boolean",
        },
        draft: {},
      },
    },
  ],
};
var $AppHistory = {
  type: "object",
  properties: {
    version: {
      type: "integer",
    },
    deployment_msg: {
      type: "string",
    },
  },
  required: ["version"],
};
var $SlackToken = {
  type: "object",
  properties: {
    access_token: {
      type: "string",
    },
    team_id: {
      type: "string",
    },
    team_name: {
      type: "string",
    },
    bot: {
      type: "object",
      properties: {
        bot_access_token: {
          type: "string",
        },
      },
    },
  },
  required: ["access_token", "team_id", "team_name", "bot"],
};
var $TokenResponse = {
  type: "object",
  properties: {
    access_token: {
      type: "string",
    },
    expires_in: {
      type: "integer",
    },
    refresh_token: {
      type: "string",
    },
    scope: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: ["access_token"],
};
var $HubScriptKind = {
  name: "kind",
  schema: {
    type: "string",
    enum: ["script", "failure", "trigger", "approval"],
  },
};
var $PolarsClientKwargs = {
  type: "object",
  properties: {
    region_name: {
      type: "string",
    },
  },
  required: ["region_name"],
};
var $LargeFileStorage = {
  type: "object",
  properties: {
    type: {
      type: "string",
      enum: [
        "S3Storage",
        "AzureBlobStorage",
        "AzureWorkloadIdentity",
        "S3AwsOidc",
      ],
    },
    s3_resource_path: {
      type: "string",
    },
    azure_blob_resource_path: {
      type: "string",
    },
    public_resource: {
      type: "boolean",
    },
  },
};
var $WindmillLargeFile = {
  type: "object",
  properties: {
    s3: {
      type: "string",
    },
  },
  required: ["s3"],
};
var $WindmillFileMetadata = {
  type: "object",
  properties: {
    mime_type: {
      type: "string",
    },
    size_in_bytes: {
      type: "integer",
    },
    last_modified: {
      type: "string",
      format: "date-time",
    },
    expires: {
      type: "string",
      format: "date-time",
    },
    version_id: {
      type: "string",
    },
  },
};
var $WindmillFilePreview = {
  type: "object",
  properties: {
    msg: {
      type: "string",
    },
    content: {
      type: "string",
    },
    content_type: {
      type: "string",
      enum: ["RawText", "Csv", "Parquet", "Unknown"],
    },
  },
  required: ["content_type"],
};
var $S3Resource = {
  type: "object",
  properties: {
    bucket: {
      type: "string",
    },
    region: {
      type: "string",
    },
    endPoint: {
      type: "string",
    },
    useSSL: {
      type: "boolean",
    },
    accessKey: {
      type: "string",
    },
    secretKey: {
      type: "string",
    },
    pathStyle: {
      type: "boolean",
    },
  },
  required: ["bucket", "region", "endPoint", "useSSL", "pathStyle"],
};
var $WorkspaceGitSyncSettings = {
  type: "object",
  properties: {
    include_path: {
      type: "array",
      items: {
        type: "string",
      },
    },
    include_type: {
      type: "array",
      items: {
        type: "string",
        enum: [
          "script",
          "flow",
          "app",
          "folder",
          "resource",
          "variable",
          "secret",
          "resourcetype",
          "schedule",
          "user",
          "group",
        ],
      },
    },
    repositories: {
      type: "array",
      items: {
        $ref: "#/components/schemas/GitRepositorySettings",
      },
    },
  },
};
var $WorkspaceDefaultScripts = {
  type: "object",
  properties: {
    order: {
      type: "array",
      items: {
        type: "string",
      },
    },
    hidden: {
      type: "array",
      items: {
        type: "string",
      },
    },
    default_script_content: {
      additionalProperties: {
        type: "string",
      },
    },
  },
};
var $GitRepositorySettings = {
  type: "object",
  properties: {
    script_path: {
      type: "string",
    },
    git_repo_resource_path: {
      type: "string",
    },
    use_individual_branch: {
      type: "boolean",
    },
    group_by_folder: {
      type: "boolean",
    },
    exclude_types_override: {
      type: "array",
      items: {
        type: "string",
        enum: [
          "script",
          "flow",
          "app",
          "folder",
          "resource",
          "variable",
          "secret",
          "resourcetype",
          "schedule",
          "user",
          "group",
        ],
      },
    },
  },
  required: ["script_path", "git_repo_resource_path"],
};
var $UploadFilePart = {
  type: "object",
  properties: {
    part_number: {
      type: "integer",
    },
    tag: {
      type: "string",
    },
  },
  required: ["part_number", "tag"],
};
var $MetricMetadata = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    name: {
      type: "string",
    },
  },
  required: ["id"],
};
var $ScalarMetric = {
  type: "object",
  properties: {
    metric_id: {
      type: "string",
    },
    value: {
      type: "number",
    },
  },
  required: ["id", "value"],
};
var $TimeseriesMetric = {
  type: "object",
  properties: {
    metric_id: {
      type: "string",
    },
    values: {
      type: "array",
      items: {
        $ref: "#/components/schemas/MetricDataPoint",
      },
    },
  },
  required: ["id", "values"],
};
var $MetricDataPoint = {
  type: "object",
  properties: {
    timestamp: {
      type: "string",
      format: "date-time",
    },
    value: {
      type: "number",
    },
  },
  required: ["timestamp", "value"],
};
var $RawScriptForDependencies = {
  type: "object",
  properties: {
    raw_code: {
      type: "string",
    },
    path: {
      type: "string",
    },
    language: {
      type: "string",
      enum: [
        "python3",
        "deno",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "bun",
      ],
    },
  },
  required: ["raw_code", "path", "language"],
};
var $ConcurrencyGroup = {
  type: "object",
  properties: {
    concurrency_id: {
      type: "string",
    },
    job_uuids: {
      type: "array",
      items: {
        type: "string",
      },
    },
  },
  required: ["concurrency_id", "job_uuids"],
};
var $OpenFlow = {
  type: "object",
  properties: {
    summary: {
      type: "string",
    },
    description: {
      type: "string",
    },
    value: {
      $ref: "#/components/schemas/FlowValue",
    },
    schema: {
      type: "object",
    },
  },
  required: ["summary", "value"],
};
var $FlowValue = {
  type: "object",
  properties: {
    modules: {
      type: "array",
      items: {
        $ref: "#/components/schemas/FlowModule",
      },
    },
    failure_module: {
      $ref: "#/components/schemas/FlowModule",
    },
    same_worker: {
      type: "boolean",
    },
    concurrent_limit: {
      type: "number",
    },
    concurrency_key: {
      type: "string",
    },
    concurrency_time_window_s: {
      type: "number",
    },
    skip_expr: {
      type: "string",
    },
    cache_ttl: {
      type: "number",
    },
    priority: {
      type: "number",
    },
    early_return: {
      type: "string",
    },
  },
  required: ["modules"],
};
var $Retry = {
  type: "object",
  properties: {
    constant: {
      type: "object",
      properties: {
        attempts: {
          type: "integer",
        },
        seconds: {
          type: "integer",
        },
      },
    },
    exponential: {
      type: "object",
      properties: {
        attempts: {
          type: "integer",
        },
        multiplier: {
          type: "integer",
        },
        seconds: {
          type: "integer",
        },
        random_factor: {
          type: "integer",
          minimum: 0,
          maximum: 100,
        },
      },
    },
  },
};
var $FlowModule = {
  type: "object",
  properties: {
    id: {
      type: "string",
    },
    value: {
      $ref: "#/components/schemas/FlowModuleValue",
    },
    stop_after_if: {
      type: "object",
      properties: {
        skip_if_stopped: {
          type: "boolean",
        },
        expr: {
          type: "string",
        },
      },
      required: ["expr"],
    },
    sleep: {
      $ref: "#/components/schemas/InputTransform",
    },
    cache_ttl: {
      type: "number",
    },
    timeout: {
      type: "number",
    },
    delete_after_use: {
      type: "boolean",
    },
    summary: {
      type: "string",
    },
    mock: {
      type: "object",
      properties: {
        enabled: {
          type: "boolean",
        },
        return_value: {},
      },
    },
    suspend: {
      type: "object",
      properties: {
        required_events: {
          type: "integer",
        },
        timeout: {
          type: "integer",
        },
        resume_form: {
          type: "object",
          properties: {
            schema: {
              type: "object",
            },
          },
        },
        user_auth_required: {
          type: "boolean",
        },
        user_groups_required: {
          $ref: "#/components/schemas/InputTransform",
        },
        self_approval_disabled: {
          type: "boolean",
        },
        hide_cancel: {
          type: "boolean",
        },
      },
    },
    priority: {
      type: "number",
    },
    continue_on_error: {
      type: "boolean",
    },
    retry: {
      $ref: "#/components/schemas/Retry",
    },
  },
  required: ["value", "id"],
};
var $InputTransform = {
  oneOf: [
    {
      $ref: "#/components/schemas/StaticTransform",
    },
    {
      $ref: "#/components/schemas/JavascriptTransform",
    },
  ],
  discriminator: {
    propertyName: "type",
    mapping: {
      static: "#/components/schemas/StaticTransform",
      javascript: "#/components/schemas/JavascriptTransform",
    },
  },
};
var $StaticTransform = {
  type: "object",
  properties: {
    value: {},
    type: {
      type: "string",
      enum: ["javascript"],
    },
  },
  required: ["expr", "type"],
};
var $JavascriptTransform = {
  type: "object",
  properties: {
    expr: {
      type: "string",
    },
    type: {
      type: "string",
      enum: ["javascript"],
    },
  },
  required: ["expr", "type"],
};
var $FlowModuleValue = {
  oneOf: [
    {
      $ref: "#/components/schemas/RawScript",
    },
    {
      $ref: "#/components/schemas/PathScript",
    },
    {
      $ref: "#/components/schemas/PathFlow",
    },
    {
      $ref: "#/components/schemas/ForloopFlow",
    },
    {
      $ref: "#/components/schemas/WhileloopFlow",
    },
    {
      $ref: "#/components/schemas/BranchOne",
    },
    {
      $ref: "#/components/schemas/BranchAll",
    },
    {
      $ref: "#/components/schemas/Identity",
    },
  ],
  discriminator: {
    propertyName: "type",
    mapping: {
      rawscript: "#/components/schemas/RawScript",
      script: "#/components/schemas/PathScript",
      flow: "#/components/schemas/PathFlow",
      forloopflow: "#/components/schemas/ForloopFlow",
      whileloopflow: "#/components/schemas/WhileloopFlow",
      branchone: "#/components/schemas/BranchOne",
      branchall: "#/components/schemas/BranchAll",
      identity: "#/components/schemas/Identity",
    },
  },
};
var $RawScript = {
  type: "object",
  properties: {
    input_transforms: {
      type: "object",
      additionalProperties: {
        $ref: "#/components/schemas/InputTransform",
      },
    },
    content: {
      type: "string",
    },
    language: {
      type: "string",
      enum: [
        "deno",
        "bun",
        "python3",
        "go",
        "bash",
        "powershell",
        "postgresql",
        "mysql",
        "bigquery",
        "snowflake",
        "mssql",
        "graphql",
        "nativets",
        "duckdb",
        "ruby",
        // for related places search: ADD_NEW_LANG
      ],
    },
    path: {
      type: "string",
    },
    lock: {
      type: "string",
    },
    type: {
      type: "string",
      enum: ["rawscript"],
    },
    tag: {
      type: "string",
    },
    concurrent_limit: {
      type: "number",
    },
    concurrency_time_window_s: {
      type: "number",
    },
  },
  required: ["type", "content", "language", "input_transforms"],
};
var $PathScript = {
  type: "object",
  properties: {
    input_transforms: {
      type: "object",
      additionalProperties: {
        $ref: "#/components/schemas/InputTransform",
      },
    },
    path: {
      type: "string",
    },
    hash: {
      type: "string",
    },
    type: {
      type: "string",
      enum: ["script"],
    },
  },
  required: ["type", "path", "input_transforms"],
};
var $PathFlow = {
  type: "object",
  properties: {
    input_transforms: {
      type: "object",
      additionalProperties: {
        $ref: "#/components/schemas/InputTransform",
      },
    },
    path: {
      type: "string",
    },
    type: {
      type: "string",
      enum: ["flow"],
    },
  },
  required: ["type", "path", "input_transforms"],
};
var $ForloopFlow = {
  type: "object",
  properties: {
    modules: {
      type: "array",
      items: {
        $ref: "#/components/schemas/FlowModule",
      },
    },
    iterator: {
      $ref: "#/components/schemas/InputTransform",
    },
    skip_failures: {
      type: "boolean",
    },
    type: {
      type: "string",
      enum: ["forloopflow"],
    },
    parallel: {
      type: "boolean",
    },
    parallelism: {
      type: "integer",
    },
  },
  required: ["modules", "iterator", "skip_failures", "type"],
};
var $WhileloopFlow = {
  type: "object",
  properties: {
    modules: {
      type: "array",
      items: {
        $ref: "#/components/schemas/FlowModule",
      },
    },
    skip_failures: {
      type: "boolean",
    },
    type: {
      type: "string",
      enum: ["forloopflow"],
    },
    parallel: {
      type: "boolean",
    },
    parallelism: {
      type: "integer",
    },
  },
  required: ["modules", "skip_failures", "type"],
};
var $BranchOne = {
  type: "object",
  properties: {
    branches: {
      type: "array",
      items: {
        type: "object",
        properties: {
          summary: {
            type: "string",
          },
          expr: {
            type: "string",
          },
          modules: {
            type: "array",
            items: {
              $ref: "#/components/schemas/FlowModule",
            },
          },
        },
        required: ["modules", "expr"],
      },
    },
    default: {
      type: "array",
      items: {
        $ref: "#/components/schemas/FlowModule",
      },
      required: ["modules"],
    },
    type: {
      type: "string",
      enum: ["branchone"],
    },
  },
  required: ["branches", "default", "type"],
};
var $BranchAll = {
  type: "object",
  properties: {
    branches: {
      type: "array",
      items: {
        type: "object",
        properties: {
          summary: {
            type: "string",
          },
          skip_failure: {
            type: "boolean",
          },
          modules: {
            type: "array",
            items: {
              $ref: "#/components/schemas/FlowModule",
            },
          },
        },
        required: ["modules", "expr"],
      },
    },
    type: {
      type: "string",
      enum: ["branchall"],
    },
    parallel: {
      type: "boolean",
    },
  },
  required: ["branches", "type"],
};
var $Identity = {
  type: "object",
  properties: {
    type: {
      type: "string",
      enum: ["identity"],
    },
    flow: {
      type: "boolean",
    },
  },
  required: ["type"],
};
var $FlowStatus = {
  type: "object",
  properties: {
    step: {
      type: "integer",
    },
    modules: {
      type: "array",
      items: {
        $ref: "#/components/schemas/FlowStatusModule",
      },
    },
    user_states: {
      additionalProperties: true,
    },
    failure_module: {
      allOf: [
        {
          $ref: "#/components/schemas/FlowStatusModule",
        },
        {
          type: "object",
          properties: {
            parent_module: {
              type: "string",
            },
          },
        },
      ],
    },
    retry: {
      type: "object",
      properties: {
        fail_count: {
          type: "integer",
        },
        failed_jobs: {
          type: "array",
          items: {
            type: "string",
            format: "uuid",
          },
        },
      },
    },
  },
  required: ["step", "modules", "failure_module"],
};
var $FlowStatusModule = {
  type: "object",
  properties: {
    type: {
      type: "string",
      enum: [
        "WaitingForPriorSteps",
        "WaitingForEvents",
        "WaitingForExecutor",
        "InProgress",
        "Success",
        "Failure",
      ],
    },
    id: {
      type: "string",
    },
    job: {
      type: "string",
      format: "uuid",
    },
    count: {
      type: "integer",
    },
    iterator: {
      type: "object",
      properties: {
        index: {
          type: "integer",
        },
        itered: {
          type: "array",
          items: {},
        },
        args: {},
      },
    },
    flow_jobs: {
      type: "array",
      items: {
        type: "string",
      },
    },
    branch_chosen: {
      type: "object",
      properties: {
        type: {
          type: "string",
          enum: ["branch", "default"],
        },
        branch: {
          type: "integer",
        },
      },
      required: ["type"],
    },
    branchall: {
      type: "object",
      properties: {
        branch: {
          type: "integer",
        },
        len: {
          type: "integer",
        },
      },
      required: ["branch", "len"],
    },
    approvers: {
      type: "array",
      items: {
        type: "object",
        properties: {
          resume_id: {
            type: "integer",
          },
          approver: {
            type: "string",
          },
        },
        required: ["resume_id", "approver"],
      },
    },
  },
  required: ["type"],
};

// src/core/request.ts
var isString = (value) => {
  return typeof value === "string";
};
var isStringWithValue = (value) => {
  return isString(value) && value !== "";
};
var isBlob = (value) => {
  return value instanceof Blob;
};
var isFormData = (value) => {
  return value instanceof FormData;
};
var base64 = (str) => {
  try {
    return btoa(str);
  } catch (err) {
    return Buffer.from(str).toString("base64");
  }
};
var getQueryString = (params) => {
  const qs = [];
  const append = (key, value) => {
    qs.push(`${encodeURIComponent(key)}=${encodeURIComponent(String(value))}`);
  };
  const encodePair = (key, value) => {
    if (value === void 0 || value === null) {
      return;
    }
    if (value instanceof Date) {
      append(key, value.toISOString());
    } else if (Array.isArray(value)) {
      value.forEach((v) => encodePair(key, v));
    } else if (typeof value === "object") {
      Object.entries(value).forEach(([k, v]) => encodePair(`${key}[${k}]`, v));
    } else {
      append(key, value);
    }
  };
  Object.entries(params).forEach(([key, value]) => encodePair(key, value));
  return qs.length ? `?${qs.join("&")}` : "";
};
var getUrl = (config, options) => {
  const encoder = config.ENCODE_PATH || encodeURI;
  const path = options.url
    .replace("{api-version}", config.VERSION)
    .replace(/{(.*?)}/g, (substring, group) => {
      if (options.path?.hasOwnProperty(group)) {
        return encoder(String(options.path[group]));
      }
      return substring;
    });
  const url = config.BASE + path;
  return options.query ? url + getQueryString(options.query) : url;
};
var getFormData = (options) => {
  if (options.formData) {
    const formData = new FormData();
    const process2 = (key, value) => {
      if (isString(value) || isBlob(value)) {
        formData.append(key, value);
      } else {
        formData.append(key, JSON.stringify(value));
      }
    };
    Object.entries(options.formData)
      .filter(([, value]) => value !== void 0 && value !== null)
      .forEach(([key, value]) => {
        if (Array.isArray(value)) {
          value.forEach((v) => process2(key, v));
        } else {
          process2(key, value);
        }
      });
    return formData;
  }
  return void 0;
};
var resolve = async (options, resolver) => {
  if (typeof resolver === "function") {
    return resolver(options);
  }
  return resolver;
};
var getHeaders = async (config, options) => {
  const [token, username, password, additionalHeaders] = await Promise.all([
    resolve(options, config.TOKEN),
    resolve(options, config.USERNAME),
    resolve(options, config.PASSWORD),
    resolve(options, config.HEADERS),
  ]);
  const headers = Object.entries({
    Accept: "application/json",
    ...additionalHeaders,
    ...options.headers,
  })
    .filter(([, value]) => value !== void 0 && value !== null)
    .reduce(
      (headers2, [key, value]) => ({
        ...headers2,
        [key]: String(value),
      }),
      {}
    );
  if (isStringWithValue(token)) {
    headers["Authorization"] = `Bearer ${token}`;
  }
  if (isStringWithValue(username) && isStringWithValue(password)) {
    const credentials = base64(`${username}:${password}`);
    headers["Authorization"] = `Basic ${credentials}`;
  }
  if (options.body !== void 0) {
    if (options.mediaType) {
      headers["Content-Type"] = options.mediaType;
    } else if (isBlob(options.body)) {
      headers["Content-Type"] = options.body.type || "application/octet-stream";
    } else if (isString(options.body)) {
      headers["Content-Type"] = "text/plain";
    } else if (!isFormData(options.body)) {
      headers["Content-Type"] = "application/json";
    }
  }
  return new Headers(headers);
};
var getRequestBody = (options) => {
  if (options.body !== void 0) {
    if (
      options.mediaType?.includes("application/json") ||
      options.mediaType?.includes("+json")
    ) {
      return JSON.stringify(options.body);
    } else if (
      isString(options.body) ||
      isBlob(options.body) ||
      isFormData(options.body)
    ) {
      return options.body;
    } else {
      return JSON.stringify(options.body);
    }
  }
  return void 0;
};
var sendRequest = async (
  config,
  options,
  url,
  body,
  formData,
  headers,
  onCancel
) => {
  const controller = new AbortController();
  let request2 = {
    headers,
    body: body ?? formData,
    method: options.method,
    signal: controller.signal,
  };
  if (config.WITH_CREDENTIALS) {
    request2.credentials = config.CREDENTIALS;
  }
  for (const fn of config.interceptors.request._fns) {
    request2 = await fn(request2);
  }
  onCancel(() => controller.abort());
  return await fetch(url, request2);
};
var getResponseHeader = (response, responseHeader) => {
  if (responseHeader) {
    const content = response.headers.get(responseHeader);
    if (isString(content)) {
      return content;
    }
  }
  return void 0;
};
var getResponseBody = async (response) => {
  if (response.status !== 204) {
    try {
      const contentType = response.headers.get("Content-Type");
      if (contentType) {
        const binaryTypes = [
          "application/octet-stream",
          "application/pdf",
          "application/zip",
          "audio/",
          "image/",
          "video/",
        ];
        if (
          contentType.includes("application/json") ||
          contentType.includes("+json")
        ) {
          return await response.json();
        } else if (binaryTypes.some((type) => contentType.includes(type))) {
          return await response.blob();
        } else if (contentType.includes("multipart/form-data")) {
          return await response.formData();
        } else if (contentType.includes("text/")) {
          return await response.text();
        }
      }
    } catch (error) {
      console.error(error);
    }
  }
  return void 0;
};
var catchErrorCodes = (options, result) => {
  const errors = {
    400: "Bad Request",
    401: "Unauthorized",
    402: "Payment Required",
    403: "Forbidden",
    404: "Not Found",
    405: "Method Not Allowed",
    406: "Not Acceptable",
    407: "Proxy Authentication Required",
    408: "Request Timeout",
    409: "Conflict",
    410: "Gone",
    411: "Length Required",
    412: "Precondition Failed",
    413: "Payload Too Large",
    414: "URI Too Long",
    415: "Unsupported Media Type",
    416: "Range Not Satisfiable",
    417: "Expectation Failed",
    418: "Im a teapot",
    421: "Misdirected Request",
    422: "Unprocessable Content",
    423: "Locked",
    424: "Failed Dependency",
    425: "Too Early",
    426: "Upgrade Required",
    428: "Precondition Required",
    429: "Too Many Requests",
    431: "Request Header Fields Too Large",
    451: "Unavailable For Legal Reasons",
    500: "Internal Server Error",
    501: "Not Implemented",
    502: "Bad Gateway",
    503: "Service Unavailable",
    504: "Gateway Timeout",
    505: "HTTP Version Not Supported",
    506: "Variant Also Negotiates",
    507: "Insufficient Storage",
    508: "Loop Detected",
    510: "Not Extended",
    511: "Network Authentication Required",
    ...options.errors,
  };
  const error = errors[result.status];
  if (error) {
    throw new ApiError(options, result, error);
  }
  if (!result.ok) {
    const errorStatus = result.status ?? "unknown";
    const errorStatusText = result.statusText ?? "unknown";
    const errorBody = (() => {
      try {
        return JSON.stringify(result.body, null, 2);
      } catch (e) {
        return void 0;
      }
    })();
    throw new ApiError(
      options,
      result,
      `Generic Error: status: ${errorStatus}; status text: ${errorStatusText}; body: ${errorBody}`
    );
  }
};
var request = (config, options) => {
  return new CancelablePromise(async (resolve2, reject, onCancel) => {
    try {
      const url = getUrl(config, options);
      const formData = getFormData(options);
      const body = getRequestBody(options);
      const headers = await getHeaders(config, options);
      if (!onCancel.isCancelled) {
        let response = await sendRequest(
          config,
          options,
          url,
          body,
          formData,
          headers,
          onCancel
        );
        for (const fn of config.interceptors.response._fns) {
          response = await fn(response);
        }
        const responseBody = await getResponseBody(response);
        const responseHeader = getResponseHeader(
          response,
          options.responseHeader
        );
        const result = {
          url,
          ok: response.ok,
          status: response.status,
          statusText: response.statusText,
          body: responseHeader ?? responseBody,
        };
        catchErrorCodes(options, result);
        resolve2(result.body);
      }
    } catch (error) {
      reject(error);
    }
  });
};

// src/services.gen.ts
var SettingsService = class {
  /**
   * get backend version
   * @returns string git version of backend
   * @throws ApiError
   */
  static backendVersion() {
    return request(OpenAPI, {
      method: "GET",
      url: "/version",
    });
  }
  /**
   * is backend up to date
   * @returns string is backend up to date
   * @throws ApiError
   */
  static backendUptodate() {
    return request(OpenAPI, {
      method: "GET",
      url: "/uptodate",
    });
  }
  /**
   * get license id
   * @returns string get license id (empty if not ee)
   * @throws ApiError
   */
  static getLicenseId() {
    return request(OpenAPI, {
      method: "GET",
      url: "/ee_license",
    });
  }
  /**
   * get openapi yaml spec
   * @returns string openapi yaml file content
   * @throws ApiError
   */
  static getOpenApiYaml() {
    return request(OpenAPI, {
      method: "GET",
      url: "/openapi.yaml",
    });
  }
};
var AuditService = class {
  /**
   * get audit log (requires admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns AuditLog an audit log
   * @throws ApiError
   */
  static getAuditLog(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/audit/get/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * list audit logs (requires admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.before filter on created before (exclusive) timestamp
   * @param data.after filter on created after (exclusive) timestamp
   * @param data.username filter on exact username of user
   * @param data.operation filter on exact or prefix name of operation
   * @param data.resource filter on exact or prefix name of resource
   * @param data.actionKind filter on type of operation
   * @returns AuditLog a list of audit logs
   * @throws ApiError
   */
  static listAuditLogs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/audit/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        before: data.before,
        after: data.after,
        username: data.username,
        operation: data.operation,
        resource: data.resource,
        action_kind: data.actionKind,
      },
    });
  }
};
var UserService = class {
  /**
   * login with password
   * @param data The data for the request.
   * @param data.requestBody credentials
   * @returns string Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
   *
   * @throws ApiError
   */
  static login(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/auth/login",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * logout
   * @returns string clear cookies and clear token (if applicable)
   * @throws ApiError
   */
  static logout() {
    return request(OpenAPI, {
      method: "POST",
      url: "/auth/logout",
    });
  }
  /**
   * get user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @returns User user created
   * @throws ApiError
   */
  static getUser(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
    });
  }
  /**
   * update user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @param data.requestBody new user
   * @returns string edited user
   * @throws ApiError
   */
  static updateUser(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/users/update/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * is owner of path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean is owner
   * @throws ApiError
   */
  static isOwnerOfPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/is_owner/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * set password
   * @param data The data for the request.
   * @param data.requestBody set password
   * @returns string password set
   * @throws ApiError
   */
  static setPassword(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/setpassword",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * create user
   * @param data The data for the request.
   * @param data.requestBody user info
   * @returns string user created
   * @throws ApiError
   */
  static createUserGlobally(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/create",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * global update user (require super admin)
   * @param data The data for the request.
   * @param data.email
   * @param data.requestBody new user info
   * @returns string user updated
   * @throws ApiError
   */
  static globalUserUpdate(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/update/{email}",
      path: {
        email: data.email,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * global username info (require super admin)
   * @param data The data for the request.
   * @param data.email
   * @returns unknown user renamed
   * @throws ApiError
   */
  static globalUsernameInfo(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/username_info/{email}",
      path: {
        email: data.email,
      },
    });
  }
  /**
   * global rename user (require super admin)
   * @param data The data for the request.
   * @param data.email
   * @param data.requestBody new username
   * @returns string user renamed
   * @throws ApiError
   */
  static globalUserRename(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/rename/{email}",
      path: {
        email: data.email,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * global delete user (require super admin)
   * @param data The data for the request.
   * @param data.email
   * @returns string user deleted
   * @throws ApiError
   */
  static globalUserDelete(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/users/delete/{email}",
      path: {
        email: data.email,
      },
    });
  }
  /**
   * delete user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @returns string delete user
   * @throws ApiError
   */
  static deleteUser(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/users/delete/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
    });
  }
  /**
   * get current user email (if logged in)
   * @returns string user email
   * @throws ApiError
   */
  static getCurrentEmail() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/email",
    });
  }
  /**
   * refresh the current token
   * @returns string free usage
   * @throws ApiError
   */
  static refreshUserToken() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/refresh_token",
    });
  }
  /**
   * get tutorial progress
   * @returns unknown tutorial progress
   * @throws ApiError
   */
  static getTutorialProgress() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/tutorial_progress",
    });
  }
  /**
   * update tutorial progress
   * @param data The data for the request.
   * @param data.requestBody progress update
   * @returns string tutorial progress
   * @throws ApiError
   */
  static updateTutorialProgress(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/tutorial_progress",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * leave instance
   * @returns string status
   * @throws ApiError
   */
  static leaveInstance() {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/leave_instance",
    });
  }
  /**
   * get current usage outside of premium workspaces
   * @returns number free usage
   * @throws ApiError
   */
  static getUsage() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/usage",
    });
  }
  /**
   * get all runnables in every workspace
   * @returns unknown free all runnables
   * @throws ApiError
   */
  static getRunnable() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/all_runnables",
    });
  }
  /**
   * get current global whoami (if logged in)
   * @returns GlobalUserInfo user email
   * @throws ApiError
   */
  static globalWhoami() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/whoami",
    });
  }
  /**
   * list all workspace invites
   * @returns WorkspaceInvite list all workspace invites
   * @throws ApiError
   */
  static listWorkspaceInvites() {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/list_invites",
    });
  }
  /**
   * whoami
   * @param data The data for the request.
   * @param data.workspace
   * @returns User user
   * @throws ApiError
   */
  static whoami(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/whoami",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * accept invite to workspace
   * @param data The data for the request.
   * @param data.requestBody accept invite
   * @returns string status
   * @throws ApiError
   */
  static acceptInvite(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/accept_invite",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * decline invite to workspace
   * @param data The data for the request.
   * @param data.requestBody decline invite
   * @returns string status
   * @throws ApiError
   */
  static declineInvite(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/decline_invite",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * whois
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @returns User user
   * @throws ApiError
   */
  static whois(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/whois/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
    });
  }
  /**
   * exists email
   * @param data The data for the request.
   * @param data.email
   * @returns boolean user
   * @throws ApiError
   */
  static existsEmail(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/exists/{email}",
      path: {
        email: data.email,
      },
    });
  }
  /**
   * list all users as super admin (require to be super amdin)
   * @param data The data for the request.
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns GlobalUserInfo user
   * @throws ApiError
   */
  static listUsersAsSuperAdmin(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/list_as_super_admin",
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * list users
   * @param data The data for the request.
   * @param data.workspace
   * @returns User user
   * @throws ApiError
   */
  static listUsers(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/list",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list users usage
   * @param data The data for the request.
   * @param data.workspace
   * @returns UserUsage user
   * @throws ApiError
   */
  static listUsersUsage(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/list_usage",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list usernames
   * @param data The data for the request.
   * @param data.workspace
   * @returns string user
   * @throws ApiError
   */
  static listUsernames(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/list_usernames",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * create token
   * @param data The data for the request.
   * @param data.requestBody new token
   * @returns string token created
   * @throws ApiError
   */
  static createToken(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/tokens/create",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * create token to impersonate a user (require superadmin)
   * @param data The data for the request.
   * @param data.requestBody new token
   * @returns string token created
   * @throws ApiError
   */
  static createTokenImpersonate(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/users/tokens/impersonate",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete token
   * @param data The data for the request.
   * @param data.tokenPrefix
   * @returns string delete token
   * @throws ApiError
   */
  static deleteToken(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/users/tokens/delete/{token_prefix}",
      path: {
        token_prefix: data.tokenPrefix,
      },
    });
  }
  /**
   * list token
   * @param data The data for the request.
   * @param data.excludeEphemeral
   * @returns TruncatedToken truncated token
   * @throws ApiError
   */
  static listTokens(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/users/tokens/list",
      query: {
        exclude_ephemeral: data.excludeEphemeral,
      },
    });
  }
  /**
   * login with oauth authorization flow
   * @param data The data for the request.
   * @param data.clientName
   * @param data.requestBody Partially filled script
   * @returns string Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
   *
   * @throws ApiError
   */
  static loginWithOauth(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/oauth/login_callback/{client_name}",
      path: {
        client_name: data.clientName,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var AdminService = class {
  /**
   * get user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @returns User user created
   * @throws ApiError
   */
  static getUser(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/users/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
    });
  }
  /**
   * update user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @param data.requestBody new user
   * @returns string edited user
   * @throws ApiError
   */
  static updateUser(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/users/update/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete user (require admin privilege)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.username
   * @returns string delete user
   * @throws ApiError
   */
  static deleteUser(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/users/delete/{username}",
      path: {
        workspace: data.workspace,
        username: data.username,
      },
    });
  }
};
var WorkspaceService = class {
  /**
   * list all workspaces visible to me
   * @returns Workspace all workspaces
   * @throws ApiError
   */
  static listWorkspaces() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workspaces/list",
    });
  }
  /**
   * is domain allowed for auto invi
   * @returns boolean domain allowed or not
   * @throws ApiError
   */
  static isDomainAllowed() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workspaces/allowed_domain_auto_invite",
    });
  }
  /**
   * list all workspaces visible to me with user info
   * @returns UserWorkspaceList workspace with associated username
   * @throws ApiError
   */
  static listUserWorkspaces() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workspaces/users",
    });
  }
  /**
   * list all workspaces as super admin (require to be super admin)
   * @param data The data for the request.
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Workspace workspaces
   * @throws ApiError
   */
  static listWorkspacesAsSuperAdmin(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/workspaces/list_as_superadmin",
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * create workspace
   * @param data The data for the request.
   * @param data.requestBody new token
   * @returns string token created
   * @throws ApiError
   */
  static createWorkspace(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/workspaces/create",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * exists workspace
   * @param data The data for the request.
   * @param data.requestBody id of workspace
   * @returns boolean status
   * @throws ApiError
   */
  static existsWorkspace(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/workspaces/exists",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * exists username
   * @param data The data for the request.
   * @param data.requestBody
   * @returns boolean status
   * @throws ApiError
   */
  static existsUsername(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/workspaces/exists_username",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * invite user to workspace
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceInvite
   * @returns string status
   * @throws ApiError
   */
  static inviteUser(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/invite_user",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * add user to workspace
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceInvite
   * @returns string status
   * @throws ApiError
   */
  static addUser(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/add_user",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete user invite
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceInvite
   * @returns string status
   * @throws ApiError
   */
  static deleteInvite(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/delete_invite",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * archive workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns string status
   * @throws ApiError
   */
  static archiveWorkspace(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/archive",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * unarchive workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns string status
   * @throws ApiError
   */
  static unarchiveWorkspace(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/workspaces/unarchive/{workspace}",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * delete workspace (require super admin)
   * @param data The data for the request.
   * @param data.workspace
   * @returns string status
   * @throws ApiError
   */
  static deleteWorkspace(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/workspaces/delete/{workspace}",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * leave workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns string status
   * @throws ApiError
   */
  static leaveWorkspace(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/leave",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get workspace name
   * @param data The data for the request.
   * @param data.workspace
   * @returns string status
   * @throws ApiError
   */
  static getWorkspaceName(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/get_workspace_name",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * change workspace name
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns string status
   * @throws ApiError
   */
  static changeWorkspaceName(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/change_workspace_name",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * change workspace id
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns string status
   * @throws ApiError
   */
  static changeWorkspaceId(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/change_workspace_id",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * list pending invites for a workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns WorkspaceInvite user
   * @throws ApiError
   */
  static listPendingInvites(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/list_pending_invites",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get settings
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getSettings(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/get_settings",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get deploy to
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getDeployTo(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/get_deploy_to",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get if workspace is premium
   * @param data The data for the request.
   * @param data.workspace
   * @returns boolean status
   * @throws ApiError
   */
  static getIsPremium(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/is_premium",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get premium info
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getPremiumInfo(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/premium_info",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * set automatic billing
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody automatic billing
   * @returns string status
   * @throws ApiError
   */
  static setAutomaticBilling(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/set_automatic_billing",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit slack command
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceInvite
   * @returns string status
   * @throws ApiError
   */
  static editSlackCommand(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_slack_command",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run a job that sends a message to Slack
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody path to hub script to run and its corresponding args
   * @returns unknown status
   * @throws ApiError
   */
  static runSlackMessageTestJob(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/run_slack_message_test_job",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit deploy to
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns string status
   * @throws ApiError
   */
  static editDeployTo(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_deploy_to",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit auto invite
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceInvite
   * @returns string status
   * @throws ApiError
   */
  static editAutoInvite(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_auto_invite",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit webhook
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceWebhook
   * @returns string status
   * @throws ApiError
   */
  static editWebhook(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_webhook",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit copilot config
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceCopilotConfig
   * @returns string status
   * @throws ApiError
   */
  static editCopilotConfig(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_copilot_config",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get copilot info
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getCopilotInfo(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/get_copilot_info",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * edit error handler
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody WorkspaceErrorHandler
   * @returns string status
   * @throws ApiError
   */
  static editErrorHandler(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_error_handler",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit large file storage settings
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody LargeFileStorage info
   * @returns unknown status
   * @throws ApiError
   */
  static editLargeFileStorageConfig(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_large_file_storage_config",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit workspace git sync settings
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Workspace Git sync settings
   * @returns unknown status
   * @throws ApiError
   */
  static editWorkspaceGitSyncConfig(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_git_sync_config",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit default app for workspace
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Workspace default app
   * @returns string status
   * @throws ApiError
   */
  static editWorkspaceDefaultApp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/edit_default_app",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * edit default scripts for workspace
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Workspace default app
   * @returns string status
   * @throws ApiError
   */
  static editDefaultScripts(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/default_scripts",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get default scripts for workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns WorkspaceDefaultScripts status
   * @throws ApiError
   */
  static getDefaultScripts(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/default_scripts",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * set environment variable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Workspace default app
   * @returns string status
   * @throws ApiError
   */
  static setEnvironmentVariable(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/set_environment_variable",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * retrieves the encryption key for this workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getWorkspaceEncryptionKey(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/encryption_key",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * update the encryption key for this workspace
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody New encryption key
   * @returns string status
   * @throws ApiError
   */
  static setWorkspaceEncryptionKey(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/workspaces/encryption_key",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get default app for workspace
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown status
   * @throws ApiError
   */
  static getWorkspaceDefaultApp(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/default_app",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get large file storage config
   * @param data The data for the request.
   * @param data.workspace
   * @returns LargeFileStorage status
   * @throws ApiError
   */
  static getLargeFileStorageConfig(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/get_large_file_storage_config",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * get usage
   * @param data The data for the request.
   * @param data.workspace
   * @returns number usage
   * @throws ApiError
   */
  static getWorkspaceUsage(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/workspaces/usage",
      path: {
        workspace: data.workspace,
      },
    });
  }
};
var SettingService = class {
  /**
   * get global settings
   * @param data The data for the request.
   * @param data.key
   * @returns unknown status
   * @throws ApiError
   */
  static getGlobal(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/settings/global/{key}",
      path: {
        key: data.key,
      },
    });
  }
  /**
   * post global settings
   * @param data The data for the request.
   * @param data.key
   * @param data.requestBody value set
   * @returns string status
   * @throws ApiError
   */
  static setGlobal(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/settings/global/{key}",
      path: {
        key: data.key,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get local settings
   * @returns unknown status
   * @throws ApiError
   */
  static getLocal() {
    return request(OpenAPI, {
      method: "GET",
      url: "/settings/local",
    });
  }
  /**
   * test smtp
   * @param data The data for the request.
   * @param data.requestBody test smtp payload
   * @returns string status
   * @throws ApiError
   */
  static testSmtp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/settings/test_smtp",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * test license key
   * @param data The data for the request.
   * @param data.requestBody test license key
   * @returns string status
   * @throws ApiError
   */
  static testLicenseKey(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/settings/test_license_key",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * test object storage config
   * @param data The data for the request.
   * @param data.requestBody test object storage config
   * @returns string status
   * @throws ApiError
   */
  static testObjectStorageConfig(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/settings/test_object_storage_config",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * send stats
   * @returns string status
   * @throws ApiError
   */
  static sendStats() {
    return request(OpenAPI, {
      method: "POST",
      url: "/settings/send_stats",
    });
  }
  /**
   * test metadata
   * @param data The data for the request.
   * @param data.requestBody test metadata
   * @returns string status
   * @throws ApiError
   */
  static testMetadata(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/saml/test_metadata",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var OidcService = class {
  /**
   * get OIDC token (ee only)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.audience
   * @returns string new oidc token
   * @throws ApiError
   */
  static getOidcToken(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oidc/token/{audience}",
      path: {
        workspace: data.workspace,
        audience: data.audience,
      },
    });
  }
};
var VariableService = class {
  /**
   * create variable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new variable
   * @param data.alreadyEncrypted
   * @returns string variable created
   * @throws ApiError
   */
  static createVariable(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/variables/create",
      path: {
        workspace: data.workspace,
      },
      query: {
        already_encrypted: data.alreadyEncrypted,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * encrypt value
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new variable
   * @returns string encrypted value
   * @throws ApiError
   */
  static encryptValue(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/variables/encrypt",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete variable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string variable deleted
   * @throws ApiError
   */
  static deleteVariable(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/variables/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update variable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated variable
   * @param data.alreadyEncrypted
   * @returns string variable updated
   * @throws ApiError
   */
  static updateVariable(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/variables/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        already_encrypted: data.alreadyEncrypted,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get variable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.decryptSecret ask to decrypt secret if this variable is secret
   * (if not secret no effect, default: true)
   *
   * @param data.includeEncrypted ask to include the encrypted value if secret and decrypt secret is not true (default: false)
   *
   * @returns ListableVariable variable
   * @throws ApiError
   */
  static getVariable(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/variables/get/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        decrypt_secret: data.decryptSecret,
        include_encrypted: data.includeEncrypted,
      },
    });
  }
  /**
   * get variable value
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string variable
   * @throws ApiError
   */
  static getVariableValue(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/variables/get_value/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * does variable exists at path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean variable
   * @throws ApiError
   */
  static existsVariable(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/variables/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * list variables
   * @param data The data for the request.
   * @param data.workspace
   * @returns ListableVariable variable list
   * @throws ApiError
   */
  static listVariable(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/variables/list",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list contextual variables
   * @param data The data for the request.
   * @param data.workspace
   * @returns ContextualVariable contextual variable list
   * @throws ApiError
   */
  static listContextualVariables(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/variables/list_contextual",
      path: {
        workspace: data.workspace,
      },
    });
  }
};
var OauthService = class {
  /**
   * connect slack callback
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody code endpoint
   * @returns string slack token
   * @throws ApiError
   */
  static connectSlackCallback(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oauth/connect_slack_callback",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * connect callback
   * @param data The data for the request.
   * @param data.clientName
   * @param data.requestBody code endpoint
   * @returns TokenResponse oauth token
   * @throws ApiError
   */
  static connectCallback(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/oauth/connect_callback/{client_name}",
      path: {
        client_name: data.clientName,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * create OAuth account
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody code endpoint
   * @returns string account set
   * @throws ApiError
   */
  static createAccount(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oauth/create_account",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * refresh token
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.requestBody variable path
   * @returns string token refreshed
   * @throws ApiError
   */
  static refreshToken(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oauth/refresh_token/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * disconnect account
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns string disconnected client
   * @throws ApiError
   */
  static disconnectAccount(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oauth/disconnect/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * disconnect slack
   * @param data The data for the request.
   * @param data.workspace
   * @returns string disconnected slack
   * @throws ApiError
   */
  static disconnectSlack(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/oauth/disconnect_slack",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list oauth logins
   * @returns unknown list of oauth and saml login clients
   * @throws ApiError
   */
  static listOauthLogins() {
    return request(OpenAPI, {
      method: "GET",
      url: "/oauth/list_logins",
    });
  }
  /**
   * list oauth connects
   * @returns unknown list of oauth connects clients
   * @throws ApiError
   */
  static listOauthConnects() {
    return request(OpenAPI, {
      method: "GET",
      url: "/oauth/list_connects",
    });
  }
};
var ResourceService = class {
  /**
   * create resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new resource
   * @param data.updateIfExists
   * @returns string resource created
   * @throws ApiError
   */
  static createResource(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/resources/create",
      path: {
        workspace: data.workspace,
      },
      query: {
        update_if_exists: data.updateIfExists,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string resource deleted
   * @throws ApiError
   */
  static deleteResource(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/resources/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated resource
   * @returns string resource updated
   * @throws ApiError
   */
  static updateResource(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/resources/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update resource value
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated resource
   * @returns string resource value updated
   * @throws ApiError
   */
  static updateResourceValue(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/resources/update_value/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns Resource resource
   * @throws ApiError
   */
  static getResource(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/get/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get resource interpolated (variables and resources are fully unrolled)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.jobId job id
   * @returns unknown resource value
   * @throws ApiError
   */
  static getResourceValueInterpolated(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/get_value_interpolated/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        job_id: data.jobId,
      },
    });
  }
  /**
   * get resource value
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown resource value
   * @throws ApiError
   */
  static getResourceValue(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/get_value/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * does resource exists
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean does resource exists
   * @throws ApiError
   */
  static existsResource(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * list resources
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.resourceType resource_types to list from, separated by ',',
   * @param data.resourceTypeExclude resource_types to not list from, separated by ',',
   * @returns ListableResource resource list
   * @throws ApiError
   */
  static listResource(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        resource_type: data.resourceType,
        resource_type_exclude: data.resourceTypeExclude,
      },
    });
  }
  /**
   * list resources for search
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown resource list
   * @throws ApiError
   */
  static listSearchResource(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/list_search",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list resource names
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns unknown resource list names
   * @throws ApiError
   */
  static listResourceNames(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/list_names/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * create resource_type
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new resource_type
   * @returns string resource_type created
   * @throws ApiError
   */
  static createResourceType(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/resources/type/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete resource_type
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string resource_type deleted
   * @throws ApiError
   */
  static deleteResourceType(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/resources/type/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update resource_type
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated resource_type
   * @returns string resource_type updated
   * @throws ApiError
   */
  static updateResourceType(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/resources/type/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get resource_type
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns ResourceType resource_type deleted
   * @throws ApiError
   */
  static getResourceType(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/type/get/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * does resource_type exists
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean does resource_type exist
   * @throws ApiError
   */
  static existsResourceType(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/type/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * list resource_types
   * @param data The data for the request.
   * @param data.workspace
   * @returns ResourceType resource_type list
   * @throws ApiError
   */
  static listResourceType(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/type/list",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list resource_types names
   * @param data The data for the request.
   * @param data.workspace
   * @returns string resource_type list
   * @throws ApiError
   */
  static listResourceTypeNames(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/resources/type/listnames",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * query resource types by similarity
   * @param data The data for the request.
   * @param data.workspace
   * @param data.text query text
   * @param data.limit query limit
   * @returns unknown resource type details
   * @throws ApiError
   */
  static queryResourceTypes(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/embeddings/query_resource_types",
      path: {
        workspace: data.workspace,
      },
      query: {
        text: data.text,
        limit: data.limit,
      },
    });
  }
};
var IntegrationService = class {
  /**
   * list hub integrations
   * @param data The data for the request.
   * @param data.kind query integrations kind
   * @returns unknown integrations details
   * @throws ApiError
   */
  static listHubIntegrations(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/integrations/hub/list",
      query: {
        kind: data.kind,
      },
    });
  }
};
var FlowService = class {
  /**
   * list all hub flows
   * @returns unknown hub flows list
   * @throws ApiError
   */
  static listHubFlows() {
    return request(OpenAPI, {
      method: "GET",
      url: "/flows/hub/list",
    });
  }
  /**
   * get hub flow by id
   * @param data The data for the request.
   * @param data.id
   * @returns unknown flow
   * @throws ApiError
   */
  static getHubFlowById(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/flows/hub/get/{id}",
      path: {
        id: data.id,
      },
    });
  }
  /**
   * list all flow paths
   * @param data The data for the request.
   * @param data.workspace
   * @returns string list of flow paths
   * @throws ApiError
   */
  static listFlowPaths(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/list_paths",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list flows for search
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown flow list
   * @throws ApiError
   */
  static listSearchFlow(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/list_search",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list all flows
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.pathStart mask to filter matching starting path
   * @param data.pathExact mask to filter exact matching path
   * @param data.showArchived (default false)
   * show also the archived files.
   * when multiple archived hash share the same path, only the ones with the latest create_at
   * are displayed.
   *
   * @param data.starredOnly (default false)
   * show only the starred items
   *
   * @returns unknown All flow
   * @throws ApiError
   */
  static listFlows(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        path_start: data.pathStart,
        path_exact: data.pathExact,
        show_archived: data.showArchived,
        starred_only: data.starredOnly,
      },
    });
  }
  /**
   * get flow by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns Flow flow details
   * @throws ApiError
   */
  static getFlowByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/get/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * Toggle ON and OFF the workspace error handler for a given flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody Workspace error handler enabled
   * @returns string error handler toggled
   * @throws ApiError
   */
  static toggleWorkspaceErrorHandlerForFlow(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/flows/toggle_workspace_error_handler/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get flow by path with draft
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown flow details with draft
   * @throws ApiError
   */
  static getFlowByPathWithDraft(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/get/draft/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * exists flow by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean flow details
   * @throws ApiError
   */
  static existsFlowByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * create flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Partially filled flow
   * @returns string flow created
   * @throws ApiError
   */
  static createFlow(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/flows/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody Partially filled flow
   * @returns string flow updated
   * @throws ApiError
   */
  static updateFlow(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/flows/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * archive flow by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody archiveFlow
   * @returns string flow archived
   * @throws ApiError
   */
  static archiveFlowByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/flows/archive/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete flow by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string flow delete
   * @throws ApiError
   */
  static deleteFlowByPath(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/flows/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * list inputs for previous completed flow jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Input input history for completed jobs with this flow path
   * @throws ApiError
   */
  static getFlowInputHistoryByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/flows/input_history/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
};
var AppService = class {
  /**
   * list all hub apps
   * @returns unknown hub apps list
   * @throws ApiError
   */
  static listHubApps() {
    return request(OpenAPI, {
      method: "GET",
      url: "/apps/hub/list",
    });
  }
  /**
   * get hub app by id
   * @param data The data for the request.
   * @param data.id
   * @returns unknown app
   * @throws ApiError
   */
  static getHubAppById(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/apps/hub/get/{id}",
      path: {
        id: data.id,
      },
    });
  }
  /**
   * list apps for search
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown app list
   * @throws ApiError
   */
  static listSearchApp(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/list_search",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list all apps
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.pathStart mask to filter matching starting path
   * @param data.pathExact mask to filter exact matching path
   * @param data.starredOnly (default false)
   * show only the starred items
   *
   * @returns ListableApp All apps
   * @throws ApiError
   */
  static listApps(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        path_start: data.pathStart,
        path_exact: data.pathExact,
        starred_only: data.starredOnly,
      },
    });
  }
  /**
   * create app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new app
   * @returns string app created
   * @throws ApiError
   */
  static createApp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/apps/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * does an app exisst at path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean app exists
   * @throws ApiError
   */
  static existsApp(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get app by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns AppWithLastVersion app details
   * @throws ApiError
   */
  static getAppByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/get/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get app by path with draft
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns AppWithLastVersionWDraft app details with draft
   * @throws ApiError
   */
  static getAppByPathWithDraft(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/get/draft/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get app history by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns AppHistory app history
   * @throws ApiError
   */
  static getAppHistoryByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/history/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update app history
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.version
   * @param data.requestBody App deployment message
   * @returns string success
   * @throws ApiError
   */
  static updateAppHistory(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/apps/history_update/a/{id}/v/{version}",
      path: {
        workspace: data.workspace,
        id: data.id,
        version: data.version,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get public app by secret
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns AppWithLastVersion app details
   * @throws ApiError
   */
  static getPublicAppBySecret(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps_u/public_app/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get public resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown resource value
   * @throws ApiError
   */
  static getPublicResource(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps_u/public_resource/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get public secret of app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string app secret
   * @throws ApiError
   */
  static getPublicSecretOfApp(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/secret_of/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get app by version
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns AppWithLastVersion app details
   * @throws ApiError
   */
  static getAppByVersion(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/get/v/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * delete app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string app deleted
   * @throws ApiError
   */
  static deleteApp(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/apps/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody update app
   * @returns string app updated
   * @throws ApiError
   */
  static updateApp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/apps/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * executeComponent
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody update app
   * @returns string job uuid
   * @throws ApiError
   */
  static executeComponent(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/apps_u/execute_component/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var ScriptService = class {
  /**
   * get hub script content by path
   * @param data The data for the request.
   * @param data.path
   * @returns string script details
   * @throws ApiError
   */
  static getHubScriptContentByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/scripts/hub/get/{path}",
      path: {
        path: data.path,
      },
    });
  }
  /**
   * get full hub script by path
   * @param data The data for the request.
   * @param data.path
   * @returns unknown script details
   * @throws ApiError
   */
  static getHubScriptByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/scripts/hub/get_full/{path}",
      path: {
        path: data.path,
      },
    });
  }
  /**
   * get top hub scripts
   * @param data The data for the request.
   * @param data.limit query limit
   * @param data.app query scripts app
   * @param data.kind query scripts kind
   * @returns unknown hub scripts list
   * @throws ApiError
   */
  static getTopHubScripts(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/scripts/hub/top",
      query: {
        limit: data.limit,
        app: data.app,
        kind: data.kind,
      },
    });
  }
  /**
   * query hub scripts by similarity
   * @param data The data for the request.
   * @param data.text query text
   * @param data.kind query scripts kind
   * @param data.limit query limit
   * @param data.app query scripts app
   * @returns unknown script details
   * @throws ApiError
   */
  static queryHubScripts(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/embeddings/query_hub_scripts",
      query: {
        text: data.text,
        kind: data.kind,
        limit: data.limit,
        app: data.app,
      },
    });
  }
  /**
   * list scripts for search
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown script list
   * @throws ApiError
   */
  static listSearchScript(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/list_search",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list all scripts
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.pathStart mask to filter matching starting path
   * @param data.pathExact mask to filter exact matching path
   * @param data.firstParentHash mask to filter scripts whom first direct parent has exact hash
   * @param data.lastParentHash mask to filter scripts whom last parent in the chain has exact hash.
   * Beware that each script stores only a limited number of parents. Hence
   * the last parent hash for a script is not necessarily its top-most parent.
   * To find the top-most parent you will have to jump from last to last hash
   * until finding the parent
   *
   * @param data.parentHash is the hash present in the array of stored parent hashes for this script.
   * The same warning applies than for last_parent_hash. A script only store a
   * limited number of direct parent
   *
   * @param data.showArchived (default false)
   * show also the archived files.
   * when multiple archived hash share the same path, only the ones with the latest create_at
   * are
   * ed.
   *
   * @param data.includeWithoutMain (default false)
   * include scripts without an exported main function
   *
   * @param data.isTemplate (default regardless)
   * if true show only the templates
   * if false show only the non templates
   * if not defined, show all regardless of if the script is a template
   *
   * @param data.kinds (default regardless)
   * script kinds to filter, split by comma
   *
   * @param data.starredOnly (default false)
   * show only the starred items
   *
   * @returns Script All scripts
   * @throws ApiError
   */
  static listScripts(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        path_start: data.pathStart,
        path_exact: data.pathExact,
        first_parent_hash: data.firstParentHash,
        last_parent_hash: data.lastParentHash,
        parent_hash: data.parentHash,
        show_archived: data.showArchived,
        includeWithoutMain: data.includeWithoutMain,
        is_template: data.isTemplate,
        kinds: data.kinds,
        starred_only: data.starredOnly,
      },
    });
  }
  /**
   * list all scripts paths
   * @param data The data for the request.
   * @param data.workspace
   * @returns string list of script paths
   * @throws ApiError
   */
  static listScriptPaths(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/list_paths",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * create script
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Partially filled script
   * @returns string script created
   * @throws ApiError
   */
  static createScript(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Toggle ON and OFF the workspace error handler for a given script
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody Workspace error handler enabled
   * @returns string error handler toggled
   * @throws ApiError
   */
  static toggleWorkspaceErrorHandlerForScript(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/toggle_workspace_error_handler/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * archive script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string script archived
   * @throws ApiError
   */
  static archiveScriptByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/archive/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * archive script by hash
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @returns Script script details
   * @throws ApiError
   */
  static archiveScriptByHash(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/archive/h/{hash}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
      },
    });
  }
  /**
   * delete script by hash (erase content but keep hash, require admin)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @returns Script script details
   * @throws ApiError
   */
  static deleteScriptByHash(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/delete/h/{hash}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
      },
    });
  }
  /**
   * delete all scripts at a given path (require admin)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string script path
   * @throws ApiError
   */
  static deleteScriptByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/delete/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns Script script details
   * @throws ApiError
   */
  static getScriptByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/get/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get script by path with draft
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns NewScriptWithDraft script details
   * @throws ApiError
   */
  static getScriptByPathWithDraft(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/get/draft/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get history of a script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns ScriptHistory script history
   * @throws ApiError
   */
  static getScriptHistoryByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/history/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * update history of a script
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @param data.path
   * @param data.requestBody Script deployment message
   * @returns string success
   * @throws ApiError
   */
  static updateScriptHistory(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/scripts/history_update/h/{hash}/p/{path}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * raw script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string script content
   * @throws ApiError
   */
  static rawScriptByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/raw/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * raw script by path with a token (mostly used by lsp to be used with import maps to resolve scripts)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.token
   * @param data.path
   * @returns string script content
   * @throws ApiError
   */
  static rawScriptByPathTokened(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/scripts_u/tokened_raw/{workspace}/{token}/{path}",
      path: {
        workspace: data.workspace,
        token: data.token,
        path: data.path,
      },
    });
  }
  /**
   * exists script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean does it exists
   * @throws ApiError
   */
  static existsScriptByPath(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/exists/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get script by hash
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @returns Script script details
   * @throws ApiError
   */
  static getScriptByHash(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/get/h/{hash}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
      },
    });
  }
  /**
   * raw script by hash
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string script content
   * @throws ApiError
   */
  static rawScriptByHash(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/raw/h/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get script deployment status
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @returns unknown script details
   * @throws ApiError
   */
  static getScriptDeploymentStatus(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/scripts/deployment_status/h/{hash}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
      },
    });
  }
};
var DraftService = class {
  /**
   * create draft
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns string draft created
   * @throws ApiError
   */
  static createDraft(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/drafts/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete draft
   * @param data The data for the request.
   * @param data.workspace
   * @param data.kind
   * @param data.path
   * @returns string draft deleted
   * @throws ApiError
   */
  static deleteDraft(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/drafts/delete/{kind}/{path}",
      path: {
        workspace: data.workspace,
        kind: data.kind,
        path: data.path,
      },
    });
  }
};
var WorkerService = class {
  /**
   * get all instance custom tags (tags are used to dispatch jobs to different worker groups)
   * @returns string list of custom tags
   * @throws ApiError
   */
  static getCustomTags() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/custom_tags",
    });
  }
  /**
   * get all instance default tags
   * @returns string list of default tags
   * @throws ApiError
   */
  static geDefaultTags() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/get_default_tags",
    });
  }
  /**
   * is default tags per workspace
   * @returns boolean is the default tags per workspace
   * @throws ApiError
   */
  static isDefaultTagsPerWorkspace() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/is_default_tags_per_workspace",
    });
  }
  /**
   * list workers
   * @param data The data for the request.
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.pingSince number of seconds the worker must have had a last ping more recent of (default to 300)
   * @returns WorkerPing a list of workers
   * @throws ApiError
   */
  static listWorkers(data = {}) {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/list",
      query: {
        page: data.page,
        per_page: data.perPage,
        ping_since: data.pingSince,
      },
    });
  }
  /**
   * exists worker with tag
   * @param data The data for the request.
   * @param data.tag
   * @returns boolean whether a worker with the tag exists
   * @throws ApiError
   */
  static existsWorkerWithTag(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/exists_worker_with_tag",
      query: {
        tag: data.tag,
      },
    });
  }
  /**
   * get queue metrics
   * @returns unknown metrics
   * @throws ApiError
   */
  static getQueueMetrics() {
    return request(OpenAPI, {
      method: "GET",
      url: "/workers/queue_metrics",
    });
  }
};
var JobService = class {
  /**
   * run script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody script args
   * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
   * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
   * @returns string job created
   * @throws ApiError
   */
  static runScriptByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        scheduled_for: data.scheduledFor,
        scheduled_in_secs: data.scheduledInSecs,
        parent_job: data.parentJob,
        tag: data.tag,
        cache_ttl: data.cacheTtl,
        job_id: data.jobId,
        invisible_to_owner: data.invisibleToOwner,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run script by path in openai format
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody script args
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
   *
   * @returns unknown job result
   * @throws ApiError
   */
  static openaiSyncScriptByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/openai_sync/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        parent_job: data.parentJob,
        job_id: data.jobId,
        include_header: data.includeHeader,
        queue_limit: data.queueLimit,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run script by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody script args
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
   *
   * @returns unknown job result
   * @throws ApiError
   */
  static runWaitResultScriptByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run_wait_result/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        parent_job: data.parentJob,
        tag: data.tag,
        cache_ttl: data.cacheTtl,
        job_id: data.jobId,
        include_header: data.includeHeader,
        queue_limit: data.queueLimit,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run script by path with get
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
   *
   * @param data.payload The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
   * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
   *
   * @returns unknown job result
   * @throws ApiError
   */
  static runWaitResultScriptByPathGet(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/run_wait_result/p/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        parent_job: data.parentJob,
        tag: data.tag,
        cache_ttl: data.cacheTtl,
        job_id: data.jobId,
        include_header: data.includeHeader,
        queue_limit: data.queueLimit,
        payload: data.payload,
      },
    });
  }
  /**
   * run flow by path and wait until completion in openai format
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody script args
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
   *
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @returns unknown job result
   * @throws ApiError
   */
  static openaiSyncFlowByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/openai_sync/f/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        include_header: data.includeHeader,
        queue_limit: data.queueLimit,
        job_id: data.jobId,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run flow by path and wait until completion
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody script args
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.queueLimit The maximum size of the queue for which the request would get rejected if that job would push it above that limit
   *
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @returns unknown job result
   * @throws ApiError
   */
  static runWaitResultFlowByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run_wait_result/f/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        include_header: data.includeHeader,
        queue_limit: data.queueLimit,
        job_id: data.jobId,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get job result by id
   * @param data The data for the request.
   * @param data.workspace
   * @param data.flowJobId
   * @param data.nodeId
   * @returns unknown job result
   * @throws ApiError
   */
  static resultById(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/result_by_id/{flow_job_id}/{node_id}",
      path: {
        workspace: data.workspace,
        flow_job_id: data.flowJobId,
        node_id: data.nodeId,
      },
    });
  }
  /**
   * run flow by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody flow args
   * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
   * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.invisibleToOwner make the run invisible to the the flow owner (default false)
   * @returns string job created
   * @throws ApiError
   */
  static runFlowByPath(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/f/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        scheduled_for: data.scheduledFor,
        scheduled_in_secs: data.scheduledInSecs,
        parent_job: data.parentJob,
        tag: data.tag,
        job_id: data.jobId,
        include_header: data.includeHeader,
        invisible_to_owner: data.invisibleToOwner,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * restart a completed flow at a given step
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.stepId step id to restart the flow from
   * @param data.branchOrIterationN for branchall or loop, the iteration at which the flow should restart
   * @param data.requestBody flow args
   * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
   * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.invisibleToOwner make the run invisible to the the flow owner (default false)
   * @returns string job created
   * @throws ApiError
   */
  static restartFlowAtStep(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/restart/f/{id}/from/{step_id}/{branch_or_iteration_n}",
      path: {
        workspace: data.workspace,
        id: data.id,
        step_id: data.stepId,
        branch_or_iteration_n: data.branchOrIterationN,
      },
      query: {
        scheduled_for: data.scheduledFor,
        scheduled_in_secs: data.scheduledInSecs,
        parent_job: data.parentJob,
        tag: data.tag,
        job_id: data.jobId,
        include_header: data.includeHeader,
        invisible_to_owner: data.invisibleToOwner,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run script by hash
   * @param data The data for the request.
   * @param data.workspace
   * @param data.hash
   * @param data.requestBody Partially filled args
   * @param data.scheduledFor when to schedule this job (leave empty for immediate run)
   * @param data.scheduledInSecs schedule the script to execute in the number of seconds starting now
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.tag Override the tag to use
   * @param data.cacheTtl Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
   * @returns string job created
   * @throws ApiError
   */
  static runScriptByHash(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/h/{hash}",
      path: {
        workspace: data.workspace,
        hash: data.hash,
      },
      query: {
        scheduled_for: data.scheduledFor,
        scheduled_in_secs: data.scheduledInSecs,
        parent_job: data.parentJob,
        tag: data.tag,
        cache_ttl: data.cacheTtl,
        job_id: data.jobId,
        include_header: data.includeHeader,
        invisible_to_owner: data.invisibleToOwner,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run script preview
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody preview
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @returns string job created
   * @throws ApiError
   */
  static runScriptPreview(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/preview",
      path: {
        workspace: data.workspace,
      },
      query: {
        include_header: data.includeHeader,
        invisible_to_owner: data.invisibleToOwner,
        job_id: data.jobId,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run code-workflow task
   * @param data The data for the request.
   * @param data.workspace
   * @param data.jobId
   * @param data.entrypoint
   * @param data.requestBody preview
   * @returns string job created
   * @throws ApiError
   */
  static runCodeWorkflowTask(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/workflow_as_code/{job_id}/{entrypoint}",
      path: {
        workspace: data.workspace,
        job_id: data.jobId,
        entrypoint: data.entrypoint,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run a one-off dependencies job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody raw script content
   * @returns unknown dependency job result
   * @throws ApiError
   */
  static runRawScriptDependencies(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/dependencies",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * run flow preview
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody preview
   * @param data.includeHeader List of headers's keys (separated with ',') whove value are added to the args
   * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
   *
   * @param data.invisibleToOwner make the run invisible to the the script owner (default false)
   * @param data.jobId The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
   * @returns string job created
   * @throws ApiError
   */
  static runFlowPreview(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/run/preview_flow",
      path: {
        workspace: data.workspace,
      },
      query: {
        include_header: data.includeHeader,
        invisible_to_owner: data.invisibleToOwner,
        job_id: data.jobId,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * list all queued jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.scriptPathExact mask to filter exact matching path
   * @param data.scriptPathStart mask to filter matching starting path
   * @param data.schedulePath mask to filter by schedule path
   * @param data.scriptHash mask to filter exact matching path
   * @param data.startedBefore filter on started before (inclusive) timestamp
   * @param data.startedAfter filter on started after (exclusive) timestamp
   * @param data.success filter on successful jobs
   * @param data.scheduledForBeforeNow filter on jobs scheduled_for before now (hence waitinf for a worker)
   * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
   * @param data.suspended filter on suspended jobs
   * @param data.running filter on running jobs
   * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
   * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
   * @param data.tag filter on jobs with a given tag/worker group
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
   * @param data.isNotSchedule is not a scheduled job
   * @returns QueuedJob All queued jobs
   * @throws ApiError
   */
  static listQueue(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/queue/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        parent_job: data.parentJob,
        script_path_exact: data.scriptPathExact,
        script_path_start: data.scriptPathStart,
        schedule_path: data.schedulePath,
        script_hash: data.scriptHash,
        started_before: data.startedBefore,
        started_after: data.startedAfter,
        success: data.success,
        scheduled_for_before_now: data.scheduledForBeforeNow,
        job_kinds: data.jobKinds,
        suspended: data.suspended,
        running: data.running,
        args: data.args,
        result: data.result,
        tag: data.tag,
        page: data.page,
        per_page: data.perPage,
        all_workspaces: data.allWorkspaces,
        is_not_schedule: data.isNotSchedule,
      },
    });
  }
  /**
   * get queue count
   * @param data The data for the request.
   * @param data.workspace
   * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
   * @returns unknown queue count
   * @throws ApiError
   */
  static getQueueCount(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/queue/count",
      path: {
        workspace: data.workspace,
      },
      query: {
        all_workspaces: data.allWorkspaces,
      },
    });
  }
  /**
   * get completed count
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown completed count
   * @throws ApiError
   */
  static getCompletedCount(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/completed/count",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * cancel all jobs
   * @param data The data for the request.
   * @param data.workspace
   * @returns string uuids of canceled jobs
   * @throws ApiError
   */
  static cancelAll(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/queue/cancel_all",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * list all completed jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.label mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.scriptPathExact mask to filter exact matching path
   * @param data.scriptPathStart mask to filter matching starting path
   * @param data.schedulePath mask to filter by schedule path
   * @param data.scriptHash mask to filter exact matching path
   * @param data.startedBefore filter on started before (inclusive) timestamp
   * @param data.startedAfter filter on started after (exclusive) timestamp
   * @param data.success filter on successful jobs
   * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
   * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
   * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
   * @param data.tag filter on jobs with a given tag/worker group
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.isSkipped is the job skipped
   * @param data.isFlowStep is the job a flow step
   * @param data.hasNullParent has null parent
   * @param data.isNotSchedule is not a scheduled job
   * @returns CompletedJob All completed jobs
   * @throws ApiError
   */
  static listCompletedJobs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/completed/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        label: data.label,
        parent_job: data.parentJob,
        script_path_exact: data.scriptPathExact,
        script_path_start: data.scriptPathStart,
        schedule_path: data.schedulePath,
        script_hash: data.scriptHash,
        started_before: data.startedBefore,
        started_after: data.startedAfter,
        success: data.success,
        job_kinds: data.jobKinds,
        args: data.args,
        result: data.result,
        tag: data.tag,
        page: data.page,
        per_page: data.perPage,
        is_skipped: data.isSkipped,
        is_flow_step: data.isFlowStep,
        has_null_parent: data.hasNullParent,
        is_not_schedule: data.isNotSchedule,
      },
    });
  }
  /**
   * list all jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.label mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
   * @param data.parentJob The parent job that is at the origin and responsible for the execution of this script if any
   * @param data.scriptPathExact mask to filter exact matching path
   * @param data.scriptPathStart mask to filter matching starting path
   * @param data.schedulePath mask to filter by schedule path
   * @param data.scriptHash mask to filter exact matching path
   * @param data.startedBefore filter on started before (inclusive) timestamp
   * @param data.startedAfter filter on started after (exclusive) timestamp
   * @param data.createdOrStartedBefore filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
   * @param data.running filter on running jobs
   * @param data.scheduledForBeforeNow filter on jobs scheduled_for before now (hence waitinf for a worker)
   * @param data.createdOrStartedAfter filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
   * @param data.jobKinds filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
   * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
   * @param data.tag filter on jobs with a given tag/worker group
   * @param data.result filter on jobs containing those result as a json subset (@> in postgres)
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.isSkipped is the job skipped
   * @param data.isFlowStep is the job a flow step
   * @param data.hasNullParent has null parent
   * @param data.success filter on successful jobs
   * @param data.allWorkspaces get jobs from all workspaces (only valid if request come from the `admins` workspace)
   * @param data.isNotSchedule is not a scheduled job
   * @returns Job All jobs
   * @throws ApiError
   */
  static listJobs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        created_by: data.createdBy,
        label: data.label,
        parent_job: data.parentJob,
        script_path_exact: data.scriptPathExact,
        script_path_start: data.scriptPathStart,
        schedule_path: data.schedulePath,
        script_hash: data.scriptHash,
        started_before: data.startedBefore,
        started_after: data.startedAfter,
        created_or_started_before: data.createdOrStartedBefore,
        running: data.running,
        scheduled_for_before_now: data.scheduledForBeforeNow,
        created_or_started_after: data.createdOrStartedAfter,
        job_kinds: data.jobKinds,
        args: data.args,
        tag: data.tag,
        result: data.result,
        page: data.page,
        per_page: data.perPage,
        is_skipped: data.isSkipped,
        is_flow_step: data.isFlowStep,
        has_null_parent: data.hasNullParent,
        success: data.success,
        all_workspaces: data.allWorkspaces,
        is_not_schedule: data.isNotSchedule,
      },
    });
  }
  /**
   * get db clock
   * @returns number the timestamp of the db that can be used to compute the drift
   * @throws ApiError
   */
  static getDbClock() {
    return request(OpenAPI, {
      method: "GET",
      url: "/jobs/db_clock",
    });
  }
  /**
   * get job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.noLogs
   * @returns Job job details
   * @throws ApiError
   */
  static getJob(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      query: {
        no_logs: data.noLogs,
      },
    });
  }
  /**
   * get root job id
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns string get root job id
   * @throws ApiError
   */
  static getRootJobId(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get_root_job_id/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * get job logs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns string job details
   * @throws ApiError
   */
  static getJobLogs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get_logs/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * get job updates
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.running
   * @param data.logOffset
   * @returns unknown job details
   * @throws ApiError
   */
  static getJobUpdates(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/getupdate/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      query: {
        running: data.running,
        log_offset: data.logOffset,
      },
    });
  }
  /**
   * get log file from object store
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown job log
   * @throws ApiError
   */
  static getLogFileFromStore(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get_log_file/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get flow debug info
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns unknown flow debug info details
   * @throws ApiError
   */
  static getFlowDebugInfo(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get_flow_debug_info/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * get completed job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns CompletedJob job details
   * @throws ApiError
   */
  static getCompletedJob(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/completed/get/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * get completed job result
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns unknown result
   * @throws ApiError
   */
  static getCompletedJobResult(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/completed/get_result/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * get completed job result if job is completed
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.getStarted
   * @returns unknown result
   * @throws ApiError
   */
  static getCompletedJobResultMaybe(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/completed/get_result_maybe/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      query: {
        get_started: data.getStarted,
      },
    });
  }
  /**
   * delete completed job (erase content but keep run id)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @returns CompletedJob job details
   * @throws ApiError
   */
  static deleteCompletedJob(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/completed/delete/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
    });
  }
  /**
   * cancel queued job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.requestBody reason
   * @returns string job canceled
   * @throws ApiError
   */
  static cancelQueuedJob(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs_u/queue/cancel/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * cancel all queued jobs for persistent script
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody reason
   * @returns string persistent job scaled down to zero
   * @throws ApiError
   */
  static cancelPersistentQueuedJobs(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs_u/queue/cancel_persistent/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * force cancel queued job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.requestBody reason
   * @returns string job canceled
   * @throws ApiError
   */
  static forceCancelQueuedJob(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs_u/queue/force_cancel/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * create an HMac signature given a job id and a resume id
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.approver
   * @returns string job signature
   * @throws ApiError
   */
  static createJobSignature(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/job_signature/{id}/{resume_id}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
      },
      query: {
        approver: data.approver,
      },
    });
  }
  /**
   * get resume urls given a job_id, resume_id and a nonce to resume a flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.approver
   * @returns unknown url endpoints
   * @throws ApiError
   */
  static getResumeUrls(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/resume_urls/{id}/{resume_id}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
      },
      query: {
        approver: data.approver,
      },
    });
  }
  /**
   * resume a job for a suspended flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.signature
   * @param data.payload The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
   * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
   *
   * @param data.approver
   * @returns string job resumed
   * @throws ApiError
   */
  static resumeSuspendedJobGet(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
        signature: data.signature,
      },
      query: {
        payload: data.payload,
        approver: data.approver,
      },
    });
  }
  /**
   * resume a job for a suspended flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.signature
   * @param data.requestBody
   * @param data.approver
   * @returns string job resumed
   * @throws ApiError
   */
  static resumeSuspendedJobPost(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
        signature: data.signature,
      },
      query: {
        approver: data.approver,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * set flow user state at a given key
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.key
   * @param data.requestBody new value
   * @returns string flow user state updated
   * @throws ApiError
   */
  static setFlowUserState(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/flow/user_states/{id}/{key}",
      path: {
        workspace: data.workspace,
        id: data.id,
        key: data.key,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get flow user state at a given key
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.key
   * @returns unknown flow user state updated
   * @throws ApiError
   */
  static getFlowUserState(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs/flow/user_states/{id}/{key}",
      path: {
        workspace: data.workspace,
        id: data.id,
        key: data.key,
      },
    });
  }
  /**
   * resume a job for a suspended flow as an owner
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.requestBody
   * @returns string job resumed
   * @throws ApiError
   */
  static resumeSuspendedFlowAsOwner(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs/flow/resume/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * cancel a job for a suspended flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.signature
   * @param data.approver
   * @returns string job canceled
   * @throws ApiError
   */
  static cancelSuspendedJobGet(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/cancel/{id}/{resume_id}/{signature}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
        signature: data.signature,
      },
      query: {
        approver: data.approver,
      },
    });
  }
  /**
   * cancel a job for a suspended flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.signature
   * @param data.requestBody
   * @param data.approver
   * @returns string job canceled
   * @throws ApiError
   */
  static cancelSuspendedJobPost(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/jobs_u/cancel/{id}/{resume_id}/{signature}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
        signature: data.signature,
      },
      query: {
        approver: data.approver,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * get parent flow job of suspended job
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.resumeId
   * @param data.signature
   * @param data.approver
   * @returns unknown parent flow details
   * @throws ApiError
   */
  static getSuspendedJobFlow(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/jobs_u/get_flow/{id}/{resume_id}/{signature}",
      path: {
        workspace: data.workspace,
        id: data.id,
        resume_id: data.resumeId,
        signature: data.signature,
      },
      query: {
        approver: data.approver,
      },
    });
  }
};
var RawAppService = class {
  /**
   * list all raw apps
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.orderDesc order by desc order (default true)
   * @param data.createdBy mask to filter exact matching user creator
   * @param data.pathStart mask to filter matching starting path
   * @param data.pathExact mask to filter exact matching path
   * @param data.starredOnly (default false)
   * show only the starred items
   *
   * @returns ListableRawApp All raw apps
   * @throws ApiError
   */
  static listRawApps(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/raw_apps/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        order_desc: data.orderDesc,
        created_by: data.createdBy,
        path_start: data.pathStart,
        path_exact: data.pathExact,
        starred_only: data.starredOnly,
      },
    });
  }
  /**
   * does an app exisst at path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean app exists
   * @throws ApiError
   */
  static existsRawApp(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/raw_apps/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get app by path
   * @param data The data for the request.
   * @param data.workspace
   * @param data.version
   * @param data.path
   * @returns string app details
   * @throws ApiError
   */
  static getRawAppData(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/apps/get_data/{version}/{path}",
      path: {
        workspace: data.workspace,
        version: data.version,
        path: data.path,
      },
    });
  }
  /**
   * create raw app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new raw app
   * @returns string raw app created
   * @throws ApiError
   */
  static createRawApp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/raw_apps/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updateraw  app
   * @returns string app updated
   * @throws ApiError
   */
  static updateRawApp(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/raw_apps/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete raw app
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string app deleted
   * @throws ApiError
   */
  static deleteRawApp(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/raw_apps/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
};
var ScheduleService = class {
  /**
   * preview schedule
   * @param data The data for the request.
   * @param data.requestBody schedule
   * @returns string List of 5 estimated upcoming execution events (in UTC)
   * @throws ApiError
   */
  static previewSchedule(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/schedules/preview",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * create schedule
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody new schedule
   * @returns string schedule created
   * @throws ApiError
   */
  static createSchedule(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/schedules/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update schedule
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated schedule
   * @returns string schedule updated
   * @throws ApiError
   */
  static updateSchedule(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/schedules/update/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * set enabled schedule
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.requestBody updated schedule enable
   * @returns string schedule enabled set
   * @throws ApiError
   */
  static setScheduleEnabled(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/schedules/setenabled/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete schedule
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns string schedule deleted
   * @throws ApiError
   */
  static deleteSchedule(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/schedules/delete/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get schedule
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns Schedule schedule deleted
   * @throws ApiError
   */
  static getSchedule(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/schedules/get/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * does schedule exists
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns boolean schedule exists
   * @throws ApiError
   */
  static existsSchedule(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/schedules/exists/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * list schedules
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @param data.args filter on jobs containing those args as a json subset (@> in postgres)
   * @param data.path filter by path
   * @param data.isFlow
   * @returns Schedule schedule list
   * @throws ApiError
   */
  static listSchedules(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/schedules/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
        args: data.args,
        path: data.path,
        is_flow: data.isFlow,
      },
    });
  }
  /**
   * list schedules with last 20 jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns ScheduleWJobs schedule list
   * @throws ApiError
   */
  static listSchedulesWithJobs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/schedules/list_with_jobs",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * Set default error or recoevery handler
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Handler description
   * @returns unknown default error handler set
   * @throws ApiError
   */
  static setDefaultErrorOrRecoveryHandler(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/schedules/setdefaulthandler",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var GroupService = class {
  /**
   * list instance groups
   * @returns InstanceGroup instance group list
   * @throws ApiError
   */
  static listInstanceGroups() {
    return request(OpenAPI, {
      method: "GET",
      url: "/groups/list",
    });
  }
  /**
   * get instance group
   * @param data The data for the request.
   * @param data.name
   * @returns InstanceGroup instance group
   * @throws ApiError
   */
  static getInstanceGroup(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/groups/get/{name}",
      path: {
        name: data.name,
      },
    });
  }
  /**
   * create instance group
   * @param data The data for the request.
   * @param data.requestBody create instance group
   * @returns string instance group created
   * @throws ApiError
   */
  static createInstanceGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/groups/create",
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update instance group
   * @param data The data for the request.
   * @param data.name
   * @param data.requestBody update instance group
   * @returns string instance group updated
   * @throws ApiError
   */
  static updateInstanceGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/groups/update/{name}",
      path: {
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete instance group
   * @param data The data for the request.
   * @param data.name
   * @returns string instance group deleted
   * @throws ApiError
   */
  static deleteInstanceGroup(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/groups/delete/{name}",
      path: {
        name: data.name,
      },
    });
  }
  /**
   * add user to instance group
   * @param data The data for the request.
   * @param data.name
   * @param data.requestBody user to add to instance group
   * @returns string user added to instance group
   * @throws ApiError
   */
  static addUserToInstanceGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/groups/adduser/{name}",
      path: {
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * remove user from instance group
   * @param data The data for the request.
   * @param data.name
   * @param data.requestBody user to remove from instance group
   * @returns string user removed from instance group
   * @throws ApiError
   */
  static removeUserFromInstanceGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/groups/removeuser/{name}",
      path: {
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * list groups
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Group group list
   * @throws ApiError
   */
  static listGroups(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/groups/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * list group names
   * @param data The data for the request.
   * @param data.workspace
   * @param data.onlyMemberOf only list the groups the user is member of (default false)
   * @returns string group list
   * @throws ApiError
   */
  static listGroupNames(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/groups/listnames",
      path: {
        workspace: data.workspace,
      },
      query: {
        only_member_of: data.onlyMemberOf,
      },
    });
  }
  /**
   * create group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody create group
   * @returns string group created
   * @throws ApiError
   */
  static createGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/groups/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody updated group
   * @returns string group updated
   * @throws ApiError
   */
  static updateGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/groups/update/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns string group deleted
   * @throws ApiError
   */
  static deleteGroup(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/groups/delete/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * get group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns Group group
   * @throws ApiError
   */
  static getGroup(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/groups/get/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * add user to group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody added user to group
   * @returns string user added to group
   * @throws ApiError
   */
  static addUserToGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/groups/adduser/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * remove user to group
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody added user to group
   * @returns string user removed from group
   * @throws ApiError
   */
  static removeUserToGroup(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/groups/removeuser/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var FolderService = class {
  /**
   * list folders
   * @param data The data for the request.
   * @param data.workspace
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Folder folder list
   * @throws ApiError
   */
  static listFolders(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/folders/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * list folder names
   * @param data The data for the request.
   * @param data.workspace
   * @param data.onlyMemberOf only list the folders the user is member of (default false)
   * @returns string folder list
   * @throws ApiError
   */
  static listFolderNames(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/folders/listnames",
      path: {
        workspace: data.workspace,
      },
      query: {
        only_member_of: data.onlyMemberOf,
      },
    });
  }
  /**
   * create folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody create folder
   * @returns string folder created
   * @throws ApiError
   */
  static createFolder(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/folders/create",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * update folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody update folder
   * @returns string folder updated
   * @throws ApiError
   */
  static updateFolder(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/folders/update/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * delete folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns string folder deleted
   * @throws ApiError
   */
  static deleteFolder(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/folders/delete/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * get folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns Folder folder
   * @throws ApiError
   */
  static getFolder(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/folders/get/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * get folder usage
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @returns unknown folder
   * @throws ApiError
   */
  static getFolderUsage(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/folders/getusage/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
    });
  }
  /**
   * add owner to folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody owner user to folder
   * @returns string owner added to folder
   * @throws ApiError
   */
  static addOwnerToFolder(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/folders/addowner/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * remove owner to folder
   * @param data The data for the request.
   * @param data.workspace
   * @param data.name
   * @param data.requestBody added owner to folder
   * @returns string owner removed from folder
   * @throws ApiError
   */
  static removeOwnerToFolder(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/folders/removeowner/{name}",
      path: {
        workspace: data.workspace,
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var ConfigService = class {
  /**
   * list worker groups
   * @returns unknown a list of worker group configs
   * @throws ApiError
   */
  static listWorkerGroups() {
    return request(OpenAPI, {
      method: "GET",
      url: "/configs/list_worker_groups",
    });
  }
  /**
   * get config
   * @param data The data for the request.
   * @param data.name
   * @returns unknown a config
   * @throws ApiError
   */
  static getConfig(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/configs/get/{name}",
      path: {
        name: data.name,
      },
    });
  }
  /**
   * Update config
   * @param data The data for the request.
   * @param data.name
   * @param data.requestBody worker group
   * @returns string Update a worker group
   * @throws ApiError
   */
  static updateConfig(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/configs/update/{name}",
      path: {
        name: data.name,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Delete Config
   * @param data The data for the request.
   * @param data.name
   * @returns string Delete config
   * @throws ApiError
   */
  static deleteConfig(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/configs/update/{name}",
      path: {
        name: data.name,
      },
    });
  }
};
var GranularAclService = class {
  /**
   * get granular acls
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.kind
   * @returns boolean acls
   * @throws ApiError
   */
  static getGranularAcls(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/acls/get/{kind}/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
        kind: data.kind,
      },
    });
  }
  /**
   * add granular acls
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.kind
   * @param data.requestBody acl to add
   * @returns string granular acl added
   * @throws ApiError
   */
  static addGranularAcls(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/acls/add/{kind}/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
        kind: data.kind,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * remove granular acls
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.kind
   * @param data.requestBody acl to add
   * @returns string granular acl removed
   * @throws ApiError
   */
  static removeGranularAcls(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/acls/remove/{kind}/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
        kind: data.kind,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var CaptureService = class {
  /**
   * update flow preview capture
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns void flow preview captured
   * @throws ApiError
   */
  static updateCapture(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/capture_u/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * create flow preview capture
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown flow preview capture created
   * @throws ApiError
   */
  static createCapture(data) {
    return request(OpenAPI, {
      method: "PUT",
      url: "/w/{workspace}/capture/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
    });
  }
  /**
   * get flow preview capture
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @returns unknown captured flow preview
   * @throws ApiError
   */
  static getCapture(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/capture/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      errors: {
        404: "capture does not exist for this flow",
      },
    });
  }
};
var FavoriteService = class {
  /**
   * star item
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns unknown star item
   * @throws ApiError
   */
  static star(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/favorites/star",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * unstar item
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody
   * @returns unknown unstar item
   * @throws ApiError
   */
  static unstar(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/favorites/unstar",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var InputService = class {
  /**
   * List Inputs used in previously completed jobs
   * @param data The data for the request.
   * @param data.workspace
   * @param data.runnableId
   * @param data.runnableType
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Input Input history for completed jobs
   * @throws ApiError
   */
  static getInputHistory(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/inputs/history",
      path: {
        workspace: data.workspace,
      },
      query: {
        runnable_id: data.runnableId,
        runnable_type: data.runnableType,
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * Get args from history or saved input
   * @param data The data for the request.
   * @param data.workspace
   * @param data.jobOrInputId
   * @returns unknown args
   * @throws ApiError
   */
  static getArgsFromHistoryOrSavedInput(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/inputs/{jobOrInputId}/args",
      path: {
        workspace: data.workspace,
        jobOrInputId: data.jobOrInputId,
      },
    });
  }
  /**
   * List saved Inputs for a Runnable
   * @param data The data for the request.
   * @param data.workspace
   * @param data.runnableId
   * @param data.runnableType
   * @param data.page which page to return (start at 1, default 1)
   * @param data.perPage number of items to return for a given page (default 30, max 100)
   * @returns Input Saved Inputs for a Runnable
   * @throws ApiError
   */
  static listInputs(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/inputs/list",
      path: {
        workspace: data.workspace,
      },
      query: {
        runnable_id: data.runnableId,
        runnable_type: data.runnableType,
        page: data.page,
        per_page: data.perPage,
      },
    });
  }
  /**
   * Create an Input for future use in a script or flow
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody Input
   * @param data.runnableId
   * @param data.runnableType
   * @returns string Input created
   * @throws ApiError
   */
  static createInput(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/inputs/create",
      path: {
        workspace: data.workspace,
      },
      query: {
        runnable_id: data.runnableId,
        runnable_type: data.runnableType,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Update an Input
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody UpdateInput
   * @returns string Input updated
   * @throws ApiError
   */
  static updateInput(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/inputs/update",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Delete a Saved Input
   * @param data The data for the request.
   * @param data.workspace
   * @param data.input
   * @returns string Input deleted
   * @throws ApiError
   */
  static deleteInput(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/inputs/delete/{input}",
      path: {
        workspace: data.workspace,
        input: data.input,
      },
    });
  }
};
var HelpersService = class {
  /**
   * Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody S3 resource to connect to
   * @returns unknown Connection settings
   * @throws ApiError
   */
  static duckdbConnectionSettings(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/duckdb_connection_settings",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Converts an S3 resource to the set of instructions necessary to connect DuckDB to an S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
   * @returns unknown Connection settings
   * @throws ApiError
   */
  static duckdbConnectionSettingsV2(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/v2/duckdb_connection_settings",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody S3 resource to connect to
   * @returns unknown Connection settings
   * @throws ApiError
   */
  static polarsConnectionSettings(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/polars_connection_settings",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Converts an S3 resource to the set of arguments necessary to connect Polars to an S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
   * @returns unknown Connection settings
   * @throws ApiError
   */
  static polarsConnectionSettingsV2(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/v2/polars_connection_settings",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Returns the s3 resource associated to the provided path, or the workspace default S3 resource
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody S3 resource path to use. If empty, the S3 resource defined in the workspace settings will be used
   * @returns S3Resource Connection settings
   * @throws ApiError
   */
  static s3ResourceInfo(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/v2/s3_resource_info",
      path: {
        workspace: data.workspace,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
  /**
   * Test connection to the workspace datasets storage
   * @param data The data for the request.
   * @param data.workspace
   * @returns unknown Connection settings
   * @throws ApiError
   */
  static datasetStorageTestConnection(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/test_connection",
      path: {
        workspace: data.workspace,
      },
    });
  }
  /**
   * List the file keys available in the workspace files storage (S3)
   * @param data The data for the request.
   * @param data.workspace
   * @param data.maxKeys
   * @param data.marker
   * @param data.prefix
   * @returns unknown List of file keys
   * @throws ApiError
   */
  static listStoredFiles(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/list_stored_files",
      path: {
        workspace: data.workspace,
      },
      query: {
        max_keys: data.maxKeys,
        marker: data.marker,
        prefix: data.prefix,
      },
    });
  }
  /**
   * Load metadata of the file
   * @param data The data for the request.
   * @param data.workspace
   * @param data.fileKey
   * @returns WindmillFileMetadata FileMetadata
   * @throws ApiError
   */
  static loadFileMetadata(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/load_file_metadata",
      path: {
        workspace: data.workspace,
      },
      query: {
        file_key: data.fileKey,
      },
    });
  }
  /**
   * Load a preview of the file
   * @param data The data for the request.
   * @param data.workspace
   * @param data.fileKey
   * @param data.fileSizeInBytes
   * @param data.fileMimeType
   * @param data.csvSeparator
   * @param data.csvHasHeader
   * @param data.readBytesFrom
   * @param data.readBytesLength
   * @returns WindmillFilePreview FilePreview
   * @throws ApiError
   */
  static loadFilePreview(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/load_file_preview",
      path: {
        workspace: data.workspace,
      },
      query: {
        file_key: data.fileKey,
        file_size_in_bytes: data.fileSizeInBytes,
        file_mime_type: data.fileMimeType,
        csv_separator: data.csvSeparator,
        csv_has_header: data.csvHasHeader,
        read_bytes_from: data.readBytesFrom,
        read_bytes_length: data.readBytesLength,
      },
    });
  }
  /**
   * Load a preview of a parquet file
   * @param data The data for the request.
   * @param data.workspace
   * @param data.path
   * @param data.offset
   * @param data.limit
   * @param data.sortCol
   * @param data.sortDesc
   * @param data.searchCol
   * @param data.searchTerm
   * @returns unknown Parquet Preview
   * @throws ApiError
   */
  static loadParquetPreview(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/load_parquet_preview/{path}",
      path: {
        workspace: data.workspace,
        path: data.path,
      },
      query: {
        offset: data.offset,
        limit: data.limit,
        sort_col: data.sortCol,
        sort_desc: data.sortDesc,
        search_col: data.searchCol,
        search_term: data.searchTerm,
      },
    });
  }
  /**
   * Permanently delete file from S3
   * @param data The data for the request.
   * @param data.workspace
   * @param data.fileKey
   * @returns unknown Confirmation
   * @throws ApiError
   */
  static deleteS3File(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/w/{workspace}/job_helpers/delete_s3_file",
      path: {
        workspace: data.workspace,
      },
      query: {
        file_key: data.fileKey,
      },
    });
  }
  /**
   * Move a S3 file from one path to the other within the same bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.srcFileKey
   * @param data.destFileKey
   * @returns unknown Confirmation
   * @throws ApiError
   */
  static moveS3File(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/move_s3_file",
      path: {
        workspace: data.workspace,
      },
      query: {
        src_file_key: data.srcFileKey,
        dest_file_key: data.destFileKey,
      },
    });
  }
  /**
   * Upload file to S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.requestBody File content
   * @param data.fileKey
   * @param data.fileExtension
   * @param data.s3ResourcePath
   * @param data.resourceType
   * @returns unknown File upload status
   * @throws ApiError
   */
  static fileUpload(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_helpers/upload_s3_file",
      path: {
        workspace: data.workspace,
      },
      query: {
        file_key: data.fileKey,
        file_extension: data.fileExtension,
        s3_resource_path: data.s3ResourcePath,
        resource_type: data.resourceType,
      },
      body: data.requestBody,
      mediaType: "application/octet-stream",
    });
  }
  /**
   * Download file to S3 bucket
   * @param data The data for the request.
   * @param data.workspace
   * @param data.fileKey
   * @param data.s3ResourcePath
   * @param data.resourceType
   * @returns binary Chunk of the downloaded file
   * @throws ApiError
   */
  static fileDownload(data) {
    return request(OpenAPI, {
      method: "GET",
      url: "/w/{workspace}/job_helpers/download_s3_file",
      path: {
        workspace: data.workspace,
      },
      query: {
        file_key: data.fileKey,
        s3_resource_path: data.s3ResourcePath,
        resource_type: data.resourceType,
      },
    });
  }
};
var MetricsService = class {
  /**
   * get job metrics
   * @param data The data for the request.
   * @param data.workspace
   * @param data.id
   * @param data.requestBody parameters for statistics retrieval
   * @returns unknown job details
   * @throws ApiError
   */
  static getJobMetrics(data) {
    return request(OpenAPI, {
      method: "POST",
      url: "/w/{workspace}/job_metrics/get/{id}",
      path: {
        workspace: data.workspace,
        id: data.id,
      },
      body: data.requestBody,
      mediaType: "application/json",
    });
  }
};
var ConcurrencyGroupsService = class {
  /**
   * List all concurrency groups
   * @returns ConcurrencyGroup all concurrency groups
   * @throws ApiError
   */
  static listConcurrencyGroups() {
    return request(OpenAPI, {
      method: "GET",
      url: "/concurrency_groups/list",
    });
  }
  /**
   * Delete concurrency group
   * @param data The data for the request.
   * @param data.concurrencyId
   * @returns unknown concurrency group removed
   * @throws ApiError
   */
  static deleteConcurrencyGroup(data) {
    return request(OpenAPI, {
      method: "DELETE",
      url: "/concurrency_groups/{concurrency_id}",
      path: {
        concurrency_id: data.concurrencyId,
      },
    });
  }
};

// src/client.ts
var clientSet = false;
function setClient(token, baseUrl) {
  if (baseUrl === void 0) {
    baseUrl =
      getEnv("BASE_INTERNAL_URL") ??
      getEnv("BASE_URL") ??
      "http://localhost:8000";
  }
  if (token === void 0) {
    token = getEnv("WM_TOKEN") ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
  clientSet = true;
}
var getEnv = (key) => {
  if (typeof window === "undefined") {
    return process.env[key];
  }
  return window.process.env[key];
};
function getWorkspace() {
  return getEnv("WM_WORKSPACE") ?? "no_workspace";
}
async function getResource(path, undefinedIfEmpty) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  path = path ?? getStatePath();
  try {
    return await ResourceService.getResourceValueInterpolated({
      workspace,
      path,
    });
  } catch (e) {
    if (undefinedIfEmpty && e.status === 404) {
      return void 0;
    } else {
      throw Error(
        `Resource not found at ${path} or not visible to you: ${e.body}`
      );
    }
  }
}
async function getRootJobId(jobId) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  jobId = jobId ?? getEnv("WM_JOB_ID");
  if (jobId === void 0) {
    throw Error("Job ID not set");
  }
  return await JobService.getRootJobId({ workspace, id: jobId });
}
async function runScript(
  path = null,
  hash_ = null,
  args = null,
  verbose = false
) {
  args = args || {};
  if (verbose) {
    console.info(`running \`${path}\` synchronously with args:`, args);
  }
  const jobId = await runScriptAsync(path, hash_, args);
  return await waitJob(jobId, verbose);
}
async function waitJob(jobId, verbose = false) {
  while (true) {
    const resultRes = await getResultMaybe(jobId);
    const started = resultRes.started;
    const completed = resultRes.completed;
    const success = resultRes.success;
    if (!started && verbose) {
      console.info(`job ${jobId} has not started yet`);
    }
    if (completed) {
      const result = resultRes.result;
      if (success) {
        return result;
      } else {
        const error = result.error;
        throw new Error(
          `Job ${jobId} was not successful: ${JSON.stringify(error)}`
        );
      }
    }
    if (verbose) {
      console.info(`sleeping 0.5 seconds for jobId: ${jobId}`);
    }
    await new Promise((resolve2) => setTimeout(resolve2, 500));
  }
}
async function getResultMaybe(jobId) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await JobService.getCompletedJobResultMaybe({ workspace, id: jobId });
}
var STRIP_COMMENTS =
  /(\/\/.*$)|(\/\*[\s\S]*?\*\/)|(\s*=[^,\)]*(('(?:\\'|[^'\r\n])*')|("(?:\\"|[^"\r\n])*"))|(\s*=[^,\)]*))/gm;
var ARGUMENT_NAMES = /([^\s,]+)/g;
function getParamNames(func) {
  const fnStr = func.toString().replace(STRIP_COMMENTS, "");
  let result = fnStr
    .slice(fnStr.indexOf("(") + 1, fnStr.indexOf(")"))
    .match(ARGUMENT_NAMES);
  if (result === null) result = [];
  return result;
}
function task(f) {
  !clientSet && setClient();
  return async (...y) => {
    const args = {};
    const paramNames = getParamNames(f);
    y.forEach((x, i) => (args[paramNames[i]] = x));
    let req = await fetch(
      `${OpenAPI.BASE}/w/${getWorkspace()}/jobs/run/workflow_as_code/${getEnv(
        "WM_JOB_ID"
      )}/${f.name}`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${getEnv("WM_TOKEN")}`,
        },
        body: JSON.stringify({ args }),
      }
    );
    let jobId = await req.text();
    console.log(`Started task ${f.name} as job ${jobId}`);
    let r = await waitJob(jobId);
    console.log(`Task ${f.name} (${jobId}) completed`);
    return r;
  };
}
async function runScriptAsync(path, hash_, args, scheduledInSeconds = null) {
  !clientSet && setClient();
  if (path && hash_) {
    throw new Error("path and hash_ are mutually exclusive");
  }
  args = args || {};
  const params = {};
  if (scheduledInSeconds) {
    params["scheduled_in_secs"] = scheduledInSeconds;
  }
  let parentJobId = getEnv("WM_JOB_ID");
  if (parentJobId !== void 0) {
    params["parent_job"] = parentJobId;
  }
  let rootJobId = getEnv("WM_ROOT_FLOW_JOB_ID");
  if (rootJobId != void 0 && rootJobId != "") {
    params["root_job"] = rootJobId;
  }
  let endpoint;
  if (path) {
    endpoint = `/w/${getWorkspace()}/jobs/run/p/${path}`;
  } else if (hash_) {
    endpoint = `/w/${getWorkspace()}/jobs/run/h/${hash_}`;
  } else {
    throw new Error("path or hash_ must be provided");
  }
  let url = new URL(OpenAPI.BASE + endpoint);
  url.search = new URLSearchParams(params).toString();
  return fetch(url, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${OpenAPI.TOKEN}`,
    },
    body: JSON.stringify(args),
  }).then((res) => res.text());
}
function getStatePath() {
  const state_path = getEnv("WM_STATE_PATH_NEW") ?? getEnv("WM_STATE_PATH");
  if (state_path === void 0) {
    throw Error("State path not set");
  }
  return state_path;
}
async function setResource(value, path, initializeToTypeIfNotExist) {
  !clientSet && setClient();
  path = path ?? getStatePath();
  const workspace = getWorkspace();
  if (await ResourceService.existsResource({ workspace, path })) {
    await ResourceService.updateResourceValue({
      workspace,
      path,
      requestBody: { value },
    });
  } else if (initializeToTypeIfNotExist) {
    await ResourceService.createResource({
      workspace,
      requestBody: { path, value, resource_type: initializeToTypeIfNotExist },
    });
  } else {
    throw Error(
      `Resource at path ${path} does not exist and no type was provided to initialize it`
    );
  }
}
async function setState(state) {
  await setResource(state, void 0, "state");
}
async function setFlowUserState(key, value, errorIfNotPossible) {
  !clientSet && setClient();
  if (value === void 0) {
    value = null;
  }
  const workspace = getWorkspace();
  try {
    await JobService.setFlowUserState({
      workspace,
      id: await getRootJobId(),
      key,
      requestBody: value,
    });
  } catch (e) {
    if (errorIfNotPossible) {
      throw Error(`Error setting flow user state at ${key}: ${e.body}`);
    } else {
      console.error(`Error setting flow user state at ${key}: ${e.body}`);
    }
  }
}
async function getFlowUserState(key, errorIfNotPossible) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  try {
    return await JobService.getFlowUserState({
      workspace,
      id: await getRootJobId(),
      key,
    });
  } catch (e) {
    if (errorIfNotPossible) {
      throw Error(`Error setting flow user state at ${key}: ${e.body}`);
    } else {
      console.error(`Error setting flow user state at ${key}: ${e.body}`);
    }
  }
}
async function getState() {
  return await getResource(getStatePath(), true);
}
async function getVariable(path) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  try {
    return await VariableService.getVariableValue({ workspace, path });
  } catch (e) {
    throw Error(
      `Variable not found at ${path} or not visible to you: ${e.body} ${e}`
    );
  }
}
async function setVariable(
  path,
  value,
  isSecretIfNotExist,
  descriptionIfNotExist
) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  if (await VariableService.existsVariable({ workspace, path })) {
    await VariableService.updateVariable({
      workspace,
      path,
      requestBody: { value },
    });
  } else {
    await VariableService.createVariable({
      workspace,
      requestBody: {
        path,
        value,
        is_secret: isSecretIfNotExist ?? false,
        description: descriptionIfNotExist ?? "",
      },
    });
  }
}
async function denoS3LightClientSettings(s3_resource_path) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  const s3Resource = await HelpersService.s3ResourceInfo({
    workspace,
    requestBody: {
      s3_resource_path,
    },
  });
  let settings = {
    ...s3Resource,
  };
  return settings;
}
async function loadS3File(s3object, s3ResourcePath = void 0) {
  !clientSet && setClient();
  const fileContentBlob = await loadS3FileStream(s3object, s3ResourcePath);
  if (fileContentBlob === void 0) {
    return void 0;
  }
  const reader = fileContentBlob.stream().getReader();
  const chunks = [];
  while (true) {
    const { value: chunk, done } = await reader.read();
    if (done) {
      break;
    }
    chunks.push(chunk);
  }
  let fileContentLength = 0;
  chunks.forEach((item) => {
    fileContentLength += item.length;
  });
  let fileContent = new Uint8Array(fileContentLength);
  let offset = 0;
  chunks.forEach((chunk) => {
    fileContent.set(chunk, offset);
    offset += chunk.length;
  });
  return fileContent;
}
async function loadS3FileStream(s3object, s3ResourcePath = void 0) {
  !clientSet && setClient();
  let params = {};
  params["file_key"] = s3object.s3;
  if (s3ResourcePath !== void 0) {
    params["s3_resource_path"] = s3ResourcePath;
  }
  const queryParams = new URLSearchParams(params);
  const fileContentBlob = await fetch(
    `${
      OpenAPI.BASE
    }/w/${getWorkspace()}/job_helpers/download_s3_file?${queryParams}`,
    {
      method: "GET",
      headers: {
        Authorization: `Bearer ${OpenAPI.TOKEN}`,
      },
    }
  );
  return fileContentBlob.blob();
}
async function writeS3File(s3object, fileContent, s3ResourcePath = void 0) {
  !clientSet && setClient();
  let fileContentBlob;
  if (typeof fileContent === "string") {
    fileContentBlob = new Blob([fileContent], {
      type: "text/plain",
    });
  } else {
    fileContentBlob = fileContent;
  }
  const response = await HelpersService.fileUpload({
    workspace: getWorkspace(),
    fileKey: s3object?.s3,
    fileExtension: void 0,
    s3ResourcePath,
    requestBody: fileContentBlob,
  });
  return {
    s3: response.file_key,
  };
}
async function getResumeUrls(approver) {
  const nonce = Math.floor(Math.random() * 4294967295);
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await JobService.getResumeUrls({
    workspace,
    resumeId: nonce,
    approver,
    id: getEnv("WM_JOB_ID") ?? "NO_JOB_ID",
  });
}
async function getIdToken(audience) {
  !clientSet && setClient();
  const workspace = getWorkspace();
  return await OidcService.getOidcToken({
    workspace,
    audience,
  });
}
export {
  $AppHistory,
  $AppWithLastVersion,
  $AppWithLastVersionWDraft,
  $AuditLog,
  $BranchAll,
  $BranchOne,
  $CompletedJob,
  $ConcurrencyGroup,
  $ContextualVariable,
  $CreateInput,
  $CreateResource,
  $CreateVariable,
  $CreateWorkspace,
  $EditResource,
  $EditResourceType,
  $EditSchedule,
  $EditVariable,
  $EditWorkspaceUser,
  $ExtraPerms,
  $Flow,
  $FlowMetadata,
  $FlowModule,
  $FlowModuleValue,
  $FlowPreview,
  $FlowStatus,
  $FlowStatusModule,
  $FlowValue,
  $Folder,
  $ForloopFlow,
  $GitRepositorySettings,
  $GlobalUserInfo,
  $Group,
  $HubScriptKind,
  $Identity,
  $Input,
  $InputTransform,
  $InstanceGroup,
  $JavascriptTransform,
  $Job,
  $LargeFileStorage,
  $ListableApp,
  $ListableRawApp,
  $ListableResource,
  $ListableVariable,
  $Login,
  $MainArgSignature,
  $MetricDataPoint,
  $MetricMetadata,
  $NewSchedule,
  $NewScript,
  $NewScriptWithDraft,
  $NewToken,
  $NewTokenImpersonate,
  $OpenFlow,
  $OpenFlowWPath,
  $PathFlow,
  $PathScript,
  $PolarsClientKwargs,
  $Policy,
  $Preview,
  $QueuedJob,
  $RawScript,
  $RawScriptForDependencies,
  $Resource,
  $ResourceType,
  $RestartedFrom,
  $Retry,
  $RunnableType,
  $S3Resource,
  $ScalarMetric,
  $Schedule,
  $ScheduleWJobs,
  $Script,
  $ScriptArgs,
  $ScriptHistory,
  $SlackToken,
  $StaticTransform,
  $TimeseriesMetric,
  $TokenResponse,
  $TruncatedToken,
  $UpdateInput,
  $UploadFilePart,
  $User,
  $UserUsage,
  $UserWorkspaceList,
  $WhileloopFlow,
  $WindmillFileMetadata,
  $WindmillFilePreview,
  $WindmillLargeFile,
  $WorkerPing,
  $WorkflowStatus,
  $WorkflowStatusRecord,
  $WorkflowTask,
  $Workspace,
  $WorkspaceDefaultScripts,
  $WorkspaceGitSyncSettings,
  $WorkspaceInvite,
  AdminService,
  ApiError,
  AppService,
  AuditService,
  CancelError,
  CancelablePromise,
  CaptureService,
  ConcurrencyGroupsService,
  ConfigService,
  DraftService,
  FavoriteService,
  FlowService,
  FolderService,
  GranularAclService,
  GroupService,
  HelpersService,
  InputService,
  IntegrationService,
  JobService,
  MetricsService,
  OauthService,
  OidcService,
  OpenAPI,
  RawAppService,
  ResourceService,
  ScheduleService,
  ScriptService,
  SettingService,
  SettingsService,
  UserService,
  VariableService,
  WorkerService,
  WorkspaceService,
  denoS3LightClientSettings,
  getFlowUserState,
  getIdToken,
  getResource,
  getResumeUrls,
  getRootJobId,
  getState,
  getVariable,
  loadS3File,
  loadS3FileStream,
  runScript,
  runScriptAsync,
  setClient,
  setFlowUserState,
  setResource,
  setState,
  setVariable,
  task,
  waitJob,
  writeS3File,
};
