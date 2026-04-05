# Windmill CLI Commands

The Windmill CLI (`wmill`) provides commands for managing scripts, flows, apps, and other resources.

## Global Options

- `--workspace <workspace:string>` - Specify the target workspace. This overrides the default workspace.
- `--debug --verbose` - Show debug/verbose logs
- `--show-diffs` - Show diff informations when syncing (may show sensitive informations)
- `--token <token:string>` - Specify an API token. This will override any stored token.
- `--base-url <baseUrl:string>` - Specify the base URL of the API. If used, --token and --workspace are required and no local remote/workspace already set will be used.
- `--config-dir <configDir:string>` - Specify a custom config directory. Overrides WMILL_CONFIG_DIR environment variable and default ~/.config location.

## Commands

### app

app related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `app list` - list all apps
  - `--json` - Output as JSON (for piping to jq)
- `app get <path:string>` - get an app's details
  - `--json` - Output as JSON (for piping to jq)
- `app push <file_path:string> <remote_path:string>` - push a local app 
- `app dev [app_folder:string]` - Start a development server for building apps with live reload and hot module replacement
  - `--port <port:number>` - Port to run the dev server on (will find next available port if occupied)
  - `--host <host:string>` - Host to bind the dev server to
  - `--entry <entry:string>` - Entry point file (default: index.ts for Svelte/Vue, index.tsx otherwise)
  - `--no-open` - Don't automatically open the browser
- `app lint [app_folder:string]` - Lint a raw app folder to validate structure and buildability
  - `--fix` - Attempt to fix common issues (not implemented yet)
- `app new` - create a new raw app from a template
- `app generate-agents [app_folder:string]` - regenerate AGENTS.md and DATATABLES.md from remote workspace

### audit

View audit logs (requires admin)

**Subcommands:**

- `audit list` - List audit log entries
- `audit get <id:string>` - Get a specific audit log entry
  - `--json` - Output as JSON (for piping to jq)

### config

Show all available wmill.yaml configuration options

**Options:**
- `--json` - Output as JSON for programmatic consumption

### dependencies

workspace dependencies related commands

**Alias:** `deps`

**Subcommands:**

- `dependencies push <file_path:string>` - Push workspace dependencies from a local file

### dev

Launch a dev server that watches for local file changes and auto-pushes them to the remote workspace. Provides live reload for scripts and flows during development.

**Options:**
- `--includes <pattern...:string>` - Filter paths givena glob pattern or path
- `--proxy-port <port:number>` - Port for a localhost reverse proxy to the remote Windmill server
- `--path <path:string>` - Watch a specific windmill path (e.g., u/admin/my_script or f/my_flow)

### docs

Search Windmill documentation.

**Arguments:** `<query:string>`

**Options:**
- `--json` - Output results as JSON.

### flow

flow related commands

**Options:**
- `--show-archived` - Enable archived flows in output
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `flow list` - list all flows
  - `--show-archived` - Enable archived flows in output
  - `--json` - Output as JSON (for piping to jq)
- `flow get <path:string>` - get a flow's details
  - `--json` - Output as JSON (for piping to jq)
- `flow push <file_path:string> <remote_path:string>` - push a local flow spec. This overrides any remote versions.
  - `--message <message:string>` - Deployment message
- `flow run <path:string>` - run a flow by path.
  - `-d --data <data:string>` - Inputs specified as a JSON string or a file using @<filename> or stdin using @-.
  - `-s --silent` - Do not ouput anything other then the final output. Useful for scripting.
- `flow preview <flow_path:string>` - preview a local flow without deploying it. Runs the flow definition from local files and uses local PathScripts by default.
  - `-d --data <data:string>` - Inputs specified as a JSON string or a file using @<filename> or stdin using @-.
  - `-s --silent` - Do not output anything other then the final output. Useful for scripting.
  - `--remote` - Use deployed workspace scripts for PathScript steps instead of local files.
- `flow new <flow_path:string>` - create a new empty flow
  - `--summary <summary:string>` - flow summary
  - `--description <description:string>` - flow description
