use std::collections::BTreeSet;

use windmill_parser::{
    asset_parser::{
        merge_assets, parse_pipeline_annotations, AssetKind, AssetUsageAccessType,
        ParseAssetsOutput, ParseAssetsResult,
    },
    item_reference::{ItemKind, ItemReference, ReferenceAccess},
};

use crate::{parse_ansible_reqs, AnsibleRequirements, ResourceOrVariablePath};

pub fn parse_assets(input: &str) -> anyhow::Result<ParseAssetsOutput> {
    let mut assets = vec![];
    let mut references = vec![];
    if let (_, Some(ansible_reqs), _) = parse_ansible_reqs(input)? {
        references = item_references(&ansible_reqs);
        if let Some(delegate_to_git_repo_details) = ansible_reqs.delegate_to_git_repo {
            assets.push(ParseAssetsResult {
                kind: AssetKind::Resource,
                path: delegate_to_git_repo_details.resource,
                access_type: Some(AssetUsageAccessType::R),
                columns: None,
            })
        }

        for i in ansible_reqs.inventories {
            if let Some(pinned_res) = i.pinned_resource {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: pinned_res,
                    access_type: Some(AssetUsageAccessType::R),
                    columns: None,
                })
            }
        }

        for file in ansible_reqs.file_resources {
            if let ResourceOrVariablePath::Resource(resource) = file.resource_path {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: resource,
                    access_type: Some(AssetUsageAccessType::R),
                    columns: None,
                })
            }
        }
    }

    let pipeline = parse_pipeline_annotations(input);
    let mut output = ParseAssetsOutput::new(merge_assets(assets), Vec::new(), pipeline);
    output.references = references;
    Ok(output)
}

/// The resources and variables the Ansible executor reads with the job's identity.
fn item_references(reqs: &AnsibleRequirements) -> Vec<ItemReference> {
    let resource = |p: &str| ItemReference::of(ItemKind::Resource, ReferenceAccess::Read, p);
    let variable = |p: &str| ItemReference::of(ItemKind::Variable, ReferenceAccess::Read, p);
    let mut found = BTreeSet::new();
    found.extend(
        reqs.delegate_to_git_repo
            .iter()
            .filter_map(|d| resource(&d.resource)),
    );
    found.extend(
        reqs.inventories
            .iter()
            .filter_map(|i| i.pinned_resource.as_deref().and_then(resource)),
    );
    found.extend(
        reqs.file_resources
            .iter()
            .filter_map(|f| match &f.resource_path {
                ResourceOrVariablePath::Resource(p) => resource(p),
                ResourceOrVariablePath::Variable(p) => variable(p),
            }),
    );
    found.extend(reqs.vars.iter().filter_map(|(_, v)| variable(v)));
    found.extend(reqs.resources.iter().filter_map(|(_, r)| resource(r)));
    found.extend(reqs.vault_password.as_deref().and_then(variable));
    found.extend(reqs.git_ssh_identity.iter().filter_map(|p| variable(p)));
    found.into_iter().collect()
}
