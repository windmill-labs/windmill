export type Script = {
    workspace_id?: string;
    hash: string;
    path: string;
    /**
     * The first element is the direct parent of the script, the second is the parent of the first, etc
     *
     */
    parent_hashes?: Array<(string)>;
    summary: string;
    description: string;
    content: string;
    created_by: string;
    created_at: string;
    archived: boolean;
    schema?: {
        [key: string]: unknown;
    };
    deleted: boolean;
    is_template: boolean;
    extra_perms: {
        [key: string]: (boolean);
    };
    lock?: string;
    lock_error_logs?: string;
    language: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
    kind: 'script' | 'failure' | 'trigger' | 'command' | 'approval';
    starred: boolean;
    tag?: string;
    has_draft?: boolean;
    draft_only?: boolean;
    envs?: Array<(string)>;
    concurrent_limit?: number;
    concurrency_time_window_s?: number;
    cache_ttl?: number;
    dedicated_worker?: boolean;
    ws_error_handler_muted?: boolean;
    priority?: number;
    restart_unless_cancelled?: boolean;
    timeout?: number;
    delete_after_use?: boolean;
    visible_to_runner_only?: boolean;
    no_main_func: boolean;
    codebase?: string;
};
export type NewScript = {
    path: string;
    parent_hash?: string;
    summary: string;
    description: string;
    content: string;
    schema?: {
        [key: string]: unknown;
    };
    is_template?: boolean;
    lock?: string;
    language: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
    kind?: 'script' | 'failure' | 'trigger' | 'command' | 'approval';
    tag?: string;
    draft_only?: boolean;
    envs?: Array<(string)>;
    concurrent_limit?: number;
    concurrency_time_window_s?: number;
    cache_ttl?: number;
    dedicated_worker?: boolean;
    ws_error_handler_muted?: boolean;
    priority?: number;
    restart_unless_cancelled?: boolean;
    timeout?: number;
    delete_after_use?: boolean;
    deployment_message?: string;
    concurrency_key?: string;
    visible_to_runner_only?: boolean;
    no_main_func?: boolean;
    codebase?: string;
};
export type NewScriptWithDraft = NewScript & {
    draft?: NewScript;
    hash: string;
};
export type ScriptHistory = {
    script_hash: string;
    deployment_msg?: string;
};
export type ScriptArgs = {
    [key: string]: unknown;
};
export type Input = {
    id: string;
    name: string;
    args: {
        [key: string]: unknown;
    };
    created_by: string;
    created_at: string;
    is_public: boolean;
    success?: boolean;
};
export type CreateInput = {
    name: string;
    args: {
        [key: string]: unknown;
    };
};
export type UpdateInput = {
    id: string;
    name: string;
    is_public: boolean;
};
export type RunnableType = 'ScriptHash' | 'ScriptPath' | 'FlowPath';
export type QueuedJob = {
    workspace_id?: string;
    id: string;
    parent_job?: string;
    created_by?: string;
    created_at?: string;
    started_at?: string;
    scheduled_for?: string;
    running: boolean;
    script_path?: string;
    script_hash?: string;
    args?: ScriptArgs;
    logs?: string;
    raw_code?: string;
    canceled: boolean;
    canceled_by?: string;
    canceled_reason?: string;
    last_ping?: string;
    job_kind: 'script' | 'preview' | 'dependencies' | 'flowdependencies' | 'appdependencies' | 'flow' | 'flowpreview' | 'script_hub' | 'identity' | 'deploymentcallback' | 'singlescriptflow';
    schedule_path?: string;
    /**
     * The user (u/userfoo) or group (g/groupfoo) whom
     * the execution of this script will be permissioned_as and by extension its DT_TOKEN.
     *
     */
    permissioned_as: string;
    flow_status?: FlowStatus;
    raw_flow?: FlowValue;
    is_flow_step: boolean;
    language?: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
    email: string;
    visible_to_owner: boolean;
    mem_peak?: number;
    tag: string;
    priority?: number;
};
export type CompletedJob = {
    workspace_id?: string;
    id: string;
    parent_job?: string;
    created_by: string;
    created_at: string;
    started_at: string;
    duration_ms: number;
    success: boolean;
    script_path?: string;
    script_hash?: string;
    args?: ScriptArgs;
    result?: unknown;
    logs?: string;
    deleted?: boolean;
    raw_code?: string;
    canceled: boolean;
    canceled_by?: string;
    canceled_reason?: string;
    job_kind: 'script' | 'preview' | 'dependencies' | 'flow' | 'flowdependencies' | 'appdependencies' | 'flowpreview' | 'script_hub' | 'identity' | 'deploymentcallback' | 'singlescriptflow';
    schedule_path?: string;
    /**
     * The user (u/userfoo) or group (g/groupfoo) whom
     * the execution of this script will be permissioned_as and by extension its DT_TOKEN.
     *
     */
    permissioned_as: string;
    flow_status?: FlowStatus;
    raw_flow?: FlowValue;
    is_flow_step: boolean;
    language?: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
    is_skipped: boolean;
    email: string;
    visible_to_owner: boolean;
    mem_peak?: number;
    tag: string;
    priority?: number;
    labels?: Array<(string)>;
};
export type Job = CompletedJob & {
    type?: 'CompletedJob';
} | QueuedJob & {
    type?: 'QueuedJob';
};
export type User = {
    email: string;
    username: string;
    is_admin: boolean;
    is_super_admin: boolean;
    created_at: string;
    operator: boolean;
    disabled: boolean;
    groups?: Array<(string)>;
    folders: Array<(string)>;
    folders_owners: Array<(string)>;
};
export type UserUsage = {
    email?: string;
    executions?: number;
};
export type Login = {
    email: string;
    password: string;
};
export type EditWorkspaceUser = {
    is_admin?: boolean;
    operator?: boolean;
    disabled?: boolean;
};
export type TruncatedToken = {
    label?: string;
    expiration?: string;
    token_prefix: string;
    created_at: string;
    last_used_at: string;
    scopes?: Array<(string)>;
};
export type NewToken = {
    label?: string;
    expiration?: string;
    scopes?: Array<(string)>;
};
export type NewTokenImpersonate = {
    label?: string;
    expiration?: string;
    impersonate_email: string;
};
export type ListableVariable = {
    workspace_id: string;
    path: string;
    value?: string;
    is_secret: boolean;
    description?: string;
    account?: number;
    is_oauth?: boolean;
    extra_perms: {
        [key: string]: (boolean);
    };
    is_expired?: boolean;
    refresh_error?: string;
    is_linked?: boolean;
    is_refreshed?: boolean;
};
export type ContextualVariable = {
    name: string;
    value: string;
    description: string;
    is_custom: boolean;
};
export type CreateVariable = {
    path: string;
    value: string;
    is_secret: boolean;
    description: string;
    account?: number;
    is_oauth?: boolean;
};
export type EditVariable = {
    path?: string;
    value?: string;
    is_secret?: boolean;
    description?: string;
};
export type AuditLog = {
    id: number;
    timestamp: string;
    username: string;
    operation: 'jobs.run' | 'jobs.run.script' | 'jobs.run.preview' | 'jobs.run.flow' | 'jobs.run.flow_preview' | 'jobs.run.script_hub' | 'jobs.run.dependencies' | 'jobs.run.identity' | 'jobs.run.noop' | 'jobs.flow_dependencies' | 'jobs' | 'jobs.cancel' | 'jobs.force_cancel' | 'jobs.disapproval' | 'jobs.delete' | 'account.delete' | 'openai.request' | 'resources.create' | 'resources.update' | 'resources.delete' | 'resource_types.create' | 'resource_types.update' | 'resource_types.delete' | 'schedule.create' | 'schedule.setenabled' | 'schedule.edit' | 'schedule.delete' | 'scripts.create' | 'scripts.update' | 'scripts.archive' | 'scripts.delete' | 'users.create' | 'users.delete' | 'users.update' | 'users.login' | 'users.logout' | 'users.accept_invite' | 'users.decline_invite' | 'users.token.create' | 'users.token.delete' | 'users.add_to_workspace' | 'users.add_global' | 'users.setpassword' | 'users.impersonate' | 'users.leave_workspace' | 'oauth.login' | 'oauth.signup' | 'variables.create' | 'variables.delete' | 'variables.update' | 'flows.create' | 'flows.update' | 'flows.delete' | 'flows.archive' | 'apps.create' | 'apps.update' | 'apps.delete' | 'folder.create' | 'folder.update' | 'folder.delete' | 'folder.add_owner' | 'folder.remove_owner' | 'group.create' | 'group.delete' | 'group.edit' | 'group.adduser' | 'group.removeuser' | 'igroup.create' | 'igroup.delete' | 'igroup.adduser' | 'igroup.removeuser' | 'variables.decrypt_secret' | 'workspaces.edit_command_script' | 'workspaces.edit_deploy_to' | 'workspaces.edit_auto_invite_domain' | 'workspaces.edit_webhook' | 'workspaces.edit_copilot_config' | 'workspaces.edit_error_handler' | 'workspaces.create' | 'workspaces.update' | 'workspaces.archive' | 'workspaces.unarchive' | 'workspaces.delete';
    action_kind: 'Created' | 'Updated' | 'Delete' | 'Execute';
    resource?: string;
    parameters?: {
        [key: string]: unknown;
    };
};
export type MainArgSignature = {
    type: 'Valid' | 'Invalid';
    error: string;
    star_args: boolean;
    star_kwargs?: boolean;
    args: Array<{
        name: string;
        typ: 'float' | 'int' | 'bool' | 'email' | 'unknown' | 'bytes' | 'dict' | 'datetime' | 'sql' | {
            resource: string | null;
        } | {
            str: Array<(string)> | null;
        } | {
            object: Array<{
                key: string;
                typ: 'float' | 'int' | 'bool' | 'email' | 'unknown' | 'bytes' | 'dict' | 'datetime' | 'sql' | {
                    str: unknown;
                };
            }>;
        } | {
            list: 'float' | 'int' | 'bool' | 'email' | 'unknown' | 'bytes' | 'dict' | 'datetime' | 'sql' | {
                str: unknown;
            } | null;
        };
        has_default?: boolean;
        default?: unknown;
    }>;
};
export type Preview = {
    content?: string;
    path?: string;
    args: ScriptArgs;
    language?: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
    tag?: string;
    kind?: 'code' | 'identity' | 'http';
    dedicated_worker?: boolean;
    lock?: string;
};
export type WorkflowTask = {
    args: ScriptArgs;
};
export type WorkflowStatusRecord = {
    [key: string]: WorkflowStatus;
};
export type WorkflowStatus = {
    scheduled_for?: string;
    started_at?: string;
    duration_ms?: number;
    name?: string;
};
export type CreateResource = {
    path: string;
    value: unknown;
    description?: string;
    resource_type: string;
};
export type EditResource = {
    path?: string;
    description?: string;
    value?: unknown;
};
export type Resource = {
    workspace_id?: string;
    path: string;
    description?: string;
    resource_type: string;
    value?: unknown;
    is_oauth: boolean;
    extra_perms?: {
        [key: string]: (boolean);
    };
};
export type ListableResource = {
    workspace_id?: string;
    path: string;
    description?: string;
    resource_type: string;
    value?: unknown;
    is_oauth: boolean;
    extra_perms?: {
        [key: string]: (boolean);
    };
    is_expired?: boolean;
    refresh_error?: string;
    is_linked: boolean;
    is_refreshed: boolean;
    account?: number;
};
export type ResourceType = {
    workspace_id?: string;
    name: string;
    schema?: unknown;
    description?: string;
};
export type EditResourceType = {
    schema?: unknown;
    description?: string;
};
export type Schedule = {
    path: string;
    edited_by: string;
    edited_at: string;
    schedule: string;
    timezone: string;
    enabled: boolean;
    script_path: string;
    is_flow: boolean;
    args?: ScriptArgs;
    extra_perms: {
        [key: string]: (boolean);
    };
    email: string;
    error?: string;
    on_failure?: string;
    on_failure_times?: number;
    on_failure_exact?: boolean;
    on_failure_extra_args?: ScriptArgs;
    on_recovery?: string;
    on_recovery_times?: number;
    on_recovery_extra_args?: ScriptArgs;
    ws_error_handler_muted?: boolean;
    retry?: Retry;
    summary?: string;
    no_flow_overlap?: boolean;
    tag?: string;
};
export type ScheduleWJobs = Schedule & {
    jobs?: Array<{
        id: string;
        success: boolean;
        duration_ms: number;
    }>;
};
export type NewSchedule = {
    path: string;
    schedule: string;
    timezone: string;
    script_path: string;
    is_flow: boolean;
    args: ScriptArgs;
    enabled?: boolean;
    on_failure?: string;
    on_failure_times?: number;
    on_failure_exact?: boolean;
    on_failure_extra_args?: ScriptArgs;
    on_recovery?: string;
    on_recovery_times?: number;
    on_recovery_extra_args?: ScriptArgs;
    ws_error_handler_muted?: boolean;
    retry?: Retry;
    no_flow_overlap?: boolean;
    summary?: string;
    tag?: string;
};
export type EditSchedule = {
    schedule: string;
    timezone: string;
    args: ScriptArgs;
    on_failure?: string;
    on_failure_times?: number;
    on_failure_exact?: boolean;
    on_failure_extra_args?: ScriptArgs;
    on_recovery?: string;
    on_recovery_times?: number;
    on_recovery_extra_args?: ScriptArgs;
    ws_error_handler_muted?: boolean;
    retry?: Retry;
    no_flow_overlap?: boolean;
    summary?: string;
    tag?: string;
};
export type Group = {
    name: string;
    summary?: string;
    members?: Array<(string)>;
    extra_perms?: {
        [key: string]: (boolean);
    };
};
export type InstanceGroup = {
    name: string;
    summary?: string;
    emails?: Array<(string)>;
};
export type Folder = {
    name: string;
    owners: Array<(string)>;
    extra_perms: {
        [key: string]: (boolean);
    };
};
export type WorkerPing = {
    worker: string;
    worker_instance: string;
    last_ping?: number;
    started_at: string;
    ip: string;
    jobs_executed: number;
    custom_tags?: Array<(string)>;
    worker_group: string;
    wm_version: string;
    current_job_id?: string;
    current_job_workspace_id?: string;
    occupancy_rate?: number;
};
export type UserWorkspaceList = {
    email: string;
    workspaces: Array<{
        id: string;
        name: string;
        username: string;
    }>;
};
export type CreateWorkspace = {
    id: string;
    name: string;
    username?: string;
};
export type Workspace = {
    id: string;
    name: string;
    owner: string;
    domain?: string;
};
export type WorkspaceInvite = {
    workspace_id: string;
    email: string;
    is_admin: boolean;
    operator: boolean;
};
export type GlobalUserInfo = {
    email: string;
    login_type: 'password' | 'github';
    super_admin: boolean;
    verified: boolean;
    name?: string;
    company?: string;
    username?: string;
};
export type Flow = OpenFlow & FlowMetadata;
export type ExtraPerms = {
    [key: string]: (boolean);
};
export type FlowMetadata = {
    workspace_id?: string;
    path: string;
    edited_by: string;
    edited_at: string;
    archived: boolean;
    extra_perms: ExtraPerms;
    starred?: boolean;
    draft_only?: boolean;
    tag?: string;
    ws_error_handler_muted?: boolean;
    priority?: number;
    dedicated_worker?: boolean;
    timeout?: number;
    visible_to_runner_only?: boolean;
};
export type OpenFlowWPath = OpenFlow & {
    path: string;
    tag?: string;
    ws_error_handler_muted?: boolean;
    priority?: number;
    dedicated_worker?: boolean;
    timeout?: number;
    visible_to_runner_only?: boolean;
};
export type FlowPreview = {
    value: FlowValue;
    path?: string;
    args: ScriptArgs;
    tag?: string;
    restarted_from?: RestartedFrom;
};
export type RestartedFrom = {
    flow_job_id?: string;
    step_id?: string;
    branch_or_iteration_n?: number;
};
export type Policy = {
    triggerables?: {
        [key: string]: {
            [key: string]: unknown;
        };
    };
    triggerables_v2?: {
        [key: string]: {
            [key: string]: unknown;
        };
    };
    execution_mode?: 'viewer' | 'publisher' | 'anonymous';
    on_behalf_of?: string;
    on_behalf_of_email?: string;
};
export type ListableApp = {
    id: number;
    workspace_id: string;
    path: string;
    summary: string;
    version: number;
    extra_perms: {
        [key: string]: (boolean);
    };
    starred?: boolean;
    edited_at: string;
    execution_mode: 'viewer' | 'publisher' | 'anonymous';
};
export type ListableRawApp = {
    workspace_id: string;
    path: string;
    summary: string;
    extra_perms: {
        [key: string]: (boolean);
    };
    starred?: boolean;
    version: number;
    edited_at: string;
};
export type AppWithLastVersion = {
    id: number;
    workspace_id: string;
    path: string;
    summary: string;
    versions: Array<(number)>;
    created_by: string;
    created_at: string;
    value: {
        [key: string]: unknown;
    };
    policy: Policy;
    execution_mode: 'viewer' | 'publisher' | 'anonymous';
    extra_perms: {
        [key: string]: (boolean);
    };
};
export type AppWithLastVersionWDraft = AppWithLastVersion & {
    draft_only?: boolean;
    draft?: unknown;
};
export type AppHistory = {
    version: number;
    deployment_msg?: string;
};
export type SlackToken = {
    access_token: string;
    team_id: string;
    team_name: string;
    bot: {
        bot_access_token?: string;
    };
};
export type TokenResponse = {
    access_token: string;
    expires_in?: number;
    refresh_token?: string;
    scope?: Array<(string)>;
};
export type HubScriptKind = unknown;
export type PolarsClientKwargs = {
    region_name: string;
};
export type LargeFileStorage = {
    type?: 'S3Storage' | 'AzureBlobStorage' | 'AzureWorkloadIdentity' | 'S3AwsOidc';
    s3_resource_path?: string;
    azure_blob_resource_path?: string;
    public_resource?: boolean;
};
export type WindmillLargeFile = {
    s3: string;
};
export type WindmillFileMetadata = {
    mime_type?: string;
    size_in_bytes?: number;
    last_modified?: string;
    expires?: string;
    version_id?: string;
};
export type WindmillFilePreview = {
    msg?: string;
    content?: string;
    content_type: 'RawText' | 'Csv' | 'Parquet' | 'Unknown';
};
export type S3Resource = {
    bucket: string;
    region: string;
    endPoint: string;
    useSSL: boolean;
    accessKey?: string;
    secretKey?: string;
    pathStyle: boolean;
};
export type WorkspaceGitSyncSettings = {
    include_path?: Array<(string)>;
    include_type?: Array<('script' | 'flow' | 'app' | 'folder' | 'resource' | 'variable' | 'secret' | 'resourcetype' | 'schedule' | 'user' | 'group')>;
    repositories?: Array<GitRepositorySettings>;
};
export type WorkspaceDefaultScripts = {
    order?: Array<(string)>;
    hidden?: Array<(string)>;
    default_script_content?: unknown;
};
export type GitRepositorySettings = {
    script_path: string;
    git_repo_resource_path: string;
    use_individual_branch?: boolean;
    group_by_folder?: boolean;
    exclude_types_override?: Array<('script' | 'flow' | 'app' | 'folder' | 'resource' | 'variable' | 'secret' | 'resourcetype' | 'schedule' | 'user' | 'group')>;
};
export type UploadFilePart = {
    part_number: number;
    tag: string;
};
export type MetricMetadata = {
    id: string;
    name?: string;
};
export type ScalarMetric = {
    metric_id?: string;
    value: number;
};
export type TimeseriesMetric = {
    metric_id?: string;
    values: Array<MetricDataPoint>;
};
export type MetricDataPoint = {
    timestamp: string;
    value: number;
};
export type RawScriptForDependencies = {
    raw_code: string;
    path: string;
    language: 'python3' | 'deno' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets' | 'bun';
};
export type ConcurrencyGroup = {
    concurrency_id: string;
    job_uuids: Array<(string)>;
};
export type OpenFlow = {
    summary: string;
    description?: string;
    value: FlowValue;
    schema?: {
        [key: string]: unknown;
    };
};
export type FlowValue = {
    modules: Array<FlowModule>;
    failure_module?: FlowModule;
    same_worker?: boolean;
    concurrent_limit?: number;
    concurrency_key?: string;
    concurrency_time_window_s?: number;
    skip_expr?: string;
    cache_ttl?: number;
    priority?: number;
    early_return?: string;
};
export type Retry = {
    constant?: {
        attempts?: number;
        seconds?: number;
    };
    exponential?: {
        attempts?: number;
        multiplier?: number;
        seconds?: number;
        random_factor?: number;
    };
};
export type FlowModule = {
    id: string;
    value: FlowModuleValue;
    stop_after_if?: {
        skip_if_stopped?: boolean;
        expr: string;
    };
    sleep?: InputTransform;
    cache_ttl?: number;
    timeout?: number;
    delete_after_use?: boolean;
    summary?: string;
    mock?: {
        enabled?: boolean;
        return_value?: unknown;
    };
    suspend?: {
        required_events?: number;
        timeout?: number;
        resume_form?: {
            schema?: {
                [key: string]: unknown;
            };
        };
        user_auth_required?: boolean;
        user_groups_required?: InputTransform;
        self_approval_disabled?: boolean;
        hide_cancel?: boolean;
    };
    priority?: number;
    continue_on_error?: boolean;
    retry?: Retry;
};
export type InputTransform = StaticTransform | JavascriptTransform;
export type StaticTransform = {
    value?: unknown;
    type: 'static';
};
export type JavascriptTransform = {
    expr: string;
    type: 'javascript';
};
export type FlowModuleValue = RawScript | PathScript | PathFlow | ForloopFlow | WhileloopFlow | BranchOne | BranchAll | Identity;
export type RawScript = {
    input_transforms: {
        [key: string]: InputTransform;
    };
    content: string;
    language: 'deno' | 'bun' | 'python3' | 'go' | 'bash' | 'powershell' | 'postgresql' | 'mysql' | 'bigquery' | 'snowflake' | 'mssql' | 'graphql' | 'nativets';
    path?: string;
    lock?: string;
    type: 'rawscript';
    tag?: string;
    concurrent_limit?: number;
    concurrency_time_window_s?: number;
};
export type PathScript = {
    input_transforms: {
        [key: string]: InputTransform;
    };
    path: string;
    hash?: string;
    type: 'script';
};
export type PathFlow = {
    input_transforms: {
        [key: string]: InputTransform;
    };
    path: string;
    type: 'flow';
};
export type ForloopFlow = {
    modules: Array<FlowModule>;
    iterator: InputTransform;
    skip_failures: boolean;
    type: 'forloopflow';
    parallel?: boolean;
    parallelism?: number;
};
export type WhileloopFlow = {
    modules: Array<FlowModule>;
    skip_failures: boolean;
    type: 'whileloopflow';
    parallel?: boolean;
    parallelism?: number;
};
export type BranchOne = {
    branches: Array<{
        summary?: string;
        expr: string;
        modules: Array<FlowModule>;
    }>;
    default: Array<FlowModule>;
    type: 'branchone';
};
export type BranchAll = {
    branches: Array<{
        summary?: string;
        skip_failure?: boolean;
        modules: Array<FlowModule>;
    }>;
    type: 'branchall';
    parallel?: boolean;
};
export type Identity = {
    type: 'identity';
    flow?: boolean;
};
export type FlowStatus = {
    step: number;
    modules: Array<FlowStatusModule>;
    user_states?: unknown;
    failure_module: FlowStatusModule & {
        parent_module?: string;
    };
    retry?: {
        fail_count?: number;
        failed_jobs?: Array<(string)>;
    };
};
export type FlowStatusModule = {
    type: 'WaitingForPriorSteps' | 'WaitingForEvents' | 'WaitingForExecutor' | 'InProgress' | 'Success' | 'Failure';
    id?: string;
    job?: string;
    count?: number;
    iterator?: {
        index?: number;
        itered?: Array<unknown>;
        args?: unknown;
    };
    flow_jobs?: Array<(string)>;
    branch_chosen?: {
        type: 'branch' | 'default';
        branch?: number;
    };
    branchall?: {
        branch: number;
        len: number;
    };
    approvers?: Array<{
        resume_id: number;
        approver: string;
    }>;
};
export type ParameterKey = string;
export type ParameterWorkspaceId = string;
export type ParameterVersionId = number;
export type ParameterToken = string;
export type ParameterAccountId = number;
export type ParameterClientName = string;
export type ParameterScriptPath = string;
export type ParameterScriptHash = string;
export type ParameterJobId = string;
export type ParameterPath = string;
export type ParameterPathId = number;
export type ParameterPathVersion = number;
export type ParameterName = string;
/**
 * which page to return (start at 1, default 1)
 */