- `flow bootstrap <flow_path:string>` - create a new empty flow (alias for new)
  - `--summary <summary:string>` - flow summary
  - `--description <description:string>` - flow description
- `flow history <path:string>` - Show version history for a flow
  - `--json` - Output as JSON (for piping to jq)
- `flow show-version <path:string> <version:string>` - Show a specific version of a flow
  - `--json` - Output as JSON (for piping to jq)

### folder

folder related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `folder list` - list all folders
  - `--json` - Output as JSON (for piping to jq)
- `folder get <name:string>` - get a folder's details
  - `--json` - Output as JSON (for piping to jq)
- `folder new <name:string>` - create a new folder locally
  - `--summary <summary:string>` - folder summary
- `folder push <name:string>` - push a local folder to the remote by name. This overrides any remote versions.
- `folder add-missing` - create default folder.meta.yaml for all subdirectories of f/ that are missing one
  - `-y, --yes` - skip confirmation prompt

### generate-metadata

Generate metadata (locks, schemas) for all scripts, flows, and apps

**Arguments:** `[folder:string]`

**Options:**
- `--yes` - Skip confirmation prompt
- `--dry-run` - Show what would be updated without making changes
- `--lock-only` - Re-generate only the lock files
- `--schema-only` - Re-generate only script schemas (skips flows and apps)
- `--skip-scripts` - Skip processing scripts
- `--skip-flows` - Skip processing flows
- `--skip-apps` - Skip processing apps
- `--strict-folder-boundaries` - Only update items inside the specified folder (requires folder argument)
- `-i --includes <patterns:file[]>` - Comma separated patterns to specify which files to include
- `-e --excludes <patterns:file[]>` - Comma separated patterns to specify which files to exclude

### gitsync-settings

Manage git-sync settings between local wmill.yaml and Windmill backend

**Subcommands:**

- `gitsync-settings pull` - Pull git-sync settings from Windmill backend to local wmill.yaml
  - `--repository <repo:string>` - Specify repository path (e.g., u/user/repo)
  - `--default` - Write settings to top-level defaults instead of overrides
  - `--replace` - Replace existing settings (non-interactive mode)
  - `--override` - Add branch-specific override (non-interactive mode)
  - `--diff` - Show differences without applying changes
  - `--json-output` - Output in JSON format
  - `--with-backend-settings <json:string>` - Use provided JSON settings instead of querying backend (for testing)
  - `--yes` - Skip interactive prompts and use default behavior
  - `--promotion <branch:string>` - Use promotionOverrides from the specified branch instead of regular overrides
- `gitsync-settings push` - Push git-sync settings from local wmill.yaml to Windmill backend
  - `--repository <repo:string>` - Specify repository path (e.g., u/user/repo)
  - `--diff` - Show what would be pushed without applying changes
  - `--json-output` - Output in JSON format
  - `--with-backend-settings <json:string>` - Use provided JSON settings instead of querying backend (for testing)
  - `--yes` - Skip interactive prompts and use default behavior
  - `--promotion <branch:string>` - Use promotionOverrides from the specified branch instead of regular overrides

### group

Manage workspace groups

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `group list` - List all groups in the workspace
  - `--json` - Output as JSON (for piping to jq)
- `group get <name:string>` - Get group details and members
  - `--json` - Output as JSON (for piping to jq)
- `group create <name:string>` - Create a new group
  - `--summary <summary:string>` - Group summary/description
- `group delete <name:string>` - Delete a group
- `group add-user <name:string> <username:string>` - Add a user to a group
- `group remove-user <name:string> <username:string>` - Remove a user from a group

### hub

Hub related commands. EXPERIMENTAL. INTERNAL USE ONLY.

**Subcommands:**

- `hub pull` - pull any supported definitions. EXPERIMENTAL.

### init

Bootstrap a windmill project with a wmill.yaml file

