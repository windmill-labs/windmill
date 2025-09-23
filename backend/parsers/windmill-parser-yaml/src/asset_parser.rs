use anyhow::anyhow;
use windmill_parser::{
    asset_parser::{
        merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
    },
    MainArgSignature,
};
use yaml_rust::{yaml::YamlIter, Yaml, YamlLoader};

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let docs =
        YamlLoader::load_from_str(input).map_err(|e| anyhow!("Failed to parse yaml: {}", e))?;

    if docs.len() < 2 {
        return Ok(vec![]);
    }

    let assets = get_assets(&docs[0])?;


    Ok(merge_assets(assets))
}

fn get_assets(doc: &Yaml) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let mut assets = vec![]; if let Yaml::Hash(doc) = doc {
        if let Some(Yaml::String(delegate)) =
            doc.get(&Yaml::String("delegate_to_git_repo".to_string()))
        {
            assets.push(ParseAssetsResult {
                kind: AssetKind::Resource,
                path: delegate.to_string(),
                access_type: Some(AssetUsageAccessType::R),
            })
        }
    }

    Ok(assets)
}

mod tests {
    use super::*;

    #[test]
    fn test_parse_ansible_assets() {
        let a = parse_assets(
            r#"
---
inventory:
  - resource_type: ansible_inventory
    # You can pin an inventory to this script by hardcoding the resource path:
    # resource: u/user/your_resource
# - name: hcloud.yml
#   resource_type: dynamic_inventory

options:
  - verbosity: vvv

delegate_to_git_repo: u/admin/git_reporino

# File resources will be written in the relative `target` location before
# running the playbook
# files:
  # - resource: u/user/fabulous_jinja_template
  #   target:  ./config_template.j2
  # - variable: u/user/ssh_key
  #   target:  ./ssh_key
  #   mode: '0600'

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
"#,
        )
        .unwrap();

        println!("The resulting assets are: {}", a.len());
    }
}