export type ParameterPage = number;
/**
 * number of items to return for a given page (default 30, max 100)
 */
export type ParameterPerPage = number;
/**
 * order by desc order (default true)
 */
export type ParameterOrderDesc = boolean;
/**
 * mask to filter exact matching user creator
 */
export type ParameterCreatedBy = string;
/**
 * mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
 */
export type ParameterLabel = string;
/**
 * The parent job that is at the origin and responsible for the execution of this script if any
 */
export type ParameterParentJob = string;
/**
 * Override the tag to use
 */
export type ParameterWorkerTag = string;
/**
 * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
 */
export type ParameterCacheTtl = string;
/**
 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
 */
export type ParameterNewJobId = string;
/**
 * List of headers's keys (separated with ',') whove value are added to the args
 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
 *
 */
export type ParameterIncludeHeader = string;
/**
 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
 *
 */
export type ParameterQueueLimit = string;
/**
 * The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
 * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
 *
 */
export type ParameterPayload = string;
/**
 * mask to filter matching starting path
 */
export type ParameterScriptStartPath = string;
/**
 * mask to filter by schedule path
 */
export type ParameterSchedulePath = string;
/**
 * mask to filter exact matching path
 */
export type ParameterScriptExactPath = string;
/**
 * mask to filter exact matching path
 */
export type ParameterScriptExactHash = string;
/**
 * filter on started before (inclusive) timestamp
 */
export type ParameterStartedBefore = string;
/**
 * filter on started after (exclusive) timestamp
 */
export type ParameterStartedAfter = string;
/**
 * filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
 */
export type ParameterCreatedOrStartedAfter = string;
/**
 * filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
 */
export type ParameterCreatedOrStartedBefore = string;
/**
 * filter on successful jobs
 */
export type ParameterSuccess = boolean;
/**
 * filter on jobs scheduled_for before now (hence waitinf for a worker)
 */
export type ParameterScheduledForBeforeNow = boolean;
/**
 * filter on suspended jobs
 */
export type ParameterSuspended = boolean;
/**
 * filter on running jobs
 */
export type ParameterRunning = boolean;
/**
 * filter on jobs containing those args as a json subset (@> in postgres)
 */
export type ParameterArgsFilter = string;
/**
 * filter on jobs with a given tag/worker group
 */
export type ParameterTag = string;
/**
 * filter on jobs containing those result as a json subset (@> in postgres)
 */
export type ParameterResultFilter = string;
/**
 * filter on created after (exclusive) timestamp
 */
export type ParameterAfter = string;
/**
 * filter on created before (exclusive) timestamp
 */
export type ParameterBefore = string;
/**
 * filter on exact username of user
 */
export type ParameterUsername = string;
/**
 * filter on exact or prefix name of operation
 */
export type ParameterOperation = string;
/**
 * filter on exact or prefix name of resource
 */
export type ParameterResourceName = string;
/**
 * filter on type of operation
 */
export type ParameterActionKind = 'Create' | 'Update' | 'Delete' | 'Execute';
/**
 * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
 */
