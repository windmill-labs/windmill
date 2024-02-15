use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct WorkspaceGitSyncSettings {
    pub include_path: Vec<String>,
    pub include_type: Vec<ObjectType>,
    pub repositories: Vec<GitRepositorySettings>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum ObjectType {
    Script,
    Flow,
    App,
    Folder,
    Resource,
    Variable,
    Secret,
    Schedule,
    ResourceType,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GitRepositorySettings {
    pub script_path: String,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
    pub exclude_types_override: Option<Vec<ObjectType>>,
}
