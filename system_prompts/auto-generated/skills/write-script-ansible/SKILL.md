---
name: write-script-ansible
description: MUST use when writing Ansible playbooks.
---

## CLI Commands

Place scripts in a folder.

After writing, tell the user which command fits what they want to do:

- `wmill script preview <script_path>` — **default when iterating on a local script.** Runs the local file without deploying.
- `wmill script run <path>` — runs the script **already deployed** in the workspace. Use only when the user explicitly wants to test the deployed version, not local edits.
- `wmill generate-metadata` — regenerate the local `.script.yaml` (input schema) and `.lock` (resolved dependencies) for scripts you changed, and refresh their content hashes in `wmill-lock.yaml`. Local files only — **not** a deploy. See "Keep metadata in sync" below.
- `wmill sync push` — deploy local changes to the workspace. Only suggest/run this when the user explicitly asks to deploy/publish/push — not when they say "run", "try", or "test".

### Preview vs run — choose by intent, not habit

If the user says "run the script", "try it", "test it", "does it work" while there are **local edits to the script file**, use `script preview`. Do NOT push the script to then `script run` it — pushing is a deploy, and deploying just to test overwrites the workspace version with untested changes.

Only use `script run` when:
- The user explicitly says "run the deployed version" / "run what's on the server".
- There is no local script being edited (you're just invoking an existing script).

Only use `sync push` when:
- The user explicitly asks to deploy, publish, push, or ship.
- The preview has already validated the change and the user wants it in the workspace.

### Keep metadata in sync after editing

`wmill-lock.yaml` tracks a content hash for each item. Editing a script's content — most importantly **adding or removing an import** or **changing `main`'s arguments** — invalidates that hash and leaves the `.lock`, the `.script.yaml` input schema, and the hash row out of date. Run `wmill generate-metadata` (scoped to what you touched) after such edits so the resolved lock, the auto-generated args UI (driven by `.script.yaml`), and `wmill-lock.yaml` all match the code. Leaving them stale produces spurious diffs in git-sync and CI.

This only writes local files (it is **not** a deploy), but it re-resolves dependencies, so it can bump unpinned versions (the same as deploying from the UI; expected, not a bug). So by default offer it and run it once the user agrees, rather than running it silently after every edit — unless the project's `AGENTS.md` opts into running metadata automatically (see the "Keeping metadata in sync" preference there). Either way YOU run the command, not the user. After running it, diff the regenerated `.lock` / `.script.lock` files and tell the user which dependency versions changed (e.g. `requests 2.31.0 → 2.32.0`), so they can catch an unwanted bump before deploying — even under `Metadata: auto`, since it's information, not a confirmation gate. Pin versions in code to keep them fixed.

With no path argument, `generate-metadata` regenerates only the items whose content hash drifted — not everything. Imports propagate: editing a script that others import marks every importer stale too, so a one-line change to a shared module can regenerate many locks (by design — their locks must reflect the imported code). If it touches more than you expect, run `wmill generate-metadata --dry-run` — it lists each stale item with a reason (`content changed` or `depends on <path>`) without changing anything — then narrow with a path argument (`wmill generate-metadata f/foo`) or `--strict-folder-boundaries`.

If the on-disk `.lock` and `.script.yaml` are already correct and only `wmill-lock.yaml` needs its hashes refreshed (hash drift, or bootstrapping missing entries), use `wmill generate-metadata rehash` — it re-records hashes from disk with no backend round-trip and no dependency changes.

### After writing — offer to test, don't wait passively

If the user hasn't already told you to run/test/preview the script, offer it as a one-sentence next step (e.g. "Want me to run `wmill script preview` with sample args?"). Do not present a multi-option menu.

If the user already asked to test/run/try the script in their original request, skip the offer and just execute `wmill script preview <path> -d '<args>'` directly — pick plausible args from the script's declared parameters. The shape varies by language: `main(...)` for code languages, the SQL dialect's own placeholder syntax (`$1` for PostgreSQL, `?` for MySQL/Snowflake, `@P1` for MSSQL, `@name` for BigQuery, etc.), positional `$1`, `$2`, … for Bash, `param(...)` for PowerShell.

`wmill script preview` does not deploy, but it still executes script code and may cause side effects; run it yourself when the user asked to test/preview (or after confirming that execution is intended). `wmill generate-metadata` does not deploy either — it only writes local files (locks, schemas, hashes) — but offer it before running (or run automatically if the project's `AGENTS.md` opts in), per "Keep metadata in sync" above. Only `wmill sync push` deploys to the workspace — run it only when the user explicitly asks to deploy/publish/push.

For a **visual** open-the-script-in-the-dev-page preview (rather than `script preview`'s run-and-print-result), use the `preview` skill.

Use `wmill resource-type list --schema` to discover available resource types.

# Ansible

A Windmill Ansible script is a YAML document with two parts separated by a `---` line:

1. A **Windmill header** that declares the script's inputs, dependencies, inventory, file resources and options.
2. One or more **Ansible plays** (the standard playbook content) executed with `ansible-playbook`.

## Structure

```yaml
---
inventory:
  - resource_type: ansible_inventory
    # Pin an inventory by hardcoding the resource path (optional):
    # resource: u/user/your_inventory

options:
  - verbosity: vvv

# File resources/variables are written to their relative `target` location
# before the playbook runs:
# files:
#   - resource: u/user/fabulous_jinja_template
#     target: ./config_template.j2
#   - variable: u/user/ssh_key
#     target: ./ssh_key
#     mode: '0600'

# Inputs of the Windmill script. Each key becomes an Ansible variable usable
# with {{ var_name }}. The JSON Schema is inferred from these declarations.
extra_vars:
  world_qualifier:
    type: string

# If using Ansible Vault:
# vault_password: u/user/ansible_vault_password

dependencies:
  galaxy:
    collections:
      - name: community.general
    roles:
  python:
    - jmespath
---
- name: Echo
  hosts: 127.0.0.1
  connection: local
  tasks:
    - name: Print debug message
      debug:
        msg: "Hello, {{ world_qualifier }} world!"
```

## Inputs (`extra_vars`)

- Each entry under `extra_vars` declares a Windmill script argument and is passed to the playbook as an Ansible variable (`--extra-vars`).
- Reference them in tasks with Jinja: `{{ world_qualifier }}`.
- Supported `type` values: `string`, `number`, `boolean`, `object` (and nested schemas). This is what generates the script's input form.

## Inventory

- `inventory:` with `resource_type: ansible_inventory` lets the user select an inventory resource at runtime; add `resource: u/user/...` to pin one.
- `dynamic_inventory` entries reference a `name` (e.g. `hcloud.yml`) for cloud/dynamic inventories.

## File resources and variables

Under `files:`, materialize Windmill resources or variables onto disk before the run:
- `resource: <path>` or `variable: <path>` — the Windmill object to fetch.
- `target: ./relative/path` — where to write it (relative paths only).
- `mode: '0600'` — optional octal file permission (useful for SSH keys).

## Dependencies

- `dependencies.galaxy.collections` / `dependencies.galaxy.roles` — installed via `ansible-galaxy`.
- `dependencies.python` — pip packages available to Ansible modules (e.g. `jmespath` for the `json_query` filter).

## Output

The script result is read from a `result.json` file in the job directory. Write it from a task, e.g.:

```yaml
- name: Write result
  delegate_to: localhost
  copy:
    content: "{{ my_result | to_json }}"
    dest: result.json
```

## Notes

- The header keys (`inventory`, `extra_vars`, `files`, `options`, `vault_password`, `dependencies`) are Windmill-specific — do not confuse them with Ansible's own keys.
- Use `hosts: 127.0.0.1` with `connection: local` for local tasks; otherwise rely on the selected inventory.
- Keep the `---` separator between the Windmill header and the plays.
