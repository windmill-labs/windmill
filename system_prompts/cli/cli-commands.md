# Windmill CLI Commands

The Windmill CLI (`wmill`) provides commands for managing scripts, flows, apps, and other resources.

## Global Options

- `--workspace <workspace>` - Specify target workspace (overrides default)
- `--token <token>` - Specify API token (overrides stored token)
- `--base-url <url>` - Specify API base URL
- `--debug` / `--verbose` - Show debug logs
- `--show-diffs` - Show diff information when syncing

## Core Commands

### Project Setup

- `wmill init` - Bootstrap a windmill project with wmill.yaml
  - `--use-default` - Use default settings without checking backend
  - `--use-backend` - Use backend git-sync settings if available
  - `--bind-profile` - Bind workspace profile to current Git branch

### Workspace Management

- `wmill workspace` - Manage workspaces
  - `wmill workspace add` - Add a new workspace
  - `wmill workspace switch` - Switch active workspace
  - `wmill workspace list` - List configured workspaces

### Sync Operations

- `wmill sync pull` - Pull remote changes to local
  - `--raw` - Pull without converting to local format
  - `--yes` - Skip confirmation prompts
  - `--include-schedules` - Include schedules in sync
  - `--include-triggers` - Include triggers in sync

- `wmill sync push` - Push local changes to remote
  - `--dry-run` - Preview changes without applying
  - `--yes` - Skip confirmation prompts
  - `--fail-conflicts` - Fail on conflicts instead of prompting

### Scripts

- `wmill script` - Manage scripts
  - `wmill script push <path>` - Push a script to remote
  - `wmill script generate-metadata` - Generate script metadata from code
    - Analyzes script code and creates/updates the `.script.yaml` metadata file
    - Infers parameter types, descriptions, and schema from code annotations

### Flows

- `wmill flow` - Manage flows
  - `wmill flow push <path>` - Push a flow to remote
  - `wmill flow generate-locks` - Generate lock files for flow dependencies
    - `--yes` - Skip confirmation prompts

### Apps

- `wmill app` - Manage apps
  - `wmill app push <path>` - Push an app to remote
  - `wmill app generate-locks` - Generate lock files for app dependencies

### Resources

- `wmill resource` - Manage resources
  - `wmill resource push` - Push resources to remote
  - `wmill resource pull` - Pull resources from remote

- `wmill resource-type` - Manage resource types
  - `wmill resource-type push` - Push resource types
  - `wmill resource-type pull` - Pull resource types

### Variables

- `wmill variable` - Manage variables
  - `wmill variable push` - Push variables to remote
  - `wmill variable pull` - Pull variables from remote

### Hub

- `wmill hub` - Interact with Windmill Hub
  - `wmill hub pull <path>` - Pull a script/flow from the hub

### Development

- `wmill dev` - Start development mode with file watching

### Other Commands

- `wmill folder` - Manage folders
- `wmill schedule` - Manage schedules
- `wmill trigger` - Manage triggers
- `wmill user` - Manage users
- `wmill jobs` - Manage jobs
- `wmill workers` - View worker status
- `wmill queues` - View queue status
- `wmill dependencies` - Manage dependencies
- `wmill instance` - Instance-level operations
- `wmill worker-groups` - Manage worker groups
- `wmill gitsync-settings` - Manage git-sync settings
- `wmill upgrade` - Upgrade CLI to latest version
- `wmill completions` - Generate shell completions

## Workflow Instructions

### After Writing a Script

After creating or modifying a script, run:
```bash
wmill script generate-metadata <path>
```

This will analyze your script code and generate/update the `.script.yaml` metadata file with inferred types and schema.

### After Writing a Flow

After creating or modifying a flow, run:
```bash
wmill flow generate-locks --yes
```

This generates lock files for all flow dependencies.

### Syncing Changes

To push all local changes to the remote workspace:
```bash
wmill sync push
```

To pull remote changes to local:
```bash
wmill sync pull
```
