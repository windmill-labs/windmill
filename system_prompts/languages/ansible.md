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