**Options:**
- `--use-default` - Use default settings without checking backend
- `--use-backend` - Use backend git-sync settings if available
- `--repository <repo:string>` - Specify repository path (e.g., u/user/repo) when using backend settings
- `--bind-profile` - Automatically bind active workspace profile to current Git branch
- `--no-bind-profile` - Skip workspace profile binding prompt

### instance

sync local with a remote instance or the opposite (push or pull)

**Subcommands:**

- `instance add [instance_name:string] [remote:string] [token:string]` - Add a new instance
- `instance remove <instance:string:instance>` - Remove an instance
- `instance switch <instance:string:instance>` - Switch the current instance
- `instance pull` - Pull instance settings, users, configs, instance groups and overwrite local
  - `--yes` - Pull without needing confirmation
  - `--dry-run` - Perform a dry run without making changes
  - `--skip-users` - Skip pulling users
  - `--skip-settings` - Skip pulling settings
  - `--skip-configs` - Skip pulling configs (worker groups and SMTP)
  - `--skip-groups` - Skip pulling instance groups
  - `--include-workspaces` - Also pull workspaces
  - `--folder-per-instance` - Create a folder per instance
  - `--instance <instance:string>` - Name of the instance to pull from, override the active instance
  - `--prefix <prefix:string>` - Prefix of the local workspaces to pull, used to create the folders when using --include-workspaces
  - `--prefix-settings` - Store instance yamls inside prefixed folders when using --prefix and --folder-per-instance
- `instance push` - Push instance settings, users, configs, group and overwrite remote
  - `--yes` - Push without needing confirmation
  - `--dry-run` - Perform a dry run without making changes
  - `--skip-users` - Skip pushing users
  - `--skip-settings` - Skip pushing settings
  - `--skip-configs` - Skip pushing configs (worker groups and SMTP)
  - `--skip-groups` - Skip pushing instance groups
  - `--include-workspaces` - Also push workspaces
  - `--folder-per-instance` - Create a folder per instance
  - `--instance <instance:string>` - Name of the instance to push to, override the active instance
  - `--prefix <prefix:string>` - Prefix of the local workspaces folders to push
  - `--prefix-settings` - Store instance yamls inside prefixed folders when using --prefix and --folder-per-instance
- `instance whoami` - Display information about the currently logged-in user
- `instance get-config` - Dump the current instance config (global settings + worker configs) as YAML
  - `-o, --output-file <file:string>` - Write YAML to a file instead of stdout
  - `--show-secrets` - Include sensitive fields (license key, JWT secret) without prompting
  - `--instance <instance:string>` - Name of the instance, override the active instance

### job

Manage jobs (list, inspect, cancel)

**Subcommands:**

- `job list` - List recent jobs
- `job get <id:string>` - Get job details. For flows: shows step tree with sub-job IDs
  - `--json` - Output as JSON (for piping to jq)
- `job result <id:string>` - Get the result of a completed job (machine-friendly)
- `job logs <id:string>` - Get job logs. For flows: aggregates all step logs
- `job cancel <id:string>` - Cancel a running or queued job
  - `--reason <reason:string>` - Reason for cancellation

### jobs

Pull completed and queued jobs from workspace

**Arguments:** `[workspace:string]`

**Options:**
- `-c, --completed-output <file:string>` - Completed jobs output file (default: completed_jobs.json)
- `-q, --queued-output <file:string>` - Queued jobs output file (default: queued_jobs.json)
- `--skip-worker-check` - Skip checking for active workers before export

**Subcommands:**

- `jobs pull`
- `jobs push`

### lint

Validate Windmill flow, schedule, and trigger YAML files in a directory

**Arguments:** `[directory:string]`

**Options:**
- `--json` - Output results in JSON format
- `--fail-on-warn` - Exit with code 1 when warnings are emitted
- `--locks-required` - Fail if scripts or flow inline scripts that need locks have no locks
- `-w, --watch` - Watch for file changes and re-lint automatically

### queues

List all queues with their metrics

**Arguments:** `[workspace:string] the optional workspace to filter by (default to all workspaces)`

**Options:**
- `--instance [instance]` - Name of the instance to push to, override the active instance
- `--base-url [baseUrl]` - If used with --token, will be used as the base url for the instance

