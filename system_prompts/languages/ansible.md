# Ansible

Windmill runs Ansible playbooks with `ansible-playbook`. A script is a single YAML
document made of two parts separated by a `---` line: a Windmill **header** and one or
more standard Ansible **plays**.

## Structure

```yaml
---
# Windmill header: configures inventories, file resources, arguments and dependencies
extra_vars:
  world_qualifier:
    type: string
dependencies:
  galaxy:
    collections:
      - name: community.general
  python:
    - jmespath
---
# Standard Ansible plays
- name: Echo
  hosts: 127.0.0.1
  connection: local
  tasks:
    - name: Print debug message
      debug:
        msg: "Hello, {{ world_qualifier }} world!"
```

## Header

The header is **not** standard Ansible — it is parsed by Windmill to build the script's
inputs and runtime environment. Supported keys:

- `extra_vars`: defines the script arguments. Each entry is passed to the playbook via
  `--extra-vars` and becomes a Jinja variable usable as `{{ name }}` in the plays. Give
  each argument a `type` (`string`, `number`, `boolean`, `object`, ...) so Windmill can
  generate the input form.
- `inventory`: lists inventories. Use `resource_type: ansible_inventory` (optionally
  pinned with `resource: u/user/your_resource`) or `resource_type: dynamic_inventory`.
- `files`: writes Windmill resources/variables to files before the run, e.g.
  `- resource: u/user/template` with `target: ./config.j2`, or
  `- variable: u/user/ssh_key` with `target: ./ssh_key` and `mode: '0600'`.
- `dependencies`: `galaxy` collections/roles (installed with `ansible-galaxy`) and
  `python` pip packages available to the playbook.
- `options`: extra `ansible-playbook` flags such as `- verbosity: vvv`.
- `vault_password`: a Windmill variable path to use as the Ansible Vault password.

## Arguments

Reference header `extra_vars` directly as Jinja variables in the plays:

```yaml
extra_vars:
  name:
    type: string
  count:
    type: number
---
- hosts: localhost
  tasks:
    - debug:
        msg: "{{ name }} x {{ count }}"
```

## Environment variables

Windmill contextual variables are available as environment variables and read with the
`env` lookup:

```yaml
- debug:
    msg: "Running in workspace {{ lookup('env', 'WM_WORKSPACE') }}"
```

## Output

To return a result, write JSON to a `result.json` file in the job directory:

```yaml
- hosts: localhost
  tasks:
    - name: Write result
      copy:
        content: "{{ { 'ok': true, 'value': 42 } | to_json }}"
        dest: result.json
```
