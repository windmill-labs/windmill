# Workspace settings

Everything an admin sets on the Workspace settings page is one document: a mapping of top-level
keys, the same layout as the `settings.yaml` file `wmill sync` keeps. Only a workspace admin can
read or change it.

## Applying a document

- A partial document changes only the keys it states; every key it omits is left as it is.
- Each key is replaced whole, so read its current value and send it back edited: a nested object
  given with one field replaces the stored object with that one field.
- A key set to `null` clears that setting; an on/off switch cleared this way reads as `false`.
- Each setting is written through the same path the settings page uses, with the same validation
  and audit log. The writes are not one transaction: when one fails the apply stops, and the
  error says what was already written.
- A dry run reports the changes, as `before` / `after` per key, without writing anything.
- `name` is never applied: rename a workspace on the settings page. `color` is applied only when
  set to a value, so `null` does not clear it.
- A key the server does not know is ignored and named in the warnings: check them, since a typo
  otherwise reads as "nothing to change".
- A setting this edition does not offer (Enterprise only) is refused, by the dry run too.

## Secrets

`slack_oauth_client_secret` is the one secret. A read shows `__SENSITIVE_AND_UNCHANGED__` in place
of a set secret; sending that placeholder back keeps the stored value. A value starting with
`$encrypted:` is decrypted with the workspace key, and one encrypted with another workspace's key
is left unchanged with a warning; any other value is taken as plaintext. Never
write a secret you were not given: keep the placeholder.

## Keys

| Key | Value |
|---|---|
| `auto_invite` | `{ enabled, mode: invite \| add, operator, instance_groups?, instance_groups_roles? }`. The instance groups are managed only when `instance_groups` is stated; clearing a group removes the members it added. |
| `error_handler` / `success_handler` | `{ path, extra_args?, muted_on_cancel?, muted_on_user_path? }`, `path` being `script/<path>` or `flow/<path>`. |
| `error_handler_fallback_to_instance_alerts` | Report failures to the instance critical alert channels when no error handler is set. |
| `webhook` | URL POSTed a JSON event on every change to the workspace's items. |
| `ai_config` | Providers, default and completion models, custom prompts, `copilot_disabled`. |
| `large_file_storage` | Workspace object storage: `{ type, s3_resource_path \| azure_blob_resource_path \| gcs_resource_path, public_resource?, secondary_storage?, advanced_permissions? }`. |
| `git_sync` | `{ repositories: [...] }`, each with `git_repo_resource_path`, `script_path` and `settings` (include/exclude paths and types). Server-owned fields (webhook secrets, sync state, credentials) are never read nor written. |
| `datatable` | `{ datatables: { <name>: { database: { resource_type, resource_path } } } }`, both required: `resource_type: postgresql` with `resource_path` a Postgres resource's path, or `instance`, or `external_instance` (Enterprise only), with `resource_path` the name of a database a superadmin set up. A fork's entry can instead be `{ reference: { workspace_id, datatable } }`, pointing at its parent's data table; keep it as is. `main` is the one scripts use by default. Role permissions are `datatable_permissions`. |
| `datatable_permissions` | `{ <data table>: { default_role, roles: { <role>: [tenants] } } }`: who may connect to each data table as which role. Tenants are `u/<user>`, `g/<group>`, `f/<folder>` or `*`; `admin` is always a role, and `default_role: null` means no default. A data table the value omits is no longer under roles. Only a data table on a cluster Windmill manages can be. |
| `protection_rules` | `{ <rule name>: { rules: [...], bypass_groups: [...], bypass_users: [...], fork_role_mapping?: { <role here>: <role in its forks> } } }`, rules among `DisableDirectDeployment`, `DisableWorkspaceForking`, `RestrictDeployToDeployers`, `RestrictAnonymousAppDeployment`, `RestrictPublicRunSharing`, `RestrictGuestAppDeployment`, `RequireIsolatedForks`. A rule the value omits is deleted. The dev workspace lock is not listed and cannot be set here. |
| `ducklake` | `{ ducklakes: { <name>: { catalog, storage, ... } } }`. |
| `dbt_warehouses` | dbt warehouse connections. |
| `default_app` | Path of the app shown as the workspace home. |
| `default_scripts` | `{ order?, hidden?, default_script_content? }` for the script language picker. |
| `deploy_ui` | `{ include_path?, include_type? }`: what the deploy-to-workspace UI offers. |
| `operator_settings` | What operators see and do: page visibility flags (`runs`, `schedules`, `resources`, `variables`, `assets`, `audit_logs`, `groups`, `folders`, `workers`, `triggers`) and write rights (`manage_schedules`, `manage_triggers`, `builder_flows`, `builder_apps`). A write right the value omits keeps its stored value. |
| `role_overrides` | `{ <role>: { label?, capabilities: { code?, flows?, apps?, manage_schedules?, manage_triggers?, fork?, visibility? } } }`: this workspace's overrides of roles the instance defines. A capability left out follows the instance definition. A role the instance does not define is refused, not skipped. |
| `workspace_only_roles` | Same shape: roles this workspace adds (Enterprise only). A role the instance defines is refused here. |
| `mute_critical_alerts` | Mute the workspace's critical alerts. |
| `slack_team_id`, `slack_name` | Read only: set by connecting Slack on the settings page. |
| `slack_command_script` / `teams_command_script` | Script or flow handling the Slack / Teams command. |
| `slack_oauth_client_id` / `slack_oauth_client_secret` | The workspace's own Slack OAuth app. Clearing either removes both. |
| `public_app_execution_limit_per_minute` | Rate limit for public apps, `null` for none. |
| `guest_jwt_public_key`, `guest_jwt_jwks_url` | The key that signs guest JWTs for apps set to Guests. |
| `add_admins_and_developers_to_forks` | Add the workspace's admins and developers to each fork. |
| `color` | Workspace color, `#rrggbb`. |
<!-- cli-only -->

## From the command line

`wmill workspace get-settings` prints the document (`--encrypted` for secrets encrypted with the
workspace key, `--json` for JSON). `wmill workspace apply-settings <file>` applies a partial
document from a YAML or JSON file: it shows the changes and asks before writing (`--dry-run` to
only show them, `-y` to skip the question, `--replace` to clear what the file omits).

## In a synced repository

With `includeSettings: true` in `wmill.yaml`, `wmill sync pull` writes the document to
`settings.yaml`, secrets encrypted with the workspace key (plaintext only with `plainSecrets`), and
`wmill sync push` applies it. Unlike a partial apply, a push treats the file as the whole
document: a setting the file omits is cleared, except `operator_settings`, `color`, the auto-invite
instance groups, and the keys `ducklake`, `dbt_warehouses`, `deploy_ui`, `teams_command_script`,
`public_app_execution_limit_per_minute`, `guest_jwt_public_key`, `guest_jwt_jwks_url`,
`error_handler_fallback_to_instance_alerts`, `add_admins_and_developers_to_forks`,
`role_overrides` and `workspace_only_roles`, which are managed only when the file states them.
`datatable_permissions` is pushed only with `includeDatatablePermissions: true` (or
`--include-datatable-permissions`). A push ignores `protection_rules`: `protection-rules.yaml` and
`wmill protection-rules push` sync them.
<!-- /cli-only -->