export type ParameterJobKinds = string;
export type ParameterRunnableId = string;
export type ParameterRunnableTypeQuery = RunnableType;
export type ParameterInputId = string;
export type ParameterGetStarted = boolean;
export type ParameterConcurrencyId = string;
export type BackendVersionResponse = string;
export type BackendUptodateResponse = string;
export type GetLicenseIdResponse = string;
export type GetOpenApiYamlResponse = string;
export type GetAuditLogData = {
    id: number;
    workspace: string;
};
export type GetAuditLogResponse = AuditLog;
export type ListAuditLogsData = {
    /**
     * filter on type of operation
     */
    actionKind?: 'Create' | 'Update' | 'Delete' | 'Execute';
    /**
     * filter on created after (exclusive) timestamp
     */
    after?: string;
    /**
     * filter on created before (exclusive) timestamp
     */
    before?: string;
    /**
     * filter on exact or prefix name of operation
     */
    operation?: string;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * filter on exact or prefix name of resource
     */
    resource?: string;
    /**
     * filter on exact username of user
     */
    username?: string;
    workspace: string;
};
export type ListAuditLogsResponse = Array<AuditLog>;
export type LoginData = {
    /**
     * credentials
     */
    requestBody: Login;
};
export type LoginResponse = string;
export type LogoutResponse = string;
export type GetUserData = {
    username: string;
    workspace: string;
};
export type GetUserResponse = User;
export type UpdateUserData = {
    /**
     * new user
     */
    requestBody: EditWorkspaceUser;
    username: string;
    workspace: string;
};
export type UpdateUserResponse = string;
export type IsOwnerOfPathData = {
    path: string;
    workspace: string;
};
export type IsOwnerOfPathResponse = boolean;
export type SetPasswordData = {
    /**
     * set password
     */
    requestBody: {
        password: string;
    };
};
export type SetPasswordResponse = string;
export type CreateUserGloballyData = {
    /**
     * user info
     */
    requestBody: {
        email: string;
        password: string;
        super_admin: boolean;
        name?: string;
        company?: string;
    };
};
export type CreateUserGloballyResponse = string;
export type GlobalUserUpdateData = {
    email: string;
    /**
     * new user info
     */
    requestBody: {
        is_super_admin?: boolean;
    };
};
export type GlobalUserUpdateResponse = string;
export type GlobalUsernameInfoData = {
    email: string;
};
export type GlobalUsernameInfoResponse = {
    username: string;
    workspace_usernames: Array<{
        workspace_id: string;
        username: string;
    }>;
};
export type GlobalUserRenameData = {
    email: string;
    /**
     * new username
     */
    requestBody: {
        new_username: string;
    };
};
export type GlobalUserRenameResponse = string;
export type GlobalUserDeleteData = {
    email: string;
};
export type GlobalUserDeleteResponse = string;
export type DeleteUserData = {
    username: string;
    workspace: string;
};
export type DeleteUserResponse = string;
export type GetCurrentEmailResponse = string;
export type RefreshUserTokenResponse = string;
export type GetTutorialProgressResponse = {
    progress?: number;
};
export type UpdateTutorialProgressData = {
    /**
     * progress update
     */
    requestBody: {
        progress?: number;
    };
};
export type UpdateTutorialProgressResponse = string;
export type LeaveInstanceResponse = string;
export type GetUsageResponse = number;
export type GetRunnableResponse = {
    workspace: string;
    endpoint_async: string;
    endpoint_sync: string;
    endpoint_openai_sync: string;
    summary: string;
    description?: string;
    kind: string;
};
export type GlobalWhoamiResponse = GlobalUserInfo;
export type ListWorkspaceInvitesResponse = Array<WorkspaceInvite>;
export type WhoamiData = {
    workspace: string;
};
export type WhoamiResponse = User;
export type AcceptInviteData = {
    /**
     * accept invite
     */
    requestBody: {
        workspace_id: string;
        username?: string;
    };
};
export type AcceptInviteResponse = string;
export type DeclineInviteData = {
    /**
     * decline invite
     */
    requestBody: {
        workspace_id: string;
    };
};
export type DeclineInviteResponse = string;
export type WhoisData = {
    username: string;
    workspace: string;
};
export type WhoisResponse = User;
export type ExistsEmailData = {
    email: string;
};
export type ExistsEmailResponse = boolean;
export type ListUsersAsSuperAdminData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
};
export type ListUsersAsSuperAdminResponse = Array<GlobalUserInfo>;
export type ListUsersData = {
    workspace: string;
};
export type ListUsersResponse = Array<User>;
export type ListUsersUsageData = {
    workspace: string;
};
export type ListUsersUsageResponse = Array<UserUsage>;
export type ListUsernamesData = {
    workspace: string;
};
export type ListUsernamesResponse = Array<(string)>;
export type CreateTokenData = {
    /**
     * new token
     */
    requestBody: NewToken;
};
export type CreateTokenResponse = string;
export type CreateTokenImpersonateData = {
    /**
     * new token
     */
    requestBody: NewTokenImpersonate;
};
export type CreateTokenImpersonateResponse = string;
export type DeleteTokenData = {
    tokenPrefix: string;
};
export type DeleteTokenResponse = string;
export type ListTokensData = {
    excludeEphemeral?: boolean;
};
export type ListTokensResponse = Array<TruncatedToken>;
export type LoginWithOauthData = {
    clientName: string;
    /**
     * Partially filled script
     */
    requestBody: {
        code?: string;
        state?: string;
    };
};
export type LoginWithOauthResponse = string;
export type ListWorkspacesResponse = Array<Workspace>;
export type IsDomainAllowedResponse = boolean;
export type ListUserWorkspacesResponse = UserWorkspaceList;
export type ListWorkspacesAsSuperAdminData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
};
export type ListWorkspacesAsSuperAdminResponse = Array<Workspace>;
export type CreateWorkspaceData = {
    /**
     * new token
     */
    requestBody: CreateWorkspace;
};
export type CreateWorkspaceResponse = string;
export type ExistsWorkspaceData = {
    /**
     * id of workspace
     */
    requestBody: {
        id: string;
    };
};
export type ExistsWorkspaceResponse = boolean;
export type ExistsUsernameData = {
    requestBody: {
        id: string;
        username: string;
    };
};
export type ExistsUsernameResponse = boolean;
export type InviteUserData = {
    /**
     * WorkspaceInvite
     */
    requestBody: {
        email: string;
        is_admin: boolean;
        operator: boolean;
    };
    workspace: string;
};
export type InviteUserResponse = string;
export type AddUserData = {
    /**
     * WorkspaceInvite
     */
    requestBody: {
        email: string;
        is_admin: boolean;
        username?: string;
        operator: boolean;
    };
    workspace: string;
};
export type AddUserResponse = string;
export type DeleteInviteData = {
    /**
     * WorkspaceInvite
     */
    requestBody: {
        email: string;
        is_admin: boolean;
        operator: boolean;
    };
    workspace: string;
};
export type DeleteInviteResponse = string;
export type ArchiveWorkspaceData = {
    workspace: string;
};
export type ArchiveWorkspaceResponse = string;
export type UnarchiveWorkspaceData = {
    workspace: string;
};
export type UnarchiveWorkspaceResponse = string;
export type DeleteWorkspaceData = {
    workspace: string;
};
export type DeleteWorkspaceResponse = string;
export type LeaveWorkspaceData = {
    workspace: string;
};
export type LeaveWorkspaceResponse = string;
export type GetWorkspaceNameData = {
    workspace: string;
};
export type GetWorkspaceNameResponse = string;
export type ChangeWorkspaceNameData = {
    requestBody?: {
        new_name?: string;
    };
    workspace: string;
};
export type ChangeWorkspaceNameResponse = string;
export type ChangeWorkspaceIdData = {
    requestBody?: {
        new_id?: string;
        new_name?: string;
    };
    workspace: string;
};
export type ChangeWorkspaceIdResponse = string;
export type ListPendingInvitesData = {
    workspace: string;
};
export type ListPendingInvitesResponse = Array<WorkspaceInvite>;
export type GetSettingsData = {
    workspace: string;
};
export type GetSettingsResponse = {
    workspace_id?: string;
    slack_name?: string;
    slack_team_id?: string;
    slack_command_script?: string;
    auto_invite_domain?: string;
    auto_invite_operator?: boolean;
    auto_add?: boolean;
    plan?: string;
    automatic_billing: boolean;
    customer_id?: string;
    webhook?: string;
    deploy_to?: string;
    openai_resource_path?: string;
    code_completion_enabled: boolean;
    error_handler?: string;
    error_handler_extra_args?: ScriptArgs;
    error_handler_muted_on_cancel: boolean;
    large_file_storage?: LargeFileStorage;
    git_sync?: WorkspaceGitSyncSettings;
    default_app?: string;
    default_scripts?: WorkspaceDefaultScripts;
};
export type GetDeployToData = {
    workspace: string;
};
export type GetDeployToResponse = {
    deploy_to?: string;
};
export type GetIsPremiumData = {
    workspace: string;
};
export type GetIsPremiumResponse = boolean;
export type GetPremiumInfoData = {
    workspace: string;
};
export type GetPremiumInfoResponse = {
    premium: boolean;
    usage?: number;
    seats?: number;
    automatic_billing: boolean;
};
export type SetAutomaticBillingData = {
    /**
     * automatic billing
     */
    requestBody: {
        automatic_billing: boolean;
        seats?: number;
    };
    workspace: string;
};
export type SetAutomaticBillingResponse = string;
export type EditSlackCommandData = {
    /**
     * WorkspaceInvite
     */
    requestBody: {
        slack_command_script?: string;
    };
    workspace: string;
};
export type EditSlackCommandResponse = string;
export type RunSlackMessageTestJobData = {
    /**
     * path to hub script to run and its corresponding args
     */
    requestBody: {
        hub_script_path?: string;
        channel?: string;
        test_msg?: string;
    };
    workspace: string;
};
export type RunSlackMessageTestJobResponse = {
    job_uuid?: string;
};
export type EditDeployToData = {
    requestBody: {
        deploy_to?: string;
    };
    workspace: string;
};
export type EditDeployToResponse = string;
export type EditAutoInviteData = {
    /**
     * WorkspaceInvite
     */
    requestBody: {
        operator?: boolean;
        invite_all?: boolean;
        auto_add?: boolean;
    };
    workspace: string;
};
export type EditAutoInviteResponse = string;
export type EditWebhookData = {
    /**
     * WorkspaceWebhook
     */
    requestBody: {
        webhook?: string;
    };
    workspace: string;
};
export type EditWebhookResponse = string;
export type EditCopilotConfigData = {
    /**
     * WorkspaceCopilotConfig
     */
    requestBody: {
        openai_resource_path?: string;
        code_completion_enabled: boolean;
    };
    workspace: string;
};
export type EditCopilotConfigResponse = string;
export type GetCopilotInfoData = {
    workspace: string;
};
export type GetCopilotInfoResponse = {
    exists_openai_resource_path: boolean;
    code_completion_enabled: boolean;
};
export type EditErrorHandlerData = {
    /**
     * WorkspaceErrorHandler
     */
    requestBody: {
        error_handler?: string;
        error_handler_extra_args?: ScriptArgs;
        error_handler_muted_on_cancel?: boolean;
    };
    workspace: string;
};
export type EditErrorHandlerResponse = string;
export type EditLargeFileStorageConfigData = {
    /**
     * LargeFileStorage info
     */
    requestBody: {
        large_file_storage?: LargeFileStorage;
    };
    workspace: string;
};
export type EditLargeFileStorageConfigResponse = unknown;
export type EditWorkspaceGitSyncConfigData = {
    /**
     * Workspace Git sync settings
     */
    requestBody: {
        git_sync_settings?: WorkspaceGitSyncSettings;
    };
    workspace: string;
};
export type EditWorkspaceGitSyncConfigResponse = unknown;
export type EditWorkspaceDefaultAppData = {
    /**
     * Workspace default app
     */
    requestBody: {
        default_app_path?: string;
    };
    workspace: string;
};
export type EditWorkspaceDefaultAppResponse = string;
export type EditDefaultScriptsData = {
    /**
     * Workspace default app
     */
    requestBody?: WorkspaceDefaultScripts;
    workspace: string;
};
export type EditDefaultScriptsResponse = string;
export type GetDefaultScriptsData = {
    workspace: string;
};
export type GetDefaultScriptsResponse = WorkspaceDefaultScripts;
export type SetEnvironmentVariableData = {
    /**
     * Workspace default app
     */
    requestBody: {
        name: string;
        value?: string;
    };
    workspace: string;
};
export type SetEnvironmentVariableResponse = string;
export type GetWorkspaceEncryptionKeyData = {
    workspace: string;
};
export type GetWorkspaceEncryptionKeyResponse = {
    key: string;
};
export type SetWorkspaceEncryptionKeyData = {
    /**
     * New encryption key
     */
    requestBody: {
        new_key: string;
    };
    workspace: string;
};
export type SetWorkspaceEncryptionKeyResponse = string;
export type GetWorkspaceDefaultAppData = {
    workspace: string;
};
export type GetWorkspaceDefaultAppResponse = {
    default_app_path?: string;
};
export type GetLargeFileStorageConfigData = {
    workspace: string;
};
export type GetLargeFileStorageConfigResponse = LargeFileStorage;
export type GetWorkspaceUsageData = {
    workspace: string;
};
export type GetWorkspaceUsageResponse = number;
export type GetGlobalData = {
    key: string;
};
export type GetGlobalResponse = unknown;
export type SetGlobalData = {
    key: string;
    /**
     * value set
     */
    requestBody: {
        value?: unknown;
    };
};
export type SetGlobalResponse = string;
export type GetLocalResponse = unknown;
export type TestSmtpData = {
    /**
     * test smtp payload
     */
    requestBody: {
        to: string;
        smtp: {
            host: string;
            username: string;
            password: string;
            port: number;
            from: string;
            tls_implicit: boolean;
        };
    };
};
export type TestSmtpResponse = string;
export type TestLicenseKeyData = {
    /**
     * test license key
     */
    requestBody: {
        license_key: string;
    };
};
export type TestLicenseKeyResponse = string;
export type TestObjectStorageConfigData = {
    /**
     * test object storage config
     */
    requestBody: {
        [key: string]: unknown;
    };
};
export type TestObjectStorageConfigResponse = string;
export type SendStatsResponse = string;
export type TestMetadataData = {
    /**
     * test metadata
     */
    requestBody: string;
};
export type TestMetadataResponse = string;
export type GetOidcTokenData = {
    audience: string;
    workspace: string;
};
export type GetOidcTokenResponse = string;
export type CreateVariableData = {
    alreadyEncrypted?: boolean;
    /**
     * new variable
     */
    requestBody: CreateVariable;
    workspace: string;
};
export type CreateVariableResponse = string;
export type EncryptValueData = {
    /**
     * new variable
     */
    requestBody: string;
    workspace: string;
};
export type EncryptValueResponse = string;
export type DeleteVariableData = {
    path: string;
    workspace: string;
};
export type DeleteVariableResponse = string;
export type UpdateVariableData = {
    alreadyEncrypted?: boolean;
    path: string;
    /**
     * updated variable
     */
    requestBody: EditVariable;
    workspace: string;
};
export type UpdateVariableResponse = string;
export type GetVariableData = {
    /**
     * ask to decrypt secret if this variable is secret
     * (if not secret no effect, default: true)
     *
     */
    decryptSecret?: boolean;
    /**
     * ask to include the encrypted value if secret and decrypt secret is not true (default: false)
     *
     */
    includeEncrypted?: boolean;
    path: string;
    workspace: string;
};
export type GetVariableResponse = ListableVariable;
export type GetVariableValueData = {
    path: string;
    workspace: string;
};
export type GetVariableValueResponse = string;
export type ExistsVariableData = {
    path: string;
    workspace: string;
};
export type ExistsVariableResponse = boolean;
export type ListVariableData = {
    workspace: string;
};
export type ListVariableResponse = Array<ListableVariable>;
export type ListContextualVariablesData = {
    workspace: string;
};
export type ListContextualVariablesResponse = Array<ContextualVariable>;
export type ConnectSlackCallbackData = {
    /**
     * code endpoint
     */
    requestBody: {
        code: string;
        state: string;
    };
    workspace: string;
};
export type ConnectSlackCallbackResponse = string;
export type ConnectCallbackData = {
    clientName: string;
    /**
     * code endpoint
     */
    requestBody: {
        code: string;
        state: string;
    };
};
export type ConnectCallbackResponse = TokenResponse;
export type CreateAccountData = {
    /**
     * code endpoint
     */
    requestBody: {
        refresh_token?: string;
        expires_in: number;
        client: string;
    };
    workspace: string;
};
export type CreateAccountResponse = string;
export type RefreshTokenData = {
    id: number;
    /**
     * variable path
     */
    requestBody: {
        path: string;
    };
    workspace: string;
};
export type RefreshTokenResponse = string;
export type DisconnectAccountData = {
    id: number;
    workspace: string;
};
export type DisconnectAccountResponse = string;
export type DisconnectSlackData = {
    workspace: string;
};
export type DisconnectSlackResponse = string;
export type ListOauthLoginsResponse = {
    oauth: Array<(string)>;
    saml?: string;
};
export type ListOauthConnectsResponse = unknown;
export type CreateResourceData = {
    /**
     * new resource
     */
    requestBody: CreateResource;
    updateIfExists?: boolean;
    workspace: string;
};
export type CreateResourceResponse = string;
export type DeleteResourceData = {
    path: string;
    workspace: string;
};
export type DeleteResourceResponse = string;
export type UpdateResourceData = {
    path: string;
    /**
     * updated resource
     */
    requestBody: EditResource;
    workspace: string;
};
export type UpdateResourceResponse = string;
export type UpdateResourceValueData = {
    path: string;
    /**
     * updated resource
     */
    requestBody: {
        value?: unknown;
    };
    workspace: string;
};
export type UpdateResourceValueResponse = string;
export type GetResourceData = {
    path: string;
    workspace: string;
};
export type GetResourceResponse = Resource;
export type GetResourceValueInterpolatedData = {
    /**
     * job id
     */
    jobId?: string;
    path: string;
    workspace: string;
};
export type GetResourceValueInterpolatedResponse = unknown;
export type GetResourceValueData = {
    path: string;
    workspace: string;
};
export type GetResourceValueResponse = unknown;
export type ExistsResourceData = {
    path: string;
    workspace: string;
};
export type ExistsResourceResponse = boolean;
export type ListResourceData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * resource_types to list from, separated by ',',
     */
    resourceType?: string;
    /**
     * resource_types to not list from, separated by ',',
     */
    resourceTypeExclude?: string;
    workspace: string;
};
export type ListResourceResponse = Array<ListableResource>;
export type ListSearchResourceData = {
    workspace: string;
};
export type ListSearchResourceResponse = Array<{
    path: string;
    value: unknown;
}>;
export type ListResourceNamesData = {
    name: string;
    workspace: string;
};
export type ListResourceNamesResponse = Array<{
    name: string;
    path: string;
}>;
export type CreateResourceTypeData = {
    /**
     * new resource_type
     */
    requestBody: ResourceType;
    workspace: string;
};
export type CreateResourceTypeResponse = string;
export type DeleteResourceTypeData = {
    path: string;
    workspace: string;
};
export type DeleteResourceTypeResponse = string;
export type UpdateResourceTypeData = {
    path: string;
    /**
     * updated resource_type
     */
    requestBody: EditResourceType;
    workspace: string;
};
export type UpdateResourceTypeResponse = string;
export type GetResourceTypeData = {
    path: string;
    workspace: string;
};
export type GetResourceTypeResponse = ResourceType;
export type ExistsResourceTypeData = {
    path: string;
    workspace: string;
};
export type ExistsResourceTypeResponse = boolean;
export type ListResourceTypeData = {
    workspace: string;
};
export type ListResourceTypeResponse = Array<ResourceType>;
export type ListResourceTypeNamesData = {
    workspace: string;
};
export type ListResourceTypeNamesResponse = Array<(string)>;
export type QueryResourceTypesData = {
    /**
     * query limit
     */
    limit?: number;
    /**
     * query text
     */
    text: string;
    workspace: string;
};
export type QueryResourceTypesResponse = Array<{
    name: string;
    score: number;
    schema?: unknown;
}>;
export type ListHubIntegrationsData = {
    /**
     * query integrations kind
     */
    kind?: string;
};
export type ListHubIntegrationsResponse = Array<{
    name: string;
}>;
export type ListHubFlowsResponse = {
    flows?: Array<{
        id: number;
        flow_id: number;
        summary: string;
        apps: Array<(string)>;
        approved: boolean;
        votes: number;
    }>;
};
export type GetHubFlowByIdData = {
    id: number;
};
export type GetHubFlowByIdResponse = {
    flow?: OpenFlow;
};
export type ListFlowPathsData = {
    workspace: string;
};
export type ListFlowPathsResponse = Array<(string)>;
export type ListSearchFlowData = {
    workspace: string;
};
export type ListSearchFlowResponse = Array<{
    path: string;
    value: unknown;
}>;
export type ListFlowsData = {
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * mask to filter exact matching path
     */
    pathExact?: string;
    /**
     * mask to filter matching starting path
     */
    pathStart?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * (default false)
     * show also the archived files.
     * when multiple archived hash share the same path, only the ones with the latest create_at
     * are displayed.
     *
     */
    showArchived?: boolean;
    /**
     * (default false)
     * show only the starred items
     *
     */
    starredOnly?: boolean;
    workspace: string;
};
export type ListFlowsResponse = Array<(Flow & {
    has_draft?: boolean;
    draft_only?: boolean;
})>;
export type GetFlowByPathData = {
    path: string;
    workspace: string;
};
export type GetFlowByPathResponse = Flow;
export type ToggleWorkspaceErrorHandlerForFlowData = {
    path: string;
    /**
     * Workspace error handler enabled
     */
    requestBody: {
        muted?: boolean;
    };
    workspace: string;
};
export type ToggleWorkspaceErrorHandlerForFlowResponse = string;
export type GetFlowByPathWithDraftData = {
    path: string;
    workspace: string;
};
export type GetFlowByPathWithDraftResponse = Flow & {
    draft?: Flow;
};
export type ExistsFlowByPathData = {
    path: string;
    workspace: string;
};
export type ExistsFlowByPathResponse = boolean;
export type CreateFlowData = {
    /**
     * Partially filled flow
     */
    requestBody: OpenFlowWPath & {
        draft_only?: boolean;
        deployment_message?: string;
    };
    workspace: string;
};
export type CreateFlowResponse = string;
export type UpdateFlowData = {
    path: string;
    /**
     * Partially filled flow
     */
    requestBody: OpenFlowWPath & {
        deployment_message?: string;
    };
    workspace: string;
};
export type UpdateFlowResponse = string;
export type ArchiveFlowByPathData = {
    path: string;
    /**
     * archiveFlow
     */
    requestBody: {
        archived?: boolean;
    };
    workspace: string;
};
export type ArchiveFlowByPathResponse = string;
export type DeleteFlowByPathData = {
    path: string;
    workspace: string;
};
export type DeleteFlowByPathResponse = string;
export type GetFlowInputHistoryByPathData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    path: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    workspace: string;
};
export type GetFlowInputHistoryByPathResponse = Array<Input>;
export type ListHubAppsResponse = {
    apps?: Array<{
        id: number;
        app_id: number;
        summary: string;
        apps: Array<(string)>;
        approved: boolean;
        votes: number;
    }>;
};
export type GetHubAppByIdData = {
    id: number;
};
export type GetHubAppByIdResponse = {
    app: {
        summary: string;
        value: unknown;
    };
};
export type ListSearchAppData = {
    workspace: string;
};
export type ListSearchAppResponse = Array<{
    path: string;
    value: unknown;
}>;
export type ListAppsData = {
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * mask to filter exact matching path
     */
    pathExact?: string;
    /**
     * mask to filter matching starting path
     */
    pathStart?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * (default false)
     * show only the starred items
     *
     */
    starredOnly?: boolean;
    workspace: string;
};
export type ListAppsResponse = Array<ListableApp>;
export type CreateAppData = {
    /**
     * new app
     */
    requestBody: {
        path: string;
        value: unknown;
        summary: string;
        policy: Policy;
        draft_only?: boolean;
        deployment_message?: string;
    };
    workspace: string;
};
export type CreateAppResponse = string;
export type ExistsAppData = {
    path: string;
    workspace: string;
};
export type ExistsAppResponse = boolean;
export type GetAppByPathData = {
    path: string;
    workspace: string;
};
export type GetAppByPathResponse = AppWithLastVersion;
export type GetAppByPathWithDraftData = {
    path: string;
    workspace: string;
};
export type GetAppByPathWithDraftResponse = AppWithLastVersionWDraft;
export type GetAppHistoryByPathData = {
    path: string;
    workspace: string;
};
export type GetAppHistoryByPathResponse = Array<AppHistory>;
export type UpdateAppHistoryData = {
    id: number;
    /**
     * App deployment message
     */
    requestBody: {
        deployment_msg?: string;
    };
    version: number;
    workspace: string;
};
export type UpdateAppHistoryResponse = string;
export type GetPublicAppBySecretData = {
    path: string;
    workspace: string;
};
export type GetPublicAppBySecretResponse = AppWithLastVersion;
export type GetPublicResourceData = {
    path: string;
    workspace: string;
};
export type GetPublicResourceResponse = unknown;
export type GetPublicSecretOfAppData = {
    path: string;
    workspace: string;
};
export type GetPublicSecretOfAppResponse = string;
export type GetAppByVersionData = {
    id: number;
    workspace: string;
};
export type GetAppByVersionResponse = AppWithLastVersion;
export type DeleteAppData = {
    path: string;
    workspace: string;
};
export type DeleteAppResponse = string;
export type UpdateAppData = {
    path: string;
    /**
     * update app
     */
    requestBody: {
        path?: string;
        summary?: string;
        value?: unknown;
        policy?: Policy;
        deployment_message?: string;
    };
    workspace: string;
};
export type UpdateAppResponse = string;
export type ExecuteComponentData = {
    path: string;
    /**
     * update app
     */
    requestBody: {
        component: string;
        path?: string;
        args: unknown;
        raw_code?: {
            content: string;
            language: string;
            path?: string;
            cache_ttl?: number;
        };
        force_viewer_static_fields?: {
            [key: string]: unknown;
        };
        force_viewer_one_of_fields?: {
            [key: string]: unknown;
        };
    };
    workspace: string;
};
export type ExecuteComponentResponse = string;
export type GetHubScriptContentByPathData = {
    path: string;
};
export type GetHubScriptContentByPathResponse = string;
export type GetHubScriptByPathData = {
    path: string;
};
export type GetHubScriptByPathResponse = {
    content: string;
    lockfile?: string;
    schema?: unknown;
    language: string;
    summary?: string;
};
export type GetTopHubScriptsData = {
    /**
     * query scripts app
     */
    app?: string;
    /**
     * query scripts kind
     */
    kind?: string;
    /**
     * query limit
     */
    limit?: number;
};
export type GetTopHubScriptsResponse = {
    asks?: Array<{
        id: number;
        ask_id: number;
        summary: string;
        app: string;
        version_id: number;
        kind: HubScriptKind;
        votes: number;
        views: number;
    }>;
};
export type QueryHubScriptsData = {
    /**
     * query scripts app
     */
    app?: string;
    /**
     * query scripts kind
     */
    kind?: string;
    /**
     * query limit
     */
    limit?: number;
    /**
     * query text
     */
    text: string;
};
export type QueryHubScriptsResponse = Array<{
    ask_id: number;
    id: number;
    version_id: number;
    summary: string;
    app: string;
    kind: HubScriptKind;
    score: number;
}>;
export type ListSearchScriptData = {
    workspace: string;
};
export type ListSearchScriptResponse = Array<{
    path: string;
    content: string;
}>;
export type ListScriptsData = {
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * mask to filter scripts whom first direct parent has exact hash
     */
    firstParentHash?: string;
    /**
     * (default false)
     * hide the scripts without an exported main function
     *
     */
    hideWithoutMain?: boolean;
    /**
     * (default regardless)
     * if true show only the templates
     * if false show only the non templates
     * if not defined, show all regardless of if the script is a template
     *
     */
    isTemplate?: boolean;
    /**
     * (default regardless)
     * script kinds to filter, split by comma
     *
     */
    kinds?: string;
    /**
     * mask to filter scripts whom last parent in the chain has exact hash.
     * Beware that each script stores only a limited number of parents. Hence
     * the last parent hash for a script is not necessarily its top-most parent.
     * To find the top-most parent you will have to jump from last to last hash
     * until finding the parent
     *
     */
    lastParentHash?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * is the hash present in the array of stored parent hashes for this script.
     * The same warning applies than for last_parent_hash. A script only store a
     * limited number of direct parent
     *
     */
    parentHash?: string;
    /**
     * mask to filter exact matching path
     */
    pathExact?: string;
    /**
     * mask to filter matching starting path
     */
    pathStart?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * (default false)
     * show also the archived files.
     * when multiple archived hash share the same path, only the ones with the latest create_at
     * are
     * ed.
     *
     */
    showArchived?: boolean;
    /**
     * (default false)
     * show only the starred items
     *
     */
    starredOnly?: boolean;
    workspace: string;
};
export type ListScriptsResponse = Array<Script>;
export type ListScriptPathsData = {
    workspace: string;
};
export type ListScriptPathsResponse = Array<(string)>;
export type CreateScriptData = {
    /**
     * Partially filled script
     */
    requestBody: NewScript;
    workspace: string;
};
export type CreateScriptResponse = string;
export type ToggleWorkspaceErrorHandlerForScriptData = {
    path: string;
    /**
     * Workspace error handler enabled
     */
    requestBody: {
        muted?: boolean;
    };
    workspace: string;
};
export type ToggleWorkspaceErrorHandlerForScriptResponse = string;
export type ArchiveScriptByPathData = {
    path: string;
    workspace: string;
};
export type ArchiveScriptByPathResponse = string;
export type ArchiveScriptByHashData = {
    hash: string;
    workspace: string;
};
export type ArchiveScriptByHashResponse = Script;
export type DeleteScriptByHashData = {
    hash: string;
    workspace: string;
};
export type DeleteScriptByHashResponse = Script;
export type DeleteScriptByPathData = {
    path: string;
    workspace: string;
};
export type DeleteScriptByPathResponse = string;
export type GetScriptByPathData = {
    path: string;
    workspace: string;
};
export type GetScriptByPathResponse = Script;
export type GetScriptByPathWithDraftData = {
    path: string;
    workspace: string;
};
export type GetScriptByPathWithDraftResponse = NewScriptWithDraft;
export type GetScriptHistoryByPathData = {
    path: string;
    workspace: string;
};
export type GetScriptHistoryByPathResponse = Array<ScriptHistory>;
export type UpdateScriptHistoryData = {
    hash: string;
    path: string;
    /**
     * Script deployment message
     */
    requestBody: {
        deployment_msg?: string;
    };
    workspace: string;
};
export type UpdateScriptHistoryResponse = string;
export type RawScriptByPathData = {
    path: string;
    workspace: string;
};
export type RawScriptByPathResponse = string;
export type RawScriptByPathTokenedData = {
    path: string;
    token: string;
    workspace: string;
};
export type RawScriptByPathTokenedResponse = string;
export type ExistsScriptByPathData = {
    path: string;
    workspace: string;
};
export type ExistsScriptByPathResponse = boolean;
export type GetScriptByHashData = {
    hash: string;
    workspace: string;
};
export type GetScriptByHashResponse = Script;
export type RawScriptByHashData = {
    path: string;
    workspace: string;
};
export type RawScriptByHashResponse = string;
export type GetScriptDeploymentStatusData = {
    hash: string;
    workspace: string;
};
export type GetScriptDeploymentStatusResponse = {
    lock?: string;
    lock_error_logs?: string;
};
export type CreateDraftData = {
    requestBody: {
        path: string;
        typ: 'flow' | 'script' | 'app';
        value?: unknown;
    };
    workspace: string;
};
export type CreateDraftResponse = string;
export type DeleteDraftData = {
    kind: 'script' | 'flow' | 'app';
    path: string;
    workspace: string;
};
export type DeleteDraftResponse = string;
export type GetCustomTagsResponse = Array<(string)>;
export type GeDefaultTagsResponse = Array<(string)>;
export type IsDefaultTagsPerWorkspaceResponse = boolean;
export type ListWorkersData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * number of seconds the worker must have had a last ping more recent of (default to 300)
     */
    pingSince?: number;
};
export type ListWorkersResponse = Array<WorkerPing>;
export type ExistsWorkerWithTagData = {
    tag: string;
};
export type ExistsWorkerWithTagResponse = boolean;
export type GetQueueMetricsResponse = Array<{
    id: string;
    values: Array<{
        created_at: string;
        value: number;
    }>;
}>;
export type RunScriptByPathData = {
    /**
     * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     */
    cacheTtl?: string;
    /**
     * make the run invisible to the the script owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    path: string;
    /**
     * script args
     */
    requestBody: ScriptArgs;
    /**
     * when to schedule this job (leave empty for immediate run)
     */
    scheduledFor?: string;
    /**
     * schedule the script to execute in the number of seconds starting now
     */
    scheduledInSecs?: number;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RunScriptByPathResponse = string;
export type OpenaiSyncScriptByPathData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    path: string;
    /**
     * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     */
    queueLimit?: string;
    /**
     * script args
     */
    requestBody: ScriptArgs;
    workspace: string;
};
export type OpenaiSyncScriptByPathResponse = unknown;
export type RunWaitResultScriptByPathData = {
    /**
     * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     */
    cacheTtl?: string;
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    path: string;
    /**
     * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     */
    queueLimit?: string;
    /**
     * script args
     */
    requestBody: ScriptArgs;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RunWaitResultScriptByPathResponse = unknown;
export type RunWaitResultScriptByPathGetData = {
    /**
     * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     */
    cacheTtl?: string;
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    path: string;
    /**
     * The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
     * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
     *
     */
    payload?: string;
    /**
     * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     */
    queueLimit?: string;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RunWaitResultScriptByPathGetResponse = unknown;
export type OpenaiSyncFlowByPathData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    path: string;
    /**
     * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     */
    queueLimit?: string;
    /**
     * script args
     */
    requestBody: ScriptArgs;
    workspace: string;
};
export type OpenaiSyncFlowByPathResponse = unknown;
export type RunWaitResultFlowByPathData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    path: string;
    /**
     * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
     *
     */
    queueLimit?: string;
    /**
     * script args
     */
    requestBody: ScriptArgs;
    workspace: string;
};
export type RunWaitResultFlowByPathResponse = unknown;
export type ResultByIdData = {
    flowJobId: string;
    nodeId: string;
    workspace: string;
};
export type ResultByIdResponse = unknown;
export type RunFlowByPathData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * make the run invisible to the the flow owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    path: string;
    /**
     * flow args
     */
    requestBody: ScriptArgs;
    /**
     * when to schedule this job (leave empty for immediate run)
     */
    scheduledFor?: string;
    /**
     * schedule the script to execute in the number of seconds starting now
     */
    scheduledInSecs?: number;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RunFlowByPathResponse = string;
