use windmill_parser::asset_parser::{
        merge_assets, AssetKind, AssetUsageAccessType, ParseAssetsResult,
    };

use crate::{parse_ansible_reqs, ResourceOrVariablePath};

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let mut assets = vec![];
    if let (_, Some(ansible_reqs), _) = parse_ansible_reqs(input)? {
        if let Some(delegate_to_git_repo_details) = ansible_reqs.delegate_to_git_repo {
            assets.push(ParseAssetsResult {
                kind: AssetKind::Resource,
                path: delegate_to_git_repo_details.resource,
                access_type: Some(AssetUsageAccessType::R),
            })
        }

        for i in ansible_reqs.inventories {
            if let Some(pinned_res) = i.pinned_resource {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: pinned_res,
                    access_type: Some(AssetUsageAccessType::R),
                })
            }
        }

        for file in ansible_reqs.file_resources {
            if let ResourceOrVariablePath::Resource(resource) = file.resource_path {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: resource,
                    access_type: Some(AssetUsageAccessType::R),
                })
            }
        }
    }

    Ok(merge_assets(assets))
}

mod tests {
    use crate::{parse_ansible_sig, parse_delegate_to_git_repo};

    use super::*;

    #[test]
    fn test_parse_ansible_assets() {
        let p = r#"
---
inventory:
  - resource_type: ansible_inventory
    # You can pin an inventory to this script by hardcoding the resource path:
    # resource: u/user/your_resource
# - name: hcloud.yml
#   resource_type: dynamic_inventory

options:
  - verbosity: vvv

delegate_to_git_repo:
  resource: u/admin/git_reportino
  playbook: ./playbooks/playbook.yml
  commit: 7sh7dh73h7dhd299d91hd1hdh3d3hygh4372


# File resources will be written in the relative `target` location before
# running the playbook
files:
  - resource: u/user/fabulous_jinja_template
    target:  ./config_template.j2
  - variable: u/user/ssh_key
    target:  ./ssh_key
    mode: '0600'

# Define the arguments of the windmill script
extra_vars:
  world_qualifier:
    type: string

# If using Ansible Vault:
# vault_password: u/user/ansible_vault_password

dependencies:
  galaxy:
    collections:
      - name: community.general
      - name: community.vmware
    roles:
  python:
    - jmespath
---
- name: Echo
  hosts: 127.0.0.1
  connection: local
  vars:
    my_result:
      a: 2
      b: true
      c: "Hello"

  tasks:
  - name: Print debug message
    debug:
      msg: "Hello, {{world_qualifier}} world!"
  - name: Write variable my_result to result.json
    delegate_to: localhost
    copy:
      content: "{{ my_result | to_json }}"
      dest: result.json
"#;
        let a = parse_assets(p).unwrap();
        println!("The resulting assets are: {}", a.len());

        let a = parse_ansible_reqs(p).unwrap();
        println!("The resulting reqs are: {:#?}", a);

        let a = parse_ansible_sig(p).unwrap();
        println!("The resulting sig is: {:#?}", a);

        let a = parse_delegate_to_git_repo(p).unwrap();
        println!("The resulting delegate_to_kit_repo is: {:#?}", a);
    }
}
