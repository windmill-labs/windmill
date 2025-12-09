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
                specific_table: None,
            })
        }

        for i in ansible_reqs.inventories {
            if let Some(pinned_res) = i.pinned_resource {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: pinned_res,
                    access_type: Some(AssetUsageAccessType::R),
                    specific_table: None,
                })
            }
        }

        for file in ansible_reqs.file_resources {
            if let ResourceOrVariablePath::Resource(resource) = file.resource_path {
                assets.push(ParseAssetsResult {
                    kind: AssetKind::Resource,
                    path: resource,
                    access_type: Some(AssetUsageAccessType::R),
                    specific_table: None,
                })
            }
        }
    }

    Ok(merge_assets(assets))
}