### resource

resource related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `resource list` - list all resources
  - `--json` - Output as JSON (for piping to jq)
- `resource get <path:string>` - get a resource's details
  - `--json` - Output as JSON (for piping to jq)
- `resource new <path:string>` - create a new resource locally
- `resource push <file_path:string> <remote_path:string>` - push a local resource spec. This overrides any remote versions.

### resource-type

resource type related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `resource-type list` - list all resource types
  - `--schema` - Show schema in the output
  - `--json` - Output as JSON (for piping to jq)
- `resource-type get <path:string>` - get a resource type's details
  - `--json` - Output as JSON (for piping to jq)
- `resource-type new <name:string>` - create a new resource type locally
- `resource-type push <file_path:string> <name:string>` - push a local resource spec. This overrides any remote versions.
- `resource-type generate-namespace` - Create a TypeScript definition file with the RT namespace generated from the resource types

### schedule

schedule related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `schedule list` - list all schedules
  - `--json` - Output as JSON (for piping to jq)
- `schedule get <path:string>` - get a schedule's details
  - `--json` - Output as JSON (for piping to jq)
- `schedule new <path:string>` - create a new schedule locally
- `schedule push <file_path:string> <remote_path:string>` - push a local schedule spec. This overrides any remote versions.
- `schedule enable <path:string>` - Enable a schedule
- `schedule disable <path:string>` - Disable a schedule

### script

script related commands

**Options:**
- `--show-archived` - Show archived scripts instead of active ones
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `script list` - list all scripts
  - `--show-archived` - Show archived scripts instead of active ones
  - `--json` - Output as JSON (for piping to jq)
- `script push <path:file>` - push a local script spec. This overrides any remote versions. Use the script file (.ts, .js, .py, .sh)
  - `--message <message:string>` - Deployment message
- `script get <path:file>` - get a script's details
  - `--json` - Output as JSON (for piping to jq)
- `script show <path:file>` - show a script's content (alias for get)
- `script run <path:file>` - run a script by path
  - `-d --data <data:file>` - Inputs specified as a JSON string or a file using @<filename> or stdin using @-.
  - `-s --silent` - Do not output anything other then the final output. Useful for scripting.
- `script preview <path:file>` - preview a local script without deploying it. Supports both regular and codebase scripts.
  - `-d --data <data:file>` - Inputs specified as a JSON string or a file using @<filename> or stdin using @-.
  - `-s --silent` - Do not output anything other than the final output. Useful for scripting.
- `script new <path:file> <language:string>` - create a new script
  - `--summary <summary:string>` - script summary
  - `--description <description:string>` - script description
- `script bootstrap <path:file> <language:string>` - create a new script (alias for new)
  - `--summary <summary:string>` - script summary
  - `--description <description:string>` - script description
- `script history <path:string>` - show version history for a script
  - `--json` - Output as JSON (for piping to jq)

### sync

sync local with a remote workspaces or the opposite (push or pull)

**Subcommands:**

- `sync pull` - Pull any remote changes and apply them locally.
  - `--yes` - Pull without needing confirmation
  - `--dry-run` - Show changes that would be pulled without actually pushing
  - `--plain-secrets` - Pull secrets as plain text
  - `--json` - Use JSON instead of YAML
  - `--skip-variables` - Skip syncing variables (including secrets)
  - `--skip-secrets` - Skip syncing only secrets variables
  - `--include-secrets` - Include secrets in sync (overrides skipSecrets in wmill.yaml)
  - `--skip-resources` - Skip syncing  resources
  - `--skip-resource-types` - Skip syncing  resource types
  - `--skip-scripts` - Skip syncing scripts
  - `--skip-flows` - Skip syncing flows
  - `--skip-apps` - Skip syncing apps
  - `--skip-folders` - Skip syncing folders
  - `--skip-workspace-dependencies` - Skip syncing workspace dependencies
  - `--skip-scripts-metadata` - Skip syncing scripts metadata, focus solely on logic
  - `--include-schedules` - Include syncing  schedules
  - `--include-triggers` - Include syncing triggers
  - `--include-users` - Include syncing users
  - `--include-groups` - Include syncing groups
  - `--include-settings` - Include syncing workspace settings
  - `--include-key` - Include workspace encryption key
  - `--skip-branch-validation` - Skip git branch validation and prompts
  - `--json-output` - Output results in JSON format
  - `-i --includes <patterns:file[]>` - Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Overrides wmill.yaml includes
  - `-e --excludes <patterns:file[]>` - Comma separated patterns to specify which file to NOT take into account. Overrides wmill.yaml excludes
  - `--extra-includes <patterns:file[]>` - Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy
  - `--repository <repo:string>` - Specify repository path (e.g., u/user/repo) when multiple repositories exist
  - `--promotion <branch:string>` - Use promotionOverrides from the specified branch instead of regular overrides
  - `--branch, --env <branch:string>` - Override the current git branch/environment (works even outside a git repository)