export type RestartFlowAtStepData = {
    /**
     * for branchall or loop, the iteration at which the flow should restart
     */
    branchOrIterationN: number;
    id: string;
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * make the run invisible to the the flow owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    /**
     * flow args
     */
    requestBody: ScriptArgs;
    /**
     * when to schedule this job (leave empty for immediate run)
     */
    scheduledFor?: string;
    /**
     * schedule the script to execute in the number of seconds starting now
     */
    scheduledInSecs?: number;
    /**
     * step id to restart the flow from
     */
    stepId: string;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RestartFlowAtStepResponse = string;
export type RunScriptByHashData = {
    /**
     * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
     */
    cacheTtl?: string;
    hash: string;
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * make the run invisible to the the script owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    /**
     * Partially filled args
     */
    requestBody: {
        [key: string]: unknown;
    };
    /**
     * when to schedule this job (leave empty for immediate run)
     */
    scheduledFor?: string;
    /**
     * schedule the script to execute in the number of seconds starting now
     */
    scheduledInSecs?: number;
    /**
     * Override the tag to use
     */
    tag?: string;
    workspace: string;
};
export type RunScriptByHashResponse = string;
export type RunScriptPreviewData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * make the run invisible to the the script owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * preview
     */
    requestBody: Preview;
    workspace: string;
};
export type RunScriptPreviewResponse = string;
export type RunCodeWorkflowTaskData = {
    entrypoint: string;
    jobId: string;
    /**
     * preview
     */
    requestBody: WorkflowTask;
    workspace: string;
};
export type RunCodeWorkflowTaskResponse = string;
export type RunRawScriptDependenciesData = {
    /**
     * raw script content
     */
    requestBody: {
        raw_scripts: Array<RawScriptForDependencies>;
        entrypoint: string;
    };
    workspace: string;
};
export type RunRawScriptDependenciesResponse = {
    lock: string;
};
export type RunFlowPreviewData = {
    /**
     * List of headers's keys (separated with ',') whove value are added to the args
     * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
     *
     */
    includeHeader?: string;
    /**
     * make the run invisible to the the script owner (default false)
     */
    invisibleToOwner?: boolean;
    /**
     * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
     */
    jobId?: string;
    /**
     * preview
     */
    requestBody: FlowPreview;
    workspace: string;
};
export type RunFlowPreviewResponse = string;
export type ListQueueData = {
    /**
     * get jobs from all workspaces (only valid if request come from the `admins` workspace)
     */
    allWorkspaces?: boolean;
    /**
     * filter on jobs containing those args as a json subset (@> in postgres)
     */
    args?: string;
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * is not a scheduled job
     */
    isNotSchedule?: boolean;
    /**
     * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     */
    jobKinds?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * filter on jobs containing those result as a json subset (@> in postgres)
     */
    result?: string;
    /**
     * filter on running jobs
     */
    running?: boolean;
    /**
     * filter on jobs scheduled_for before now (hence waitinf for a worker)
     */
    scheduledForBeforeNow?: boolean;
    /**
     * mask to filter by schedule path
     */
    schedulePath?: string;
    /**
     * mask to filter exact matching path
     */
    scriptHash?: string;
    /**
     * mask to filter exact matching path
     */
    scriptPathExact?: string;
    /**
     * mask to filter matching starting path
     */
    scriptPathStart?: string;
    /**
     * filter on started after (exclusive) timestamp
     */
    startedAfter?: string;
    /**
     * filter on started before (inclusive) timestamp
     */
    startedBefore?: string;
    /**
     * filter on successful jobs
     */
    success?: boolean;
    /**
     * filter on suspended jobs
     */
    suspended?: boolean;
    /**
     * filter on jobs with a given tag/worker group
     */
    tag?: string;
    workspace: string;
};
export type ListQueueResponse = Array<QueuedJob>;
export type GetQueueCountData = {
    /**
     * get jobs from all workspaces (only valid if request come from the `admins` workspace)
     */
    allWorkspaces?: boolean;
    workspace: string;
};
export type GetQueueCountResponse = {
    database_length: number;
};
export type GetCompletedCountData = {
    workspace: string;
};
export type GetCompletedCountResponse = {
    database_length: number;
};
export type CancelAllData = {
    workspace: string;
};
export type CancelAllResponse = Array<(string)>;
export type ListCompletedJobsData = {
    /**
     * filter on jobs containing those args as a json subset (@> in postgres)
     */
    args?: string;
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * has null parent
     */
    hasNullParent?: boolean;
    /**
     * is the job a flow step
     */
    isFlowStep?: boolean;
    /**
     * is not a scheduled job
     */
    isNotSchedule?: boolean;
    /**
     * is the job skipped
     */
    isSkipped?: boolean;
    /**
     * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     */
    jobKinds?: string;
    /**
     * mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
     */
    label?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * filter on jobs containing those result as a json subset (@> in postgres)
     */
    result?: string;
    /**
     * mask to filter by schedule path
     */
    schedulePath?: string;
    /**
     * mask to filter exact matching path
     */
    scriptHash?: string;
    /**
     * mask to filter exact matching path
     */
    scriptPathExact?: string;
    /**
     * mask to filter matching starting path
     */
    scriptPathStart?: string;
    /**
     * filter on started after (exclusive) timestamp
     */
    startedAfter?: string;
    /**
     * filter on started before (inclusive) timestamp
     */
    startedBefore?: string;
    /**
     * filter on successful jobs
     */
    success?: boolean;
    /**
     * filter on jobs with a given tag/worker group
     */
    tag?: string;
    workspace: string;
};
export type ListCompletedJobsResponse = Array<CompletedJob>;
export type ListJobsData = {
    /**
     * get jobs from all workspaces (only valid if request come from the `admins` workspace)
     */
    allWorkspaces?: boolean;
    /**
     * filter on jobs containing those args as a json subset (@> in postgres)
     */
    args?: string;
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
     */
    createdOrStartedAfter?: string;
    /**
     * filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
     */
    createdOrStartedBefore?: string;
    /**
     * has null parent
     */
    hasNullParent?: boolean;
    /**
     * is the job a flow step
     */
    isFlowStep?: boolean;
    /**
     * is not a scheduled job
     */
    isNotSchedule?: boolean;
    /**
     * is the job skipped
     */
    isSkipped?: boolean;
    /**
     * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
     */
    jobKinds?: string;
    /**
     * mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
     */
    label?: string;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * The parent job that is at the origin and responsible for the execution of this script if any
     */
    parentJob?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * filter on jobs containing those result as a json subset (@> in postgres)
     */
    result?: string;
    /**
     * filter on running jobs
     */
    running?: boolean;
    /**
     * filter on jobs scheduled_for before now (hence waitinf for a worker)
     */
    scheduledForBeforeNow?: boolean;
    /**
     * mask to filter by schedule path
     */
    schedulePath?: string;
    /**
     * mask to filter exact matching path
     */
    scriptHash?: string;
    /**
     * mask to filter exact matching path
     */
    scriptPathExact?: string;
    /**
     * mask to filter matching starting path
     */
    scriptPathStart?: string;
    /**
     * filter on started after (exclusive) timestamp
     */
    startedAfter?: string;
    /**
     * filter on started before (inclusive) timestamp
     */
    startedBefore?: string;
    /**
     * filter on successful jobs
     */
    success?: boolean;
    /**
     * filter on jobs with a given tag/worker group
     */
    tag?: string;
    workspace: string;
};
export type ListJobsResponse = Array<Job>;
export type GetDbClockResponse = number;
export type GetJobData = {
    id: string;
    noLogs?: boolean;
    workspace: string;
};
export type GetJobResponse = Job;
export type GetRootJobIdData = {
    id: string;
    workspace: string;
};
export type GetRootJobIdResponse = string;
export type GetJobLogsData = {
    id: string;
    workspace: string;
};
export type GetJobLogsResponse = string;
export type GetJobUpdatesData = {
    id: string;
    logOffset?: number;
    running?: boolean;
    workspace: string;
};
export type GetJobUpdatesResponse = {
    running?: boolean;
    completed?: boolean;
    new_logs?: string;
    log_offset?: number;
    mem_peak?: number;
    flow_status?: WorkflowStatusRecord;
};
export type GetLogFileFromStoreData = {
    path: string;
    workspace: string;
};
export type GetLogFileFromStoreResponse = unknown;
export type GetFlowDebugInfoData = {
    id: string;
    workspace: string;
};
export type GetFlowDebugInfoResponse = unknown;
export type GetCompletedJobData = {
    id: string;
    workspace: string;
};
export type GetCompletedJobResponse = CompletedJob;
export type GetCompletedJobResultData = {
    id: string;
    workspace: string;
};
export type GetCompletedJobResultResponse = unknown;
export type GetCompletedJobResultMaybeData = {
    getStarted?: boolean;
    id: string;
    workspace: string;
};
export type GetCompletedJobResultMaybeResponse = {
    completed: boolean;
    result: unknown;
    success?: boolean;
    started?: boolean;
};
export type DeleteCompletedJobData = {
    id: string;
    workspace: string;
};
export type DeleteCompletedJobResponse = CompletedJob;
export type CancelQueuedJobData = {
    id: string;
    /**
     * reason
     */
    requestBody: {
        reason?: string;
    };
    workspace: string;
};
export type CancelQueuedJobResponse = string;
export type CancelPersistentQueuedJobsData = {
    path: string;
    /**
     * reason
     */
    requestBody: {
        reason?: string;
    };
    workspace: string;
};
export type CancelPersistentQueuedJobsResponse = string;
export type ForceCancelQueuedJobData = {
    id: string;
    /**
     * reason
     */
    requestBody: {
        reason?: string;
    };
    workspace: string;
};
export type ForceCancelQueuedJobResponse = string;
export type CreateJobSignatureData = {
    approver?: string;
    id: string;
    resumeId: number;
    workspace: string;
};
export type CreateJobSignatureResponse = string;
export type GetResumeUrlsData = {
    approver?: string;
    id: string;
    resumeId: number;
    workspace: string;
};
export type GetResumeUrlsResponse = {
    approvalPage: string;
    resume: string;
    cancel: string;
};
export type ResumeSuspendedJobGetData = {
    approver?: string;
    id: string;
    /**
     * The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
     * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
     *
     */
    payload?: string;
    resumeId: number;
    signature: string;
    workspace: string;
};
export type ResumeSuspendedJobGetResponse = string;
export type ResumeSuspendedJobPostData = {
    approver?: string;
    id: string;
    requestBody: {
        [key: string]: unknown;
    };
    resumeId: number;
    signature: string;
    workspace: string;
};
export type ResumeSuspendedJobPostResponse = string;
export type SetFlowUserStateData = {
    id: string;
    key: string;
    /**
     * new value
     */
    requestBody: unknown;
    workspace: string;
};
export type SetFlowUserStateResponse = string;
export type GetFlowUserStateData = {
    id: string;
    key: string;
    workspace: string;
};
export type GetFlowUserStateResponse = unknown;
export type ResumeSuspendedFlowAsOwnerData = {
    id: string;
    requestBody: {
        [key: string]: unknown;
    };
    workspace: string;
};
export type ResumeSuspendedFlowAsOwnerResponse = string;
export type CancelSuspendedJobGetData = {
    approver?: string;
    id: string;
    resumeId: number;
    signature: string;
    workspace: string;
};
export type CancelSuspendedJobGetResponse = string;
export type CancelSuspendedJobPostData = {
    approver?: string;
    id: string;
    requestBody: {
        [key: string]: unknown;
    };
    resumeId: number;
    signature: string;
    workspace: string;
};
export type CancelSuspendedJobPostResponse = string;
export type GetSuspendedJobFlowData = {
    approver?: string;
    id: string;
    resumeId: number;
    signature: string;
    workspace: string;
};
export type GetSuspendedJobFlowResponse = {
    job: Job;
    approvers: Array<{
        resume_id: number;
        approver: string;
    }>;
};
export type ListRawAppsData = {
    /**
     * mask to filter exact matching user creator
     */
    createdBy?: string;
    /**
     * order by desc order (default true)
     */
    orderDesc?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * mask to filter exact matching path
     */
    pathExact?: string;
    /**
     * mask to filter matching starting path
     */
    pathStart?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    /**
     * (default false)
     * show only the starred items
     *
     */
    starredOnly?: boolean;
    workspace: string;
};
export type ListRawAppsResponse = Array<ListableRawApp>;
export type ExistsRawAppData = {
    path: string;
    workspace: string;
};
export type ExistsRawAppResponse = boolean;
export type GetRawAppDataData = {
    path: string;
    version: number;
    workspace: string;
};
export type GetRawAppDataResponse = string;
export type CreateRawAppData = {
    /**
     * new raw app
     */
    requestBody: {
        path: string;
        value: string;
        summary: string;
    };
    workspace: string;
};
export type CreateRawAppResponse = string;
export type UpdateRawAppData = {
    path: string;
    /**
     * updateraw  app
     */
    requestBody: {
        path?: string;
        summary?: string;
        value?: string;
    };
    workspace: string;
};
export type UpdateRawAppResponse = string;
export type DeleteRawAppData = {
    path: string;
    workspace: string;
};
export type DeleteRawAppResponse = string;
export type PreviewScheduleData = {
    /**
     * schedule
     */
    requestBody: {
        schedule: string;
        timezone: string;
    };
};
export type PreviewScheduleResponse = Array<(string)>;
export type CreateScheduleData = {
    /**
     * new schedule
     */
    requestBody: NewSchedule;
    workspace: string;
};
export type CreateScheduleResponse = string;
export type UpdateScheduleData = {
    path: string;
    /**
     * updated schedule
     */
    requestBody: EditSchedule;
    workspace: string;
};
export type UpdateScheduleResponse = string;
export type SetScheduleEnabledData = {
    path: string;
    /**
     * updated schedule enable
     */
    requestBody: {
        enabled: boolean;
    };
    workspace: string;
};
export type SetScheduleEnabledResponse = string;
export type DeleteScheduleData = {
    path: string;
    workspace: string;
};
export type DeleteScheduleResponse = string;
export type GetScheduleData = {
    path: string;
    workspace: string;
};
export type GetScheduleResponse = Schedule;
export type ExistsScheduleData = {
    path: string;
    workspace: string;
};
export type ExistsScheduleResponse = boolean;
export type ListSchedulesData = {
    /**
     * filter on jobs containing those args as a json subset (@> in postgres)
     */
    args?: string;
    isFlow?: boolean;
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * filter by path
     */
    path?: string;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    workspace: string;
};
export type ListSchedulesResponse = Array<Schedule>;
export type ListSchedulesWithJobsData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    workspace: string;
};
export type ListSchedulesWithJobsResponse = Array<ScheduleWJobs>;
export type SetDefaultErrorOrRecoveryHandlerData = {
    /**
     * Handler description
     */
    requestBody: {
        handler_type: 'error' | 'recovery';
        override_existing: boolean;
        path?: string;
        extra_args?: {
            [key: string]: unknown;
        };
        number_of_occurence?: number;
        number_of_occurence_exact?: boolean;
        workspace_handler_muted?: boolean;
    };
    workspace: string;
};
export type SetDefaultErrorOrRecoveryHandlerResponse = unknown;
export type ListInstanceGroupsResponse = Array<InstanceGroup>;
export type GetInstanceGroupData = {
    name: string;
};
export type GetInstanceGroupResponse = InstanceGroup;
export type CreateInstanceGroupData = {
    /**
     * create instance group
     */
    requestBody: {
        name: string;
        summary?: string;
    };
};
export type CreateInstanceGroupResponse = string;
export type UpdateInstanceGroupData = {
    name: string;
    /**
     * update instance group
     */
    requestBody: {
        new_summary: string;
    };
};
export type UpdateInstanceGroupResponse = string;
export type DeleteInstanceGroupData = {
    name: string;
};
export type DeleteInstanceGroupResponse = string;
export type AddUserToInstanceGroupData = {
    name: string;
    /**
     * user to add to instance group
     */
    requestBody: {
        email: string;
    };
};
export type AddUserToInstanceGroupResponse = string;
export type RemoveUserFromInstanceGroupData = {
    name: string;
    /**
     * user to remove from instance group
     */
    requestBody: {
        email: string;
    };
};
export type RemoveUserFromInstanceGroupResponse = string;
export type ListGroupsData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    workspace: string;
};
export type ListGroupsResponse = Array<Group>;
export type ListGroupNamesData = {
    /**
     * only list the groups the user is member of (default false)
     */
    onlyMemberOf?: boolean;
    workspace: string;
};
export type ListGroupNamesResponse = Array<(string)>;
export type CreateGroupData = {
    /**
     * create group
     */
    requestBody: {
        name: string;
        summary?: string;
    };
    workspace: string;
};
export type CreateGroupResponse = string;
export type UpdateGroupData = {
    name: string;
    /**
     * updated group
     */
    requestBody: {
        summary?: string;
    };
    workspace: string;
};
export type UpdateGroupResponse = string;
export type DeleteGroupData = {
    name: string;
    workspace: string;
};
export type DeleteGroupResponse = string;
export type GetGroupData = {
    name: string;
    workspace: string;
};
export type GetGroupResponse = Group;
export type AddUserToGroupData = {
    name: string;
    /**
     * added user to group
     */
    requestBody: {
        username?: string;
    };
    workspace: string;
};
export type AddUserToGroupResponse = string;
export type RemoveUserToGroupData = {
    name: string;
    /**
     * added user to group
     */
    requestBody: {
        username?: string;
    };
    workspace: string;
};
export type RemoveUserToGroupResponse = string;
export type ListFoldersData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    workspace: string;
};
export type ListFoldersResponse = Array<Folder>;
export type ListFolderNamesData = {
    /**
     * only list the folders the user is member of (default false)
     */
    onlyMemberOf?: boolean;
    workspace: string;
};
export type ListFolderNamesResponse = Array<(string)>;
export type CreateFolderData = {
    /**
     * create folder
     */
    requestBody: {
        name: string;
        owners?: Array<(string)>;
        extra_perms?: unknown;
    };
    workspace: string;
};
export type CreateFolderResponse = string;
export type UpdateFolderData = {
    name: string;
    /**
     * update folder
     */
    requestBody: {
        owners?: Array<(string)>;
        extra_perms?: unknown;
    };
    workspace: string;
};
export type UpdateFolderResponse = string;
export type DeleteFolderData = {
    name: string;
    workspace: string;
};
export type DeleteFolderResponse = string;
export type GetFolderData = {
    name: string;
    workspace: string;
};
export type GetFolderResponse = Folder;
export type GetFolderUsageData = {
    name: string;
    workspace: string;
};
export type GetFolderUsageResponse = {
    scripts: number;
    flows: number;
    apps: number;
    resources: number;
    variables: number;
    schedules: number;
};
export type AddOwnerToFolderData = {
    name: string;
    /**
     * owner user to folder
     */
    requestBody: {
        owner: string;
    };
    workspace: string;
};
export type AddOwnerToFolderResponse = string;
export type RemoveOwnerToFolderData = {
    name: string;
    /**
     * added owner to folder
     */
    requestBody: {
        owner: string;
        write?: boolean;
    };
    workspace: string;
};
export type RemoveOwnerToFolderResponse = string;
export type ListWorkerGroupsResponse = Array<{
    name: string;
    config: unknown;
}>;
export type GetConfigData = {
    name: string;
};
export type GetConfigResponse = unknown;
export type UpdateConfigData = {
    name: string;
    /**
     * worker group
     */
    requestBody: unknown;
};
export type UpdateConfigResponse = string;
export type DeleteConfigData = {
    name: string;
};
export type DeleteConfigResponse = string;
export type GetGranularAclsData = {
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
    path: string;
    workspace: string;
};
export type GetGranularAclsResponse = {
    [key: string]: (boolean);
};
export type AddGranularAclsData = {
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
    path: string;
    /**
     * acl to add
     */
    requestBody: {
        owner: string;
        write?: boolean;
    };
    workspace: string;
};
export type AddGranularAclsResponse = string;
export type RemoveGranularAclsData = {
    kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
    path: string;
    /**
     * acl to add
     */
    requestBody: {
        owner: string;
    };
    workspace: string;
};
export type RemoveGranularAclsResponse = string;
export type UpdateCaptureData = {
    path: string;
    workspace: string;
};
export type UpdateCaptureResponse = void;
export type CreateCaptureData = {
    path: string;
    workspace: string;
};
export type CreateCaptureResponse = unknown;
export type GetCaptureData = {
    path: string;
    workspace: string;
};
export type GetCaptureResponse = unknown;
export type StarData = {
    requestBody?: {
        path?: string;
        favorite_kind?: 'flow' | 'app' | 'script' | 'raw_app';
    };
    workspace: string;
};
export type StarResponse = unknown;
export type UnstarData = {
    requestBody?: {
        path?: string;
        favorite_kind?: 'flow' | 'app' | 'script' | 'raw_app';
    };
    workspace: string;
};
export type UnstarResponse = unknown;
export type GetInputHistoryData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    runnableId?: string;
    runnableType?: RunnableType;
    workspace: string;
};
export type GetInputHistoryResponse = Array<Input>;
export type GetArgsFromHistoryOrSavedInputData = {
    jobOrInputId: string;
    workspace: string;
};
export type GetArgsFromHistoryOrSavedInputResponse = unknown;
export type ListInputsData = {
    /**
     * which page to return (start at 1, default 1)
     */
    page?: number;
    /**
     * number of items to return for a given page (default 30, max 100)
     */
    perPage?: number;
    runnableId?: string;
    runnableType?: RunnableType;
    workspace: string;
};
export type ListInputsResponse = Array<Input>;
export type CreateInputData = {
    /**
     * Input
     */
    requestBody: CreateInput;
    runnableId?: string;
    runnableType?: RunnableType;
    workspace: string;
};
export type CreateInputResponse = string;
export type UpdateInputData = {
    /**
     * UpdateInput
     */
    requestBody: UpdateInput;
    workspace: string;
};
export type UpdateInputResponse = string;
export type DeleteInputData = {
    input: string;
    workspace: string;
};
export type DeleteInputResponse = string;
export type DuckdbConnectionSettingsData = {
    /**
     * S3 resource to connect to
     */
    requestBody: {
        s3_resource?: S3Resource;
    };
    workspace: string;
};
export type DuckdbConnectionSettingsResponse = {
    connection_settings_str?: string;
};
export type DuckdbConnectionSettingsV2Data = {
    /**
     * S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
     */
    requestBody: {
        s3_resource_path?: string;
    };
    workspace: string;
};
export type DuckdbConnectionSettingsV2Response = {
    connection_settings_str: string;
};
export type PolarsConnectionSettingsData = {
    /**
     * S3 resource to connect to
     */
    requestBody: {
        s3_resource?: S3Resource;
    };
    workspace: string;
};
export type PolarsConnectionSettingsResponse = {
    endpoint_url: string;
    key?: string;
    secret?: string;
    use_ssl: boolean;
    cache_regions: boolean;
    client_kwargs: PolarsClientKwargs;
};
export type PolarsConnectionSettingsV2Data = {
    /**
     * S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
     */
    requestBody: {
        s3_resource_path?: string;
    };
    workspace: string;
};
export type PolarsConnectionSettingsV2Response = {
    s3fs_args: {
        endpoint_url: string;
        key?: string;
        secret?: string;
        use_ssl: boolean;
        cache_regions: boolean;
        client_kwargs: PolarsClientKwargs;
    };
    storage_options: {
        aws_endpoint_url: string;
        aws_access_key_id?: string;
        aws_secret_access_key?: string;
        aws_region: string;
        aws_allow_http: string;
    };
};
export type S3ResourceInfoData = {
    /**
     * S3 resource path to use. If empty, the S3 resource defined in the workspace settings will be used
     */
    requestBody: {
        s3_resource_path?: string;
    };
    workspace: string;
};
export type S3ResourceInfoResponse = S3Resource;
export type DatasetStorageTestConnectionData = {
    workspace: string;
};
export type DatasetStorageTestConnectionResponse = unknown;
export type ListStoredFilesData = {
    marker?: string;
    maxKeys: number;
    prefix?: string;
    workspace: string;
};
export type ListStoredFilesResponse = {
    next_marker?: string;
    windmill_large_files: Array<WindmillLargeFile>;
    restricted_access?: boolean;
};
export type LoadFileMetadataData = {
    fileKey: string;
    workspace: string;
};
export type LoadFileMetadataResponse = WindmillFileMetadata;
export type LoadFilePreviewData = {
    csvHasHeader?: boolean;
    csvSeparator?: string;
    fileKey: string;
    fileMimeType?: string;
    fileSizeInBytes?: number;
    readBytesFrom?: number;
    readBytesLength?: number;
    workspace: string;
};
export type LoadFilePreviewResponse = WindmillFilePreview;
export type LoadParquetPreviewData = {
    limit?: number;
    offset?: number;
    path: string;
    searchCol?: string;
    searchTerm?: string;
    sortCol?: string;
    sortDesc?: boolean;
    workspace: string;
};
export type LoadParquetPreviewResponse = unknown;
export type DeleteS3FileData = {
    fileKey: string;
    workspace: string;
};
export type DeleteS3FileResponse = unknown;
export type MoveS3FileData = {
    destFileKey: string;
    srcFileKey: string;
    workspace: string;
};
export type MoveS3FileResponse = unknown;
export type FileUploadData = {
    fileExtension?: string;
    fileKey?: string;
    /**
     * File content
     */
    requestBody: (Blob | File);
    resourceType?: string;
    s3ResourcePath?: string;
    workspace: string;
};
export type FileUploadResponse = {
    file_key: string;
};
export type FileDownloadData = {
    fileKey: string;
    resourceType?: string;
    s3ResourcePath?: string;
    workspace: string;
};
export type FileDownloadResponse = (Blob | File);
export type GetJobMetricsData = {
    id: string;
    /**
     * parameters for statistics retrieval
     */
    requestBody: {
        timeseries_max_datapoints?: number;
        from_timestamp?: string;
        to_timestamp?: string;
    };
    workspace: string;
};
export type GetJobMetricsResponse = {
    metrics_metadata?: Array<MetricMetadata>;
    scalar_metrics?: Array<ScalarMetric>;
    timeseries_metrics?: Array<TimeseriesMetric>;
};
export type ListConcurrencyGroupsResponse = Array<ConcurrencyGroup>;
export type DeleteConcurrencyGroupData = {
    concurrencyId: string;
};
export type DeleteConcurrencyGroupResponse = unknown;
export type $OpenApiTs = {
    '/version': {
        get: {
            res: {
                /**
                 * git version of backend
                 */
                200: string;
            };
        };
    };
    '/uptodate': {
        get: {
            res: {
                /**
                 * is backend up to date
                 */
                200: string;
            };
        };
    };
    '/ee_license': {
        get: {
            res: {
                /**
                 * get license id (empty if not ee)
                 */
                200: string;
            };
        };
    };
    '/openapi.yaml': {
        get: {
            res: {
                /**
                 * openapi yaml file content
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/audit/get/{id}': {
        get: {
            req: {
                id: number;
                workspace: string;
            };
            res: {
                /**
                 * an audit log
                 */
                200: AuditLog;
            };
        };
    };
    '/w/{workspace}/audit/list': {
        get: {
            req: {
                /**
                 * filter on type of operation
                 */
                actionKind?: 'Create' | 'Update' | 'Delete' | 'Execute';
                /**
                 * filter on created after (exclusive) timestamp
                 */
                after?: string;
                /**
                 * filter on created before (exclusive) timestamp
                 */
                before?: string;
                /**
                 * filter on exact or prefix name of operation
                 */
                operation?: string;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * filter on exact or prefix name of resource
                 */
                resource?: string;
                /**
                 * filter on exact username of user
                 */
                username?: string;
                workspace: string;
            };
            res: {
                /**
                 * a list of audit logs
                 */
                200: Array<AuditLog>;
            };
        };
    };
    '/auth/login': {
        post: {
            req: {
                /**
                 * credentials
                 */
                requestBody: Login;
            };
            res: {
                /**
                 * Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
                 *
                 */
                200: string;
            };
        };
    };
    '/auth/logout': {
        post: {
            res: {
                /**
                 * clear cookies and clear token (if applicable)
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/users/{username}': {
        get: {
            req: {
                username: string;
                workspace: string;
            };
            res: {
                /**
                 * user created
                 */
                200: User;
            };
        };
    };
    '/w/{workspace}/users/update/{username}': {
        post: {
            req: {
                /**
                 * new user
                 */
                requestBody: EditWorkspaceUser;
                username: string;
                workspace: string;
            };
            res: {
                /**
                 * edited user
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/users/is_owner/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * is owner
                 */
                200: boolean;
            };
        };
    };
    '/users/setpassword': {
        post: {
            req: {
                /**
                 * set password
                 */
                requestBody: {
                    password: string;
                };
            };
            res: {
                /**
                 * password set
                 */
                200: string;
            };
        };
    };
    '/users/create': {
        post: {
            req: {
                /**
                 * user info
                 */
                requestBody: {
                    email: string;
                    password: string;
                    super_admin: boolean;
                    name?: string;
                    company?: string;
                };
            };
            res: {
                /**
                 * user created
                 */
                201: string;
            };
        };
    };
    '/users/update/{email}': {
        post: {
            req: {
                email: string;
                /**
                 * new user info
                 */
                requestBody: {
                    is_super_admin?: boolean;
                };
            };
            res: {
                /**
                 * user updated
                 */
                200: string;
            };
        };
    };
    '/users/username_info/{email}': {
        get: {
            req: {
                email: string;
            };
            res: {
                /**
                 * user renamed
                 */
                200: {
                    username: string;
                    workspace_usernames: Array<{
                        workspace_id: string;
                        username: string;
                    }>;
                };
            };
        };
    };
    '/users/rename/{email}': {
        post: {
            req: {
                email: string;
                /**
                 * new username
                 */
                requestBody: {
                    new_username: string;
                };
            };
            res: {
                /**
                 * user renamed
                 */
                200: string;
            };
        };
    };
    '/users/delete/{email}': {
        delete: {
            req: {
                email: string;
            };
            res: {
                /**
                 * user deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/users/delete/{username}': {
        delete: {
            req: {
                username: string;
                workspace: string;
            };
            res: {
                /**
                 * delete user
                 */
                200: string;
            };
        };
    };
    '/users/email': {
        get: {
            res: {
                /**
                 * user email
                 */
                200: string;
            };
        };
    };
    '/users/refresh_token': {
        get: {
            res: {
                /**
                 * free usage
                 */
                200: string;
            };
        };
    };
    '/users/tutorial_progress': {
        get: {
            res: {
                /**
                 * tutorial progress
                 */
                200: {
                    progress?: number;
                };
            };
        };
        post: {
            req: {
                /**
                 * progress update
                 */
                requestBody: {
                    progress?: number;
                };
            };
            res: {
                /**
                 * tutorial progress
                 */
                200: string;
            };
        };
    };
    '/users/leave_instance': {
        post: {
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/users/usage': {
        get: {
            res: {
                /**
                 * free usage
                 */
                200: number;
            };
        };
    };
    '/users/all_runnables': {
        get: {
            res: {
                /**
                 * free all runnables
                 */
                200: {
                    workspace: string;
                    endpoint_async: string;
                    endpoint_sync: string;
                    endpoint_openai_sync: string;
                    summary: string;
                    description?: string;
                    kind: string;
                };
            };
        };
    };
    '/users/whoami': {
        get: {
            res: {
                /**
                 * user email
                 */
                200: GlobalUserInfo;
            };
        };
    };
    '/users/list_invites': {
        get: {
            res: {
                /**
                 * list all workspace invites
                 */
                200: Array<WorkspaceInvite>;
            };
        };
    };
    '/w/{workspace}/users/whoami': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: User;
            };
        };
    };
    '/users/accept_invite': {
        post: {
            req: {
                /**
                 * accept invite
                 */
                requestBody: {
                    workspace_id: string;
                    username?: string;
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/users/decline_invite': {
        post: {
            req: {
                /**
                 * decline invite
                 */
                requestBody: {
                    workspace_id: string;
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/users/whois/{username}': {
        get: {
            req: {
                username: string;
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: User;
            };
        };
    };
    '/users/exists/{email}': {
        get: {
            req: {
                email: string;
            };
            res: {
                /**
                 * user
                 */
                200: boolean;
            };
        };
    };
    '/users/list_as_super_admin': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
            };
            res: {
                /**
                 * user
                 */
                200: Array<GlobalUserInfo>;
            };
        };
    };
    '/w/{workspace}/users/list': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: Array<User>;
            };
        };
    };
    '/w/{workspace}/users/list_usage': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: Array<UserUsage>;
            };
        };
    };
    '/w/{workspace}/users/list_usernames': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: Array<(string)>;
            };
        };
    };
    '/users/tokens/create': {
        post: {
            req: {
                /**
                 * new token
                 */
                requestBody: NewToken;
            };
            res: {
                /**
                 * token created
                 */
                201: string;
            };
        };
    };
    '/users/tokens/impersonate': {
        post: {
            req: {
                /**
                 * new token
                 */
                requestBody: NewTokenImpersonate;
            };
            res: {
                /**
                 * token created
                 */
                201: string;
            };
        };
    };
    '/users/tokens/delete/{token_prefix}': {
        delete: {
            req: {
                tokenPrefix: string;
            };
            res: {
                /**
                 * delete token
                 */
                200: string;
            };
        };
    };
    '/users/tokens/list': {
        get: {
            req: {
                excludeEphemeral?: boolean;
            };
            res: {
                /**
                 * truncated token
                 */
                200: Array<TruncatedToken>;
            };
        };
    };
    '/oauth/login_callback/{client_name}': {
        post: {
            req: {
                clientName: string;
                /**
                 * Partially filled script
                 */
                requestBody: {
                    code?: string;
                    state?: string;
                };
            };
            res: {
                /**
                 * Successfully authenticated. The session ID is returned in a cookie named `token` and as plaintext response. Preferred method of authorization is through the bearer token. The cookie is only for browser convenience.
                 *
                 */
                200: string;
            };
        };
    };
    '/workspaces/list': {
        get: {
            res: {
                /**
                 * all workspaces
                 */
                200: Array<Workspace>;
            };
        };
    };
    '/workspaces/allowed_domain_auto_invite': {
        get: {
            res: {
                /**
                 * domain allowed or not
                 */
                200: boolean;
            };
        };
    };
    '/workspaces/users': {
        get: {
            res: {
                /**
                 * workspace with associated username
                 */
                200: UserWorkspaceList;
            };
        };
    };
    '/workspaces/list_as_superadmin': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
            };
            res: {
                /**
                 * workspaces
                 */
                200: Array<Workspace>;
            };
        };
    };
    '/workspaces/create': {
        post: {
            req: {
                /**
                 * new token
                 */
                requestBody: CreateWorkspace;
            };
            res: {
                /**
                 * token created
                 */
                201: string;
            };
        };
    };
    '/workspaces/exists': {
        post: {
            req: {
                /**
                 * id of workspace
                 */
                requestBody: {
                    id: string;
                };
            };
            res: {
                /**
                 * status
                 */
                200: boolean;
            };
        };
    };
    '/workspaces/exists_username': {
        post: {
            req: {
                requestBody: {
                    id: string;
                    username: string;
                };
            };
            res: {
                /**
                 * status
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/workspaces/invite_user': {
        post: {
            req: {
                /**
                 * WorkspaceInvite
                 */
                requestBody: {
                    email: string;
                    is_admin: boolean;
                    operator: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/add_user': {
        post: {
            req: {
                /**
                 * WorkspaceInvite
                 */
                requestBody: {
                    email: string;
                    is_admin: boolean;
                    username?: string;
                    operator: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/delete_invite': {
        post: {
            req: {
                /**
                 * WorkspaceInvite
                 */
                requestBody: {
                    email: string;
                    is_admin: boolean;
                    operator: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/archive': {
        post: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/workspaces/unarchive/{workspace}': {
        post: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/workspaces/delete/{workspace}': {
        delete: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/leave': {
        post: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/get_workspace_name': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/change_workspace_name': {
        post: {
            req: {
                requestBody?: {
                    new_name?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/change_workspace_id': {
        post: {
            req: {
                requestBody?: {
                    new_id?: string;
                    new_name?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/list_pending_invites': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * user
                 */
                200: Array<WorkspaceInvite>;
            };
        };
    };
    '/w/{workspace}/workspaces/get_settings': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    workspace_id?: string;
                    slack_name?: string;
                    slack_team_id?: string;
                    slack_command_script?: string;
                    auto_invite_domain?: string;
                    auto_invite_operator?: boolean;
                    auto_add?: boolean;
                    plan?: string;
                    automatic_billing: boolean;
                    customer_id?: string;
                    webhook?: string;
                    deploy_to?: string;
                    openai_resource_path?: string;
                    code_completion_enabled: boolean;
                    error_handler?: string;
                    error_handler_extra_args?: ScriptArgs;
                    error_handler_muted_on_cancel: boolean;
                    large_file_storage?: LargeFileStorage;
                    git_sync?: WorkspaceGitSyncSettings;
                    default_app?: string;
                    default_scripts?: WorkspaceDefaultScripts;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/get_deploy_to': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    deploy_to?: string;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/is_premium': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/workspaces/premium_info': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    premium: boolean;
                    usage?: number;
                    seats?: number;
                    automatic_billing: boolean;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/set_automatic_billing': {
        post: {
            req: {
                /**
                 * automatic billing
                 */
                requestBody: {
                    automatic_billing: boolean;
                    seats?: number;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_slack_command': {
        post: {
            req: {
                /**
                 * WorkspaceInvite
                 */
                requestBody: {
                    slack_command_script?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/run_slack_message_test_job': {
        post: {
            req: {
                /**
                 * path to hub script to run and its corresponding args
                 */
                requestBody: {
                    hub_script_path?: string;
                    channel?: string;
                    test_msg?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    job_uuid?: string;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/edit_deploy_to': {
        post: {
            req: {
                requestBody: {
                    deploy_to?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_auto_invite': {
        post: {
            req: {
                /**
                 * WorkspaceInvite
                 */
                requestBody: {
                    operator?: boolean;
                    invite_all?: boolean;
                    auto_add?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_webhook': {
        post: {
            req: {
                /**
                 * WorkspaceWebhook
                 */
                requestBody: {
                    webhook?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_copilot_config': {
        post: {
            req: {
                /**
                 * WorkspaceCopilotConfig
                 */
                requestBody: {
                    openai_resource_path?: string;
                    code_completion_enabled: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/get_copilot_info': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    exists_openai_resource_path: boolean;
                    code_completion_enabled: boolean;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/edit_error_handler': {
        post: {
            req: {
                /**
                 * WorkspaceErrorHandler
                 */
                requestBody: {
                    error_handler?: string;
                    error_handler_extra_args?: ScriptArgs;
                    error_handler_muted_on_cancel?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_large_file_storage_config': {
        post: {
            req: {
                /**
                 * LargeFileStorage info
                 */
                requestBody: {
                    large_file_storage?: LargeFileStorage;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_git_sync_config': {
        post: {
            req: {
                /**
                 * Workspace Git sync settings
                 */
                requestBody: {
                    git_sync_settings?: WorkspaceGitSyncSettings;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/workspaces/edit_default_app': {
        post: {
            req: {
                /**
                 * Workspace default app
                 */
                requestBody: {
                    default_app_path?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/default_scripts': {
        post: {
            req: {
                /**
                 * Workspace default app
                 */
                requestBody?: WorkspaceDefaultScripts;
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: WorkspaceDefaultScripts;
            };
        };
    };
    '/w/{workspace}/workspaces/set_environment_variable': {
        post: {
            req: {
                /**
                 * Workspace default app
                 */
                requestBody: {
                    name: string;
                    value?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/encryption_key': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    key: string;
                };
            };
        };
        post: {
            req: {
                /**
                 * New encryption key
                 */
                requestBody: {
                    new_key: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/workspaces/default_app': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: {
                    default_app_path?: string;
                };
            };
        };
    };
    '/w/{workspace}/workspaces/get_large_file_storage_config': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * status
                 */
                200: LargeFileStorage;
            };
        };
    };
    '/w/{workspace}/workspaces/usage': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * usage
                 */
                200: number;
            };
        };
    };
    '/settings/global/{key}': {
        get: {
            req: {
                key: string;
            };
            res: {
                /**
                 * status
                 */
                200: unknown;
            };
        };
        post: {
            req: {
                key: string;
                /**
                 * value set
                 */
                requestBody: {
                    value?: unknown;
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/settings/local': {
        get: {
            res: {
                /**
                 * status
                 */
                200: unknown;
            };
        };
    };
    '/settings/test_smtp': {
        post: {
            req: {
                /**
                 * test smtp payload
                 */
                requestBody: {
                    to: string;
                    smtp: {
                        host: string;
                        username: string;
                        password: string;
                        port: number;
                        from: string;
                        tls_implicit: boolean;
                    };
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/settings/test_license_key': {
        post: {
            req: {
                /**
                 * test license key
                 */
                requestBody: {
                    license_key: string;
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/settings/test_object_storage_config': {
        post: {
            req: {
                /**
                 * test object storage config
                 */
                requestBody: {
                    [key: string]: unknown;
                };
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/settings/send_stats': {
        post: {
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/saml/test_metadata': {
        post: {
            req: {
                /**
                 * test metadata
                 */
                requestBody: string;
            };
            res: {
                /**
                 * status
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/oidc/token/{audience}': {
        post: {
            req: {
                audience: string;
                workspace: string;
            };
            res: {
                /**
                 * new oidc token
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/variables/create': {
        post: {
            req: {
                alreadyEncrypted?: boolean;
                /**
                 * new variable
                 */
                requestBody: CreateVariable;
                workspace: string;
            };
            res: {
                /**
                 * variable created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/variables/encrypt': {
        post: {
            req: {
                /**
                 * new variable
                 */
                requestBody: string;
                workspace: string;
            };
            res: {
                /**
                 * encrypted value
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/variables/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * variable deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/variables/update/{path}': {
        post: {
            req: {
                alreadyEncrypted?: boolean;
                path: string;
                /**
                 * updated variable
                 */
                requestBody: EditVariable;
                workspace: string;
            };
            res: {
                /**
                 * variable updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/variables/get/{path}': {
        get: {
            req: {
                /**
                 * ask to decrypt secret if this variable is secret
                 * (if not secret no effect, default: true)
                 *
                 */
                decryptSecret?: boolean;
                /**
                 * ask to include the encrypted value if secret and decrypt secret is not true (default: false)
                 *
                 */
                includeEncrypted?: boolean;
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * variable
                 */
                200: ListableVariable;
            };
        };
    };
    '/w/{workspace}/variables/get_value/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * variable
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/variables/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * variable
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/variables/list': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * variable list
                 */
                200: Array<ListableVariable>;
            };
        };
    };
    '/w/{workspace}/variables/list_contextual': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * contextual variable list
                 */
                200: Array<ContextualVariable>;
            };
        };
    };
    '/w/{workspace}/oauth/connect_slack_callback': {
        post: {
            req: {
                /**
                 * code endpoint
                 */
                requestBody: {
                    code: string;
                    state: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * slack token
                 */
                200: string;
            };
        };
    };
    '/oauth/connect_callback/{client_name}': {
        post: {
            req: {
                clientName: string;
                /**
                 * code endpoint
                 */
                requestBody: {
                    code: string;
                    state: string;
                };
            };
            res: {
                /**
                 * oauth token
                 */
                200: TokenResponse;
            };
        };
    };
    '/w/{workspace}/oauth/create_account': {
        post: {
            req: {
                /**
                 * code endpoint
                 */
                requestBody: {
                    refresh_token?: string;
                    expires_in: number;
                    client: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * account set
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/oauth/refresh_token/{id}': {
        post: {
            req: {
                id: number;
                /**
                 * variable path
                 */
                requestBody: {
                    path: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * token refreshed
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/oauth/disconnect/{id}': {
        post: {
            req: {
                id: number;
                workspace: string;
            };
            res: {
                /**
                 * disconnected client
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/oauth/disconnect_slack': {
        post: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * disconnected slack
                 */
                200: string;
            };
        };
    };
    '/oauth/list_logins': {
        get: {
            res: {
                /**
                 * list of oauth and saml login clients
                 */
                200: {
                    oauth: Array<(string)>;
                    saml?: string;
                };
            };
        };
    };
    '/oauth/list_connects': {
        get: {
            res: {
                /**
                 * list of oauth connects clients
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/resources/create': {
        post: {
            req: {
                /**
                 * new resource
                 */
                requestBody: CreateResource;
                updateIfExists?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * resource created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/resources/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/resources/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updated resource
                 */
                requestBody: EditResource;
                workspace: string;
            };
            res: {
                /**
                 * resource updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/resources/update_value/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updated resource
                 */
                requestBody: {
                    value?: unknown;
                };
                workspace: string;
            };
            res: {
                /**
                 * resource value updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/resources/get/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource
                 */
                200: Resource;
            };
        };
    };
    '/w/{workspace}/resources/get_value_interpolated/{path}': {
        get: {
            req: {
                /**
                 * job id
                 */
                jobId?: string;
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource value
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/resources/get_value/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource value
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/resources/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * does resource exists
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/resources/list': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * resource_types to list from, separated by ',',
                 */
                resourceType?: string;
                /**
                 * resource_types to not list from, separated by ',',
                 */
                resourceTypeExclude?: string;
                workspace: string;
            };
            res: {
                /**
                 * resource list
                 */
                200: Array<ListableResource>;
            };
        };
    };
    '/w/{workspace}/resources/list_search': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * resource list
                 */
                200: Array<{
                    path: string;
                    value: unknown;
                }>;
            };
        };
    };
    '/w/{workspace}/resources/list_names/{name}': {
        get: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * resource list names
                 */
                200: Array<{
                    name: string;
                    path: string;
                }>;
            };
        };
    };
    '/w/{workspace}/resources/type/create': {
        post: {
            req: {
                /**
                 * new resource_type
                 */
                requestBody: ResourceType;
                workspace: string;
            };
            res: {
                /**
                 * resource_type created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/resources/type/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource_type deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/resources/type/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updated resource_type
                 */
                requestBody: EditResourceType;
                workspace: string;
            };
            res: {
                /**
                 * resource_type updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/resources/type/get/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource_type deleted
                 */
                200: ResourceType;
            };
        };
    };
    '/w/{workspace}/resources/type/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * does resource_type exist
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/resources/type/list': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * resource_type list
                 */
                200: Array<ResourceType>;
            };
        };
    };
    '/w/{workspace}/resources/type/listnames': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * resource_type list
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/embeddings/query_resource_types': {
        get: {
            req: {
                /**
                 * query limit
                 */
                limit?: number;
                /**
                 * query text
                 */
                text: string;
                workspace: string;
            };
            res: {
                /**
                 * resource type details
                 */
                200: Array<{
                    name: string;
                    score: number;
                    schema?: unknown;
                }>;
            };
        };
    };
    '/integrations/hub/list': {
        get: {
            req: {
                /**
                 * query integrations kind
                 */
                kind?: string;
            };
            res: {
                /**
                 * integrations details
                 */
                200: Array<{
                    name: string;
                }>;
            };
        };
    };
    '/flows/hub/list': {
        get: {
            res: {
                /**
                 * hub flows list
                 */
                200: {
                    flows?: Array<{
                        id: number;
                        flow_id: number;
                        summary: string;
                        apps: Array<(string)>;
                        approved: boolean;
                        votes: number;
                    }>;
                };
            };
        };
    };
    '/flows/hub/get/{id}': {
        get: {
            req: {
                id: number;
            };
            res: {
                /**
                 * flow
                 */
                200: {
                    flow?: OpenFlow;
                };
            };
        };
    };
    '/w/{workspace}/flows/list_paths': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * list of flow paths
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/flows/list_search': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * flow list
                 */
                200: Array<{
                    path: string;
                    value: unknown;
                }>;
            };
        };
    };
    '/w/{workspace}/flows/list': {
        get: {
            req: {
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * mask to filter exact matching path
                 */
                pathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                pathStart?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * (default false)
                 * show also the archived files.
                 * when multiple archived hash share the same path, only the ones with the latest create_at
                 * are displayed.
                 *
                 */
                showArchived?: boolean;
                /**
                 * (default false)
                 * show only the starred items
                 *
                 */
                starredOnly?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * All flow
                 */
                200: Array<(Flow & {
                    has_draft?: boolean;
                    draft_only?: boolean;
                })>;
            };
        };
    };
    '/w/{workspace}/flows/get/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow details
                 */
                200: Flow;
            };
        };
    };
    '/w/{workspace}/flows/toggle_workspace_error_handler/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * Workspace error handler enabled
                 */
                requestBody: {
                    muted?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * error handler toggled
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/flows/get/draft/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow details with draft
                 */
                200: Flow & {
                    draft?: Flow;
                };
            };
        };
    };
    '/w/{workspace}/flows/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow details
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/flows/create': {
        post: {
            req: {
                /**
                 * Partially filled flow
                 */
                requestBody: OpenFlowWPath & {
                    draft_only?: boolean;
                    deployment_message?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * flow created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/flows/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * Partially filled flow
                 */
                requestBody: OpenFlowWPath & {
                    deployment_message?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * flow updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/flows/archive/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * archiveFlow
                 */
                requestBody: {
                    archived?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * flow archived
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/flows/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow delete
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/flows/input_history/p/{path}': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                path: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                workspace: string;
            };
            res: {
                /**
                 * input history for completed jobs with this flow path
                 */
                200: Array<Input>;
            };
        };
    };
    '/apps/hub/list': {
        get: {
            res: {
                /**
                 * hub apps list
                 */
                200: {
                    apps?: Array<{
                        id: number;
                        app_id: number;
                        summary: string;
                        apps: Array<(string)>;
                        approved: boolean;
                        votes: number;
                    }>;
                };
            };
        };
    };
    '/apps/hub/get/{id}': {
        get: {
            req: {
                id: number;
            };
            res: {
                /**
                 * app
                 */
                200: {
                    app: {
                        summary: string;
                        value: unknown;
                    };
                };
            };
        };
    };
    '/w/{workspace}/apps/list_search': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * app list
                 */
                200: Array<{
                    path: string;
                    value: unknown;
                }>;
            };
        };
    };
    '/w/{workspace}/apps/list': {
        get: {
            req: {
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * mask to filter exact matching path
                 */
                pathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                pathStart?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * (default false)
                 * show only the starred items
                 *
                 */
                starredOnly?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * All apps
                 */
                200: Array<ListableApp>;
            };
        };
    };
    '/w/{workspace}/apps/create': {
        post: {
            req: {
                /**
                 * new app
                 */
                requestBody: {
                    path: string;
                    value: unknown;
                    summary: string;
                    policy: Policy;
                    draft_only?: boolean;
                    deployment_message?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * app created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/apps/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app exists
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/apps/get/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app details
                 */
                200: AppWithLastVersion;
            };
        };
    };
    '/w/{workspace}/apps/get/draft/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app details with draft
                 */
                200: AppWithLastVersionWDraft;
            };
        };
    };
    '/w/{workspace}/apps/history/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app history
                 */
                200: Array<AppHistory>;
            };
        };
    };
    '/w/{workspace}/apps/history_update/a/{id}/v/{version}': {
        post: {
            req: {
                id: number;
                /**
                 * App deployment message
                 */
                requestBody: {
                    deployment_msg?: string;
                };
                version: number;
                workspace: string;
            };
            res: {
                /**
                 * success
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/apps_u/public_app/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app details
                 */
                200: AppWithLastVersion;
            };
        };
    };
    '/w/{workspace}/apps_u/public_resource/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * resource value
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/apps/secret_of/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app secret
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/apps/get/v/{id}': {
        get: {
            req: {
                id: number;
                workspace: string;
            };
            res: {
                /**
                 * app details
                 */
                200: AppWithLastVersion;
            };
        };
    };
    '/w/{workspace}/apps/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/apps/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * update app
                 */
                requestBody: {
                    path?: string;
                    summary?: string;
                    value?: unknown;
                    policy?: Policy;
                    deployment_message?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * app updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/apps_u/execute_component/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * update app
                 */
                requestBody: {
                    component: string;
                    path?: string;
                    args: unknown;
                    raw_code?: {
                        content: string;
                        language: string;
                        path?: string;
                        cache_ttl?: number;
                    };
                    force_viewer_static_fields?: {
                        [key: string]: unknown;
                    };
                    force_viewer_one_of_fields?: {
                        [key: string]: unknown;
                    };
                };
                workspace: string;
            };
            res: {
                /**
                 * job uuid
                 */
                200: string;
            };
        };
    };
    '/scripts/hub/get/{path}': {
        get: {
            req: {
                path: string;
            };
            res: {
                /**
                 * script details
                 */
                200: string;
            };
        };
    };
    '/scripts/hub/get_full/{path}': {
        get: {
            req: {
                path: string;
            };
            res: {
                /**
                 * script details
                 */
                200: {
                    content: string;
                    lockfile?: string;
                    schema?: unknown;
                    language: string;
                    summary?: string;
                };
            };
        };
    };
    '/scripts/hub/top': {
        get: {
            req: {
                /**
                 * query scripts app
                 */
                app?: string;
                /**
                 * query scripts kind
                 */
                kind?: string;
                /**
                 * query limit
                 */
                limit?: number;
            };
            res: {
                /**
                 * hub scripts list
                 */
                200: {
                    asks?: Array<{
                        id: number;
                        ask_id: number;
                        summary: string;
                        app: string;
                        version_id: number;
                        kind: HubScriptKind;
                        votes: number;
                        views: number;
                    }>;
                };
            };
        };
    };
    '/embeddings/query_hub_scripts': {
        get: {
            req: {
                /**
                 * query scripts app
                 */
                app?: string;
                /**
                 * query scripts kind
                 */
                kind?: string;
                /**
                 * query limit
                 */
                limit?: number;
                /**
                 * query text
                 */
                text: string;
            };
            res: {
                /**
                 * script details
                 */
                200: Array<{
                    ask_id: number;
                    id: number;
                    version_id: number;
                    summary: string;
                    app: string;
                    kind: HubScriptKind;
                    score: number;
                }>;
            };
        };
    };
    '/w/{workspace}/scripts/list_search': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * script list
                 */
                200: Array<{
                    path: string;
                    content: string;
                }>;
            };
        };
    };
    '/w/{workspace}/scripts/list': {
        get: {
            req: {
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * mask to filter scripts whom first direct parent has exact hash
                 */
                firstParentHash?: string;
                /**
                 * (default false)
                 * hide the scripts without an exported main function
                 *
                 */
                hideWithoutMain?: boolean;
                /**
                 * (default regardless)
                 * if true show only the templates
                 * if false show only the non templates
                 * if not defined, show all regardless of if the script is a template
                 *
                 */
                isTemplate?: boolean;
                /**
                 * (default regardless)
                 * script kinds to filter, split by comma
                 *
                 */
                kinds?: string;
                /**
                 * mask to filter scripts whom last parent in the chain has exact hash.
                 * Beware that each script stores only a limited number of parents. Hence
                 * the last parent hash for a script is not necessarily its top-most parent.
                 * To find the top-most parent you will have to jump from last to last hash
                 * until finding the parent
                 *
                 */
                lastParentHash?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * is the hash present in the array of stored parent hashes for this script.
                 * The same warning applies than for last_parent_hash. A script only store a
                 * limited number of direct parent
                 *
                 */
                parentHash?: string;
                /**
                 * mask to filter exact matching path
                 */
                pathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                pathStart?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * (default false)
                 * show also the archived files.
                 * when multiple archived hash share the same path, only the ones with the latest create_at
                 * are
                 * ed.
                 *
                 */
                showArchived?: boolean;
                /**
                 * (default false)
                 * show only the starred items
                 *
                 */
                starredOnly?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * All scripts
                 */
                200: Array<Script>;
            };
        };
    };
    '/w/{workspace}/scripts/list_paths': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * list of script paths
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/scripts/create': {
        post: {
            req: {
                /**
                 * Partially filled script
                 */
                requestBody: NewScript;
                workspace: string;
            };
            res: {
                /**
                 * script created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/scripts/toggle_workspace_error_handler/p/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * Workspace error handler enabled
                 */
                requestBody: {
                    muted?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * error handler toggled
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/archive/p/{path}': {
        post: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script archived
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/archive/h/{hash}': {
        post: {
            req: {
                hash: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: Script;
            };
        };
    };
    '/w/{workspace}/scripts/delete/h/{hash}': {
        post: {
            req: {
                hash: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: Script;
            };
        };
    };
    '/w/{workspace}/scripts/delete/p/{path}': {
        post: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script path
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/get/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: Script;
            };
        };
    };
    '/w/{workspace}/scripts/get/draft/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: NewScriptWithDraft;
            };
        };
    };
    '/w/{workspace}/scripts/history/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script history
                 */
                200: Array<ScriptHistory>;
            };
        };
    };
    '/w/{workspace}/scripts/history_update/h/{hash}/p/{path}': {
        post: {
            req: {
                hash: string;
                path: string;
                /**
                 * Script deployment message
                 */
                requestBody: {
                    deployment_msg?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * success
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/raw/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script content
                 */
                200: string;
            };
        };
    };
    '/scripts_u/tokened_raw/{workspace}/{token}/{path}': {
        get: {
            req: {
                path: string;
                token: string;
                workspace: string;
            };
            res: {
                /**
                 * script content
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/exists/p/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * does it exists
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/scripts/get/h/{hash}': {
        get: {
            req: {
                hash: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: Script;
            };
        };
    };
    '/w/{workspace}/scripts/raw/h/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * script content
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/scripts/deployment_status/h/{hash}': {
        get: {
            req: {
                hash: string;
                workspace: string;
            };
            res: {
                /**
                 * script details
                 */
                200: {
                    lock?: string;
                    lock_error_logs?: string;
                };
            };
        };
    };
    '/w/{workspace}/drafts/create': {
        post: {
            req: {
                requestBody: {
                    path: string;
                    typ: 'flow' | 'script' | 'app';
                    value?: unknown;
                };
                workspace: string;
            };
            res: {
                /**
                 * draft created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/drafts/delete/{kind}/{path}': {
        delete: {
            req: {
                kind: 'script' | 'flow' | 'app';
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * draft deleted
                 */
                200: string;
            };
        };
    };
    '/workers/custom_tags': {
        get: {
            res: {
                /**
                 * list of custom tags
                 */
                200: Array<(string)>;
            };
        };
    };
    '/workers/get_default_tags': {
        get: {
            res: {
                /**
                 * list of default tags
                 */
                200: Array<(string)>;
            };
        };
    };
    '/workers/is_default_tags_per_workspace': {
        get: {
            res: {
                /**
                 * is the default tags per workspace
                 */
                200: boolean;
            };
        };
    };
    '/workers/list': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * number of seconds the worker must have had a last ping more recent of (default to 300)
                 */
                pingSince?: number;
            };
            res: {
                /**
                 * a list of workers
                 */
                200: Array<WorkerPing>;
            };
        };
    };
    '/workers/exists_worker_with_tag': {
        get: {
            req: {
                tag: string;
            };
            res: {
                /**
                 * whether a worker with the tag exists
                 */
                200: boolean;
            };
        };
    };
    '/workers/queue_metrics': {
        get: {
            res: {
                /**
                 * metrics
                 */
                200: Array<{
                    id: string;
                    values: Array<{
                        created_at: string;
                        value: number;
                    }>;
                }>;
            };
        };
    };
    '/w/{workspace}/jobs/run/p/{path}': {
        post: {
            req: {
                /**
                 * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
                 */
                cacheTtl?: string;
                /**
                 * make the run invisible to the the script owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                path: string;
                /**
                 * script args
                 */
                requestBody: ScriptArgs;
                /**
                 * when to schedule this job (leave empty for immediate run)
                 */
                scheduledFor?: string;
                /**
                 * schedule the script to execute in the number of seconds starting now
                 */
                scheduledInSecs?: number;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/openai_sync/p/{path}': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                path: string;
                /**
                 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
                 *
                 */
                queueLimit?: string;
                /**
                 * script args
                 */
                requestBody: ScriptArgs;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/run_wait_result/p/{path}': {
        post: {
            req: {
                /**
                 * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
                 */
                cacheTtl?: string;
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                path: string;
                /**
                 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
                 *
                 */
                queueLimit?: string;
                /**
                 * script args
                 */
                requestBody: ScriptArgs;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
        get: {
            req: {
                /**
                 * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
                 */
                cacheTtl?: string;
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                path: string;
                /**
                 * The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
                 * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
                 *
                 */
                payload?: string;
                /**
                 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
                 *
                 */
                queueLimit?: string;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/openai_sync/f/{path}': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                path: string;
                /**
                 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
                 *
                 */
                queueLimit?: string;
                /**
                 * script args
                 */
                requestBody: ScriptArgs;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/run_wait_result/f/{path}': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                path: string;
                /**
                 * The maximum size of the queue for which the request would get rejected if that job would push it above that limit
                 *
                 */
                queueLimit?: string;
                /**
                 * script args
                 */
                requestBody: ScriptArgs;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/result_by_id/{flow_job_id}/{node_id}': {
        get: {
            req: {
                flowJobId: string;
                nodeId: string;
                workspace: string;
            };
            res: {
                /**
                 * job result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/run/f/{path}': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * make the run invisible to the the flow owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                path: string;
                /**
                 * flow args
                 */
                requestBody: ScriptArgs;
                /**
                 * when to schedule this job (leave empty for immediate run)
                 */
                scheduledFor?: string;
                /**
                 * schedule the script to execute in the number of seconds starting now
                 */
                scheduledInSecs?: number;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/restart/f/{id}/from/{step_id}/{branch_or_iteration_n}': {
        post: {
            req: {
                /**
                 * for branchall or loop, the iteration at which the flow should restart
                 */
                branchOrIterationN: number;
                id: string;
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * make the run invisible to the the flow owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                /**
                 * flow args
                 */
                requestBody: ScriptArgs;
                /**
                 * when to schedule this job (leave empty for immediate run)
                 */
                scheduledFor?: string;
                /**
                 * schedule the script to execute in the number of seconds starting now
                 */
                scheduledInSecs?: number;
                /**
                 * step id to restart the flow from
                 */
                stepId: string;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/run/h/{hash}': {
        post: {
            req: {
                /**
                 * Override the cache time to live (in seconds). Can not be used to disable caching, only override with a new cache ttl
                 */
                cacheTtl?: string;
                hash: string;
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * make the run invisible to the the script owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                /**
                 * Partially filled args
                 */
                requestBody: {
                    [key: string]: unknown;
                };
                /**
                 * when to schedule this job (leave empty for immediate run)
                 */
                scheduledFor?: string;
                /**
                 * schedule the script to execute in the number of seconds starting now
                 */
                scheduledInSecs?: number;
                /**
                 * Override the tag to use
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/run/preview': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * make the run invisible to the the script owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * preview
                 */
                requestBody: Preview;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/workflow_as_code/{job_id}/{entrypoint}': {
        post: {
            req: {
                entrypoint: string;
                jobId: string;
                /**
                 * preview
                 */
                requestBody: WorkflowTask;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/run/dependencies': {
        post: {
            req: {
                /**
                 * raw script content
                 */
                requestBody: {
                    raw_scripts: Array<RawScriptForDependencies>;
                    entrypoint: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * dependency job result
                 */
                201: {
                    lock: string;
                };
            };
        };
    };
    '/w/{workspace}/jobs/run/preview_flow': {
        post: {
            req: {
                /**
                 * List of headers's keys (separated with ',') whove value are added to the args
                 * Header's key lowercased and '-'' replaced to '_' such that 'Content-Type' becomes the 'content_type' arg key
                 *
                 */
                includeHeader?: string;
                /**
                 * make the run invisible to the the script owner (default false)
                 */
                invisibleToOwner?: boolean;
                /**
                 * The job id to assign to the created job. if missing, job is chosen randomly using the ULID scheme. If a job id already exists in the queue or as a completed job, the request to create one will fail (Bad Request)
                 */
                jobId?: string;
                /**
                 * preview
                 */
                requestBody: FlowPreview;
                workspace: string;
            };
            res: {
                /**
                 * job created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/queue/list': {
        get: {
            req: {
                /**
                 * get jobs from all workspaces (only valid if request come from the `admins` workspace)
                 */
                allWorkspaces?: boolean;
                /**
                 * filter on jobs containing those args as a json subset (@> in postgres)
                 */
                args?: string;
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * is not a scheduled job
                 */
                isNotSchedule?: boolean;
                /**
                 * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
                 */
                jobKinds?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * filter on jobs containing those result as a json subset (@> in postgres)
                 */
                result?: string;
                /**
                 * filter on running jobs
                 */
                running?: boolean;
                /**
                 * filter on jobs scheduled_for before now (hence waitinf for a worker)
                 */
                scheduledForBeforeNow?: boolean;
                /**
                 * mask to filter by schedule path
                 */
                schedulePath?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptHash?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptPathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                scriptPathStart?: string;
                /**
                 * filter on started after (exclusive) timestamp
                 */
                startedAfter?: string;
                /**
                 * filter on started before (inclusive) timestamp
                 */
                startedBefore?: string;
                /**
                 * filter on successful jobs
                 */
                success?: boolean;
                /**
                 * filter on suspended jobs
                 */
                suspended?: boolean;
                /**
                 * filter on jobs with a given tag/worker group
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * All queued jobs
                 */
                200: Array<QueuedJob>;
            };
        };
    };
    '/w/{workspace}/jobs/queue/count': {
        get: {
            req: {
                /**
                 * get jobs from all workspaces (only valid if request come from the `admins` workspace)
                 */
                allWorkspaces?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * queue count
                 */
                200: {
                    database_length: number;
                };
            };
        };
    };
    '/w/{workspace}/jobs/completed/count': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * completed count
                 */
                200: {
                    database_length: number;
                };
            };
        };
    };
    '/w/{workspace}/jobs/queue/cancel_all': {
        post: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * uuids of canceled jobs
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/jobs/completed/list': {
        get: {
            req: {
                /**
                 * filter on jobs containing those args as a json subset (@> in postgres)
                 */
                args?: string;
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * has null parent
                 */
                hasNullParent?: boolean;
                /**
                 * is the job a flow step
                 */
                isFlowStep?: boolean;
                /**
                 * is not a scheduled job
                 */
                isNotSchedule?: boolean;
                /**
                 * is the job skipped
                 */
                isSkipped?: boolean;
                /**
                 * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
                 */
                jobKinds?: string;
                /**
                 * mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
                 */
                label?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * filter on jobs containing those result as a json subset (@> in postgres)
                 */
                result?: string;
                /**
                 * mask to filter by schedule path
                 */
                schedulePath?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptHash?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptPathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                scriptPathStart?: string;
                /**
                 * filter on started after (exclusive) timestamp
                 */
                startedAfter?: string;
                /**
                 * filter on started before (inclusive) timestamp
                 */
                startedBefore?: string;
                /**
                 * filter on successful jobs
                 */
                success?: boolean;
                /**
                 * filter on jobs with a given tag/worker group
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * All completed jobs
                 */
                200: Array<CompletedJob>;
            };
        };
    };
    '/w/{workspace}/jobs/list': {
        get: {
            req: {
                /**
                 * get jobs from all workspaces (only valid if request come from the `admins` workspace)
                 */
                allWorkspaces?: boolean;
                /**
                 * filter on jobs containing those args as a json subset (@> in postgres)
                 */
                args?: string;
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * filter on created_at for non non started job and started_at otherwise after (exclusive) timestamp
                 */
                createdOrStartedAfter?: string;
                /**
                 * filter on created_at for non non started job and started_at otherwise before (inclusive) timestamp
                 */
                createdOrStartedBefore?: string;
                /**
                 * has null parent
                 */
                hasNullParent?: boolean;
                /**
                 * is the job a flow step
                 */
                isFlowStep?: boolean;
                /**
                 * is not a scheduled job
                 */
                isNotSchedule?: boolean;
                /**
                 * is the job skipped
                 */
                isSkipped?: boolean;
                /**
                 * filter on job kind (values 'preview', 'script', 'dependencies', 'flow') separated by,
                 */
                jobKinds?: string;
                /**
                 * mask to filter exact matching job's label (job labels are completed jobs with as a result an object containing a string in the array at key 'wm_labels')
                 */
                label?: string;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * The parent job that is at the origin and responsible for the execution of this script if any
                 */
                parentJob?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * filter on jobs containing those result as a json subset (@> in postgres)
                 */
                result?: string;
                /**
                 * filter on running jobs
                 */
                running?: boolean;
                /**
                 * filter on jobs scheduled_for before now (hence waitinf for a worker)
                 */
                scheduledForBeforeNow?: boolean;
                /**
                 * mask to filter by schedule path
                 */
                schedulePath?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptHash?: string;
                /**
                 * mask to filter exact matching path
                 */
                scriptPathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                scriptPathStart?: string;
                /**
                 * filter on started after (exclusive) timestamp
                 */
                startedAfter?: string;
                /**
                 * filter on started before (inclusive) timestamp
                 */
                startedBefore?: string;
                /**
                 * filter on successful jobs
                 */
                success?: boolean;
                /**
                 * filter on jobs with a given tag/worker group
                 */
                tag?: string;
                workspace: string;
            };
            res: {
                /**
                 * All jobs
                 */
                200: Array<Job>;
            };
        };
    };
    '/jobs/db_clock': {
        get: {
            res: {
                /**
                 * the timestamp of the db that can be used to compute the drift
                 */
                200: number;
            };
        };
    };
    '/w/{workspace}/jobs_u/get/{id}': {
        get: {
            req: {
                id: string;
                noLogs?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: Job;
            };
        };
    };
    '/w/{workspace}/jobs_u/get_root_job_id/{id}': {
        get: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * get root job id
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/get_logs/{id}': {
        get: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/getupdate/{id}': {
        get: {
            req: {
                id: string;
                logOffset?: number;
                running?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: {
                    running?: boolean;
                    completed?: boolean;
                    new_logs?: string;
                    log_offset?: number;
                    mem_peak?: number;
                    flow_status?: WorkflowStatusRecord;
                };
            };
        };
    };
    '/w/{workspace}/jobs_u/get_log_file/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * job log
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs_u/get_flow_debug_info/{id}': {
        get: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * flow debug info details
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs_u/completed/get/{id}': {
        get: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: CompletedJob;
            };
        };
    };
    '/w/{workspace}/jobs_u/completed/get_result/{id}': {
        get: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * result
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs_u/completed/get_result_maybe/{id}': {
        get: {
            req: {
                getStarted?: boolean;
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * result
                 */
                200: {
                    completed: boolean;
                    result: unknown;
                    success?: boolean;
                    started?: boolean;
                };
            };
        };
    };
    '/w/{workspace}/jobs/completed/delete/{id}': {
        post: {
            req: {
                id: string;
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: CompletedJob;
            };
        };
    };
    '/w/{workspace}/jobs_u/queue/cancel/{id}': {
        post: {
            req: {
                id: string;
                /**
                 * reason
                 */
                requestBody: {
                    reason?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * job canceled
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/queue/cancel_persistent/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * reason
                 */
                requestBody: {
                    reason?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * persistent job scaled down to zero
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/queue/force_cancel/{id}': {
        post: {
            req: {
                id: string;
                /**
                 * reason
                 */
                requestBody: {
                    reason?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * job canceled
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs/job_signature/{id}/{resume_id}': {
        get: {
            req: {
                approver?: string;
                id: string;
                resumeId: number;
                workspace: string;
            };
            res: {
                /**
                 * job signature
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/jobs/resume_urls/{id}/{resume_id}': {
        get: {
            req: {
                approver?: string;
                id: string;
                resumeId: number;
                workspace: string;
            };
            res: {
                /**
                 * url endpoints
                 */
                200: {
                    approvalPage: string;
                    resume: string;
                    cancel: string;
                };
            };
        };
    };
    '/w/{workspace}/jobs_u/resume/{id}/{resume_id}/{signature}': {
        get: {
            req: {
                approver?: string;
                id: string;
                /**
                 * The base64 encoded payload that has been encoded as a JSON. e.g how to encode such payload encodeURIComponent
                 * `encodeURIComponent(btoa(JSON.stringify({a: 2})))`
                 *
                 */
                payload?: string;
                resumeId: number;
                signature: string;
                workspace: string;
            };
            res: {
                /**
                 * job resumed
                 */
                201: string;
            };
        };
        post: {
            req: {
                approver?: string;
                id: string;
                requestBody: {
                    [key: string]: unknown;
                };
                resumeId: number;
                signature: string;
                workspace: string;
            };
            res: {
                /**
                 * job resumed
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs/flow/user_states/{id}/{key}': {
        post: {
            req: {
                id: string;
                key: string;
                /**
                 * new value
                 */
                requestBody: unknown;
                workspace: string;
            };
            res: {
                /**
                 * flow user state updated
                 */
                200: string;
            };
        };
        get: {
            req: {
                id: string;
                key: string;
                workspace: string;
            };
            res: {
                /**
                 * flow user state updated
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/jobs/flow/resume/{id}': {
        post: {
            req: {
                id: string;
                requestBody: {
                    [key: string]: unknown;
                };
                workspace: string;
            };
            res: {
                /**
                 * job resumed
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/cancel/{id}/{resume_id}/{signature}': {
        get: {
            req: {
                approver?: string;
                id: string;
                resumeId: number;
                signature: string;
                workspace: string;
            };
            res: {
                /**
                 * job canceled
                 */
                201: string;
            };
        };
        post: {
            req: {
                approver?: string;
                id: string;
                requestBody: {
                    [key: string]: unknown;
                };
                resumeId: number;
                signature: string;
                workspace: string;
            };
            res: {
                /**
                 * job canceled
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/jobs_u/get_flow/{id}/{resume_id}/{signature}': {
        get: {
            req: {
                approver?: string;
                id: string;
                resumeId: number;
                signature: string;
                workspace: string;
            };
            res: {
                /**
                 * parent flow details
                 */
                200: {
                    job: Job;
                    approvers: Array<{
                        resume_id: number;
                        approver: string;
                    }>;
                };
            };
        };
    };
    '/w/{workspace}/raw_apps/list': {
        get: {
            req: {
                /**
                 * mask to filter exact matching user creator
                 */
                createdBy?: string;
                /**
                 * order by desc order (default true)
                 */
                orderDesc?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * mask to filter exact matching path
                 */
                pathExact?: string;
                /**
                 * mask to filter matching starting path
                 */
                pathStart?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                /**
                 * (default false)
                 * show only the starred items
                 *
                 */
                starredOnly?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * All raw apps
                 */
                200: Array<ListableRawApp>;
            };
        };
    };
    '/w/{workspace}/raw_apps/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app exists
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/apps/get_data/{version}/{path}': {
        get: {
            req: {
                path: string;
                version: number;
                workspace: string;
            };
            res: {
                /**
                 * app details
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/raw_apps/create': {
        post: {
            req: {
                /**
                 * new raw app
                 */
                requestBody: {
                    path: string;
                    value: string;
                    summary: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * raw app created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/raw_apps/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updateraw  app
                 */
                requestBody: {
                    path?: string;
                    summary?: string;
                    value?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * app updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/raw_apps/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * app deleted
                 */
                200: string;
            };
        };
    };
    '/schedules/preview': {
        post: {
            req: {
                /**
                 * schedule
                 */
                requestBody: {
                    schedule: string;
                    timezone: string;
                };
            };
            res: {
                /**
                 * List of 5 estimated upcoming execution events (in UTC)
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/schedules/create': {
        post: {
            req: {
                /**
                 * new schedule
                 */
                requestBody: NewSchedule;
                workspace: string;
            };
            res: {
                /**
                 * schedule created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/schedules/update/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updated schedule
                 */
                requestBody: EditSchedule;
                workspace: string;
            };
            res: {
                /**
                 * schedule updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/schedules/setenabled/{path}': {
        post: {
            req: {
                path: string;
                /**
                 * updated schedule enable
                 */
                requestBody: {
                    enabled: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * schedule enabled set
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/schedules/delete/{path}': {
        delete: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * schedule deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/schedules/get/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * schedule deleted
                 */
                200: Schedule;
            };
        };
    };
    '/w/{workspace}/schedules/exists/{path}': {
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * schedule exists
                 */
                200: boolean;
            };
        };
    };
    '/w/{workspace}/schedules/list': {
        get: {
            req: {
                /**
                 * filter on jobs containing those args as a json subset (@> in postgres)
                 */
                args?: string;
                isFlow?: boolean;
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * filter by path
                 */
                path?: string;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                workspace: string;
            };
            res: {
                /**
                 * schedule list
                 */
                200: Array<Schedule>;
            };
        };
    };
    '/w/{workspace}/schedules/list_with_jobs': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                workspace: string;
            };
            res: {
                /**
                 * schedule list
                 */
                200: Array<ScheduleWJobs>;
            };
        };
    };
    '/w/{workspace}/schedules/setdefaulthandler': {
        post: {
            req: {
                /**
                 * Handler description
                 */
                requestBody: {
                    handler_type: 'error' | 'recovery';
                    override_existing: boolean;
                    path?: string;
                    extra_args?: {
                        [key: string]: unknown;
                    };
                    number_of_occurence?: number;
                    number_of_occurence_exact?: boolean;
                    workspace_handler_muted?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * default error handler set
                 */
                201: unknown;
            };
        };
    };
    '/groups/list': {
        get: {
            res: {
                /**
                 * instance group list
                 */
                200: Array<InstanceGroup>;
            };
        };
    };
    '/groups/get/{name}': {
        get: {
            req: {
                name: string;
            };
            res: {
                /**
                 * instance group
                 */
                200: InstanceGroup;
            };
        };
    };
    '/groups/create': {
        post: {
            req: {
                /**
                 * create instance group
                 */
                requestBody: {
                    name: string;
                    summary?: string;
                };
            };
            res: {
                /**
                 * instance group created
                 */
                200: string;
            };
        };
    };
    '/groups/update/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * update instance group
                 */
                requestBody: {
                    new_summary: string;
                };
            };
            res: {
                /**
                 * instance group updated
                 */
                200: string;
            };
        };
    };
    '/groups/delete/{name}': {
        delete: {
            req: {
                name: string;
            };
            res: {
                /**
                 * instance group deleted
                 */
                200: string;
            };
        };
    };
    '/groups/adduser/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * user to add to instance group
                 */
                requestBody: {
                    email: string;
                };
            };
            res: {
                /**
                 * user added to instance group
                 */
                200: string;
            };
        };
    };
    '/groups/removeuser/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * user to remove from instance group
                 */
                requestBody: {
                    email: string;
                };
            };
            res: {
                /**
                 * user removed from instance group
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/groups/list': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                workspace: string;
            };
            res: {
                /**
                 * group list
                 */
                200: Array<Group>;
            };
        };
    };
    '/w/{workspace}/groups/listnames': {
        get: {
            req: {
                /**
                 * only list the groups the user is member of (default false)
                 */
                onlyMemberOf?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * group list
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/groups/create': {
        post: {
            req: {
                /**
                 * create group
                 */
                requestBody: {
                    name: string;
                    summary?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * group created
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/groups/update/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * updated group
                 */
                requestBody: {
                    summary?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * group updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/groups/delete/{name}': {
        delete: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * group deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/groups/get/{name}': {
        get: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * group
                 */
                200: Group;
            };
        };
    };
    '/w/{workspace}/groups/adduser/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * added user to group
                 */
                requestBody: {
                    username?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * user added to group
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/groups/removeuser/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * added user to group
                 */
                requestBody: {
                    username?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * user removed from group
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/folders/list': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                workspace: string;
            };
            res: {
                /**
                 * folder list
                 */
                200: Array<Folder>;
            };
        };
    };
    '/w/{workspace}/folders/listnames': {
        get: {
            req: {
                /**
                 * only list the folders the user is member of (default false)
                 */
                onlyMemberOf?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * folder list
                 */
                200: Array<(string)>;
            };
        };
    };
    '/w/{workspace}/folders/create': {
        post: {
            req: {
                /**
                 * create folder
                 */
                requestBody: {
                    name: string;
                    owners?: Array<(string)>;
                    extra_perms?: unknown;
                };
                workspace: string;
            };
            res: {
                /**
                 * folder created
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/folders/update/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * update folder
                 */
                requestBody: {
                    owners?: Array<(string)>;
                    extra_perms?: unknown;
                };
                workspace: string;
            };
            res: {
                /**
                 * folder updated
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/folders/delete/{name}': {
        delete: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * folder deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/folders/get/{name}': {
        get: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * folder
                 */
                200: Folder;
            };
        };
    };
    '/w/{workspace}/folders/getusage/{name}': {
        get: {
            req: {
                name: string;
                workspace: string;
            };
            res: {
                /**
                 * folder
                 */
                200: {
                    scripts: number;
                    flows: number;
                    apps: number;
                    resources: number;
                    variables: number;
                    schedules: number;
                };
            };
        };
    };
    '/w/{workspace}/folders/addowner/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * owner user to folder
                 */
                requestBody: {
                    owner: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * owner added to folder
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/folders/removeowner/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * added owner to folder
                 */
                requestBody: {
                    owner: string;
                    write?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * owner removed from folder
                 */
                200: string;
            };
        };
    };
    '/configs/list_worker_groups': {
        get: {
            res: {
                /**
                 * a list of worker group configs
                 */
                200: Array<{
                    name: string;
                    config: unknown;
                }>;
            };
        };
    };
    '/configs/get/{name}': {
        get: {
            req: {
                name: string;
            };
            res: {
                /**
                 * a config
                 */
                200: unknown;
            };
        };
    };
    '/configs/update/{name}': {
        post: {
            req: {
                name: string;
                /**
                 * worker group
                 */
                requestBody: unknown;
            };
            res: {
                /**
                 * Update a worker group
                 */
                200: string;
            };
        };
        delete: {
            req: {
                name: string;
            };
            res: {
                /**
                 * Delete config
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/acls/get/{kind}/{path}': {
        get: {
            req: {
                kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * acls
                 */
                200: {
                    [key: string]: (boolean);
                };
            };
        };
    };
    '/w/{workspace}/acls/add/{kind}/{path}': {
        post: {
            req: {
                kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
                path: string;
                /**
                 * acl to add
                 */
                requestBody: {
                    owner: string;
                    write?: boolean;
                };
                workspace: string;
            };
            res: {
                /**
                 * granular acl added
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/acls/remove/{kind}/{path}': {
        post: {
            req: {
                kind: 'script' | 'group_' | 'resource' | 'schedule' | 'variable' | 'flow' | 'folder' | 'app' | 'raw_app';
                path: string;
                /**
                 * acl to add
                 */
                requestBody: {
                    owner: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * granular acl removed
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/capture_u/{path}': {
        post: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow preview captured
                 */
                204: void;
            };
        };
    };
    '/w/{workspace}/capture/{path}': {
        put: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * flow preview capture created
                 */
                201: unknown;
            };
        };
        get: {
            req: {
                path: string;
                workspace: string;
            };
            res: {
                /**
                 * captured flow preview
                 */
                200: unknown;
                /**
                 * capture does not exist for this flow
                 */
                404: unknown;
            };
        };
    };
    '/w/{workspace}/favorites/star': {
        post: {
            req: {
                requestBody?: {
                    path?: string;
                    favorite_kind?: 'flow' | 'app' | 'script' | 'raw_app';
                };
                workspace: string;
            };
            res: {
                /**
                 * star item
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/favorites/unstar': {
        post: {
            req: {
                requestBody?: {
                    path?: string;
                    favorite_kind?: 'flow' | 'app' | 'script' | 'raw_app';
                };
                workspace: string;
            };
            res: {
                /**
                 * unstar item
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/inputs/history': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                runnableId?: string;
                runnableType?: RunnableType;
                workspace: string;
            };
            res: {
                /**
                 * Input history for completed jobs
                 */
                200: Array<Input>;
            };
        };
    };
    '/w/{workspace}/inputs/{jobOrInputId}/args': {
        get: {
            req: {
                jobOrInputId: string;
                workspace: string;
            };
            res: {
                /**
                 * args
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/inputs/list': {
        get: {
            req: {
                /**
                 * which page to return (start at 1, default 1)
                 */
                page?: number;
                /**
                 * number of items to return for a given page (default 30, max 100)
                 */
                perPage?: number;
                runnableId?: string;
                runnableType?: RunnableType;
                workspace: string;
            };
            res: {
                /**
                 * Saved Inputs for a Runnable
                 */
                200: Array<Input>;
            };
        };
    };
    '/w/{workspace}/inputs/create': {
        post: {
            req: {
                /**
                 * Input
                 */
                requestBody: CreateInput;
                runnableId?: string;
                runnableType?: RunnableType;
                workspace: string;
            };
            res: {
                /**
                 * Input created
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/inputs/update': {
        post: {
            req: {
                /**
                 * UpdateInput
                 */
                requestBody: UpdateInput;
                workspace: string;
            };
            res: {
                /**
                 * Input updated
                 */
                201: string;
            };
        };
    };
    '/w/{workspace}/inputs/delete/{input}': {
        post: {
            req: {
                input: string;
                workspace: string;
            };
            res: {
                /**
                 * Input deleted
                 */
                200: string;
            };
        };
    };
    '/w/{workspace}/job_helpers/duckdb_connection_settings': {
        post: {
            req: {
                /**
                 * S3 resource to connect to
                 */
                requestBody: {
                    s3_resource?: S3Resource;
                };
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: {
                    connection_settings_str?: string;
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/v2/duckdb_connection_settings': {
        post: {
            req: {
                /**
                 * S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
                 */
                requestBody: {
                    s3_resource_path?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: {
                    connection_settings_str: string;
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/polars_connection_settings': {
        post: {
            req: {
                /**
                 * S3 resource to connect to
                 */
                requestBody: {
                    s3_resource?: S3Resource;
                };
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: {
                    endpoint_url: string;
                    key?: string;
                    secret?: string;
                    use_ssl: boolean;
                    cache_regions: boolean;
                    client_kwargs: PolarsClientKwargs;
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/v2/polars_connection_settings': {
        post: {
            req: {
                /**
                 * S3 resource path to use to generate the connection settings. If empty, the S3 resource defined in the workspace settings will be used
                 */
                requestBody: {
                    s3_resource_path?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: {
                    s3fs_args: {
                        endpoint_url: string;
                        key?: string;
                        secret?: string;
                        use_ssl: boolean;
                        cache_regions: boolean;
                        client_kwargs: PolarsClientKwargs;
                    };
                    storage_options: {
                        aws_endpoint_url: string;
                        aws_access_key_id?: string;
                        aws_secret_access_key?: string;
                        aws_region: string;
                        aws_allow_http: string;
                    };
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/v2/s3_resource_info': {
        post: {
            req: {
                /**
                 * S3 resource path to use. If empty, the S3 resource defined in the workspace settings will be used
                 */
                requestBody: {
                    s3_resource_path?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: S3Resource;
            };
        };
    };
    '/w/{workspace}/job_helpers/test_connection': {
        get: {
            req: {
                workspace: string;
            };
            res: {
                /**
                 * Connection settings
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/job_helpers/list_stored_files': {
        get: {
            req: {
                marker?: string;
                maxKeys: number;
                prefix?: string;
                workspace: string;
            };
            res: {
                /**
                 * List of file keys
                 */
                200: {
                    next_marker?: string;
                    windmill_large_files: Array<WindmillLargeFile>;
                    restricted_access?: boolean;
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/load_file_metadata': {
        get: {
            req: {
                fileKey: string;
                workspace: string;
            };
            res: {
                /**
                 * FileMetadata
                 */
                200: WindmillFileMetadata;
            };
        };
    };
    '/w/{workspace}/job_helpers/load_file_preview': {
        get: {
            req: {
                csvHasHeader?: boolean;
                csvSeparator?: string;
                fileKey: string;
                fileMimeType?: string;
                fileSizeInBytes?: number;
                readBytesFrom?: number;
                readBytesLength?: number;
                workspace: string;
            };
            res: {
                /**
                 * FilePreview
                 */
                200: WindmillFilePreview;
            };
        };
    };
    '/w/{workspace}/job_helpers/load_parquet_preview/{path}': {
        get: {
            req: {
                limit?: number;
                offset?: number;
                path: string;
                searchCol?: string;
                searchTerm?: string;
                sortCol?: string;
                sortDesc?: boolean;
                workspace: string;
            };
            res: {
                /**
                 * Parquet Preview
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/job_helpers/delete_s3_file': {
        delete: {
            req: {
                fileKey: string;
                workspace: string;
            };
            res: {
                /**
                 * Confirmation
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/job_helpers/move_s3_file': {
        get: {
            req: {
                destFileKey: string;
                srcFileKey: string;
                workspace: string;
            };
            res: {
                /**
                 * Confirmation
                 */
                200: unknown;
            };
        };
    };
    '/w/{workspace}/job_helpers/upload_s3_file': {
        post: {
            req: {
                fileExtension?: string;
                fileKey?: string;
                /**
                 * File content
                 */
                requestBody: (Blob | File);
                resourceType?: string;
                s3ResourcePath?: string;
                workspace: string;
            };
            res: {
                /**
                 * File upload status
                 */
                200: {
                    file_key: string;
                };
            };
        };
    };
    '/w/{workspace}/job_helpers/download_s3_file': {
        get: {
            req: {
                fileKey: string;
                resourceType?: string;
                s3ResourcePath?: string;
                workspace: string;
            };
            res: {
                /**
                 * Chunk of the downloaded file
                 */
                200: (Blob | File);
            };
        };
    };
    '/w/{workspace}/job_metrics/get/{id}': {
        post: {
            req: {
                id: string;
                /**
                 * parameters for statistics retrieval
                 */
                requestBody: {
                    timeseries_max_datapoints?: number;
                    from_timestamp?: string;
                    to_timestamp?: string;
                };
                workspace: string;
            };
            res: {
                /**
                 * job details
                 */
                200: {
                    metrics_metadata?: Array<MetricMetadata>;
                    scalar_metrics?: Array<ScalarMetric>;
                    timeseries_metrics?: Array<TimeseriesMetric>;
                };
            };
        };
    };
    '/concurrency_groups/list': {
        get: {
            res: {
                /**
                 * all concurrency groups
                 */
                200: Array<ConcurrencyGroup>;
            };
        };
    };
    '/concurrency_groups/{concurrency_id}': {
        delete: {
            req: {
                concurrencyId: string;
            };
            res: {
                /**
                 * concurrency group removed
                 */
                200: unknown;
            };
        };
    };
};