- `sync push` - Push any local changes and apply them remotely.
  - `--yes` - Push without needing confirmation
  - `--dry-run` - Show changes that would be pushed without actually pushing
  - `--plain-secrets` - Push secrets as plain text
  - `--json` - Use JSON instead of YAML
  - `--skip-variables` - Skip syncing variables (including secrets)
  - `--skip-secrets` - Skip syncing only secrets variables
  - `--include-secrets` - Include secrets in sync (overrides skipSecrets in wmill.yaml)
  - `--skip-resources` - Skip syncing  resources
  - `--skip-resource-types` - Skip syncing  resource types
  - `--skip-scripts` - Skip syncing scripts
  - `--skip-flows` - Skip syncing flows
  - `--skip-apps` - Skip syncing apps
  - `--skip-folders` - Skip syncing folders
  - `--skip-workspace-dependencies` - Skip syncing workspace dependencies
  - `--skip-scripts-metadata` - Skip syncing scripts metadata, focus solely on logic
  - `--include-schedules` - Include syncing schedules
  - `--include-triggers` - Include syncing triggers
  - `--include-users` - Include syncing users
  - `--include-groups` - Include syncing groups
  - `--include-settings` - Include syncing workspace settings
  - `--include-key` - Include workspace encryption key
  - `--skip-branch-validation` - Skip git branch validation and prompts
  - `--json-output` - Output results in JSON format
  - `-i --includes <patterns:file[]>` - Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)
  - `-e --excludes <patterns:file[]>` - Comma separated patterns to specify which file to NOT take into account.
  - `--extra-includes <patterns:file[]>` - Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string). Useful to still take wmill.yaml into account and act as a second pattern to satisfy
  - `--message <message:string>` - Include a message that will be added to all scripts/flows/apps updated during this push
  - `--parallel <number>` - Number of changes to process in parallel
  - `--repository <repo:string>` - Specify repository path (e.g., u/user/repo) when multiple repositories exist
  - `--branch, --env <branch:string>` - Override the current git branch/environment (works even outside a git repository)
  - `--lint` - Run lint validation before pushing
  - `--locks-required` - Fail if scripts or flow inline scripts that need locks have no locks
  - `--auto-metadata` - Automatically regenerate stale metadata (locks and schemas) before pushing

### token

Manage API tokens

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `token list` - List API tokens
  - `--json` - Output as JSON (for piping to jq)
- `token create` - Create a new API token
  - `--label <label:string>` - Token label
  - `--expiration <expiration:string>` - Token expiration (ISO 8601 timestamp)
- `token delete <token_prefix:string>` - Delete a token by its prefix

### trigger

trigger related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `trigger list` - list all triggers
  - `--json` - Output as JSON (for piping to jq)
- `trigger get <path:string>` - get a trigger's details
  - `--json` - Output as JSON (for piping to jq)
  - `--kind <kind:string>` - Trigger kind (http, websocket, kafka, nats, postgres, mqtt, sqs, gcp, email). Recommended for faster lookup
- `trigger new <path:string>` - create a new trigger locally
  - `--kind <kind:string>` - Trigger kind (required: http, websocket, kafka, nats, postgres, mqtt, sqs, gcp, email)
- `trigger push <file_path:string> <remote_path:string>` - push a local trigger spec. This overrides any remote versions.

### user

user related commands

**Subcommands:**

- `user add <email:string> [password:string]` - Create a user
  - `--superadmin` - Specify to make the new user superadmin.
  - `--company <company:string>` - Specify to set the company of the new user.
  - `--name <name:string>` - Specify to set the name of the new user.
- `user remove <email:string>` - Delete a user
- `user create-token` - Create a new API token for the authenticated user
  - `--email <email:string>` - Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.
  - `--password <password:string>` - Specify credentials to use for authentication. This will not be stored. It will only be used to exchange for a token with the API server, which will not be stored either.

### variable

variable related commands

**Options:**
- `--json` - Output as JSON (for piping to jq)

**Subcommands:**

- `variable list` - list all variables
  - `--json` - Output as JSON (for piping to jq)
- `variable get <path:string>` - get a variable's details
  - `--json` - Output as JSON (for piping to jq)
- `variable new <path:string>` - create a new variable locally
- `variable push <file_path:string> <remote_path:string>` - Push a local variable spec. This overrides any remote versions.
  - `--plain-secrets` - Push secrets as plain text
- `variable add <value:string> <remote_path:string>` - Create a new variable on the remote. This will update the variable if it already exists.
  - `--plain-secrets` - Push secrets as plain text
  - `--public` - Legacy option, use --plain-secrets instead

### version

Show version information

### worker-groups

display worker groups, pull and push worker groups configs

**Subcommands:**

- `worker-groups pull` - Pull worker groups (similar to `wmill instance pull --skip-users --skip-settings --skip-groups`)
  - `--instance` - Name of the instance to push to, override the active instance
  - `--base-url` - Base url to be passed to the instance settings instead of the local one
  - `--yes` - Pull without needing confirmation
- `worker-groups push` - Push instance settings, users, configs, group and overwrite remote
  - `--instance [instance]` - Name of the instance to push to, override the active instance
  - `--base-url [baseUrl]` - If used with --token, will be used as the base url for the instance
  - `--yes` - Push without needing confirmation

### workers

List all workers grouped by worker groups

**Options:**
- `--instance [instance]` - Name of the instance to push to, override the active instance
- `--base-url [baseUrl]` - If used with --token, will be used as the base url for the instance

### workspace

workspace related commands

**Alias:** `profile`

**Subcommands:**

- `workspace switch <workspace_name:string:workspace>` - Switch to another workspace
- `workspace add [workspace_name:string] [workspace_id:string] [remote:string]` - Add a workspace
  - `-c --create` - Create the workspace if it does not exist
  - `--create-workspace-name <workspace_name:string>` - Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id.
  - `--create-username <username:string>` - Specify your own username in the newly created workspace. Ignored if --create is not specified, the workspace already exists or automatic username creation is enabled on the instance.
- `workspace remove <workspace_name:string>` - Remove a workspace
- `workspace whoami` - Show the currently active user
- `workspace list` - List local workspace profiles
- `workspace list-remote` - List workspaces on the remote server that you have access to
- `workspace list-forks` - List forked workspaces on the remote server
- `workspace bind` - Bind the current Git branch to the active workspace. This adds the branch to gitBranches in wmill.yaml so sync operations use the correct workspace for each branch.
  - `--branch, --env <branch:string>` - Specify branch/environment (defaults to current)
- `workspace unbind` - Remove workspace binding from the current Git branch
  - `--branch, --env <branch:string>` - Specify branch/environment (defaults to current)
- `workspace fork [workspace_name:string] [workspace_id:string]` - Create a forked workspace
  - `--create-workspace-name <workspace_name:string>` - Specify the workspace name. Ignored if --create is not specified or the workspace already exists. Will default to the workspace id.
- `workspace delete-fork <fork_name:string>` - Delete a forked workspace and git branch
  - `-y --yes` - Skip confirmation prompt

