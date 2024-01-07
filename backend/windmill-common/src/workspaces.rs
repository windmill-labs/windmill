use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WorkspaceGitRepo {
    pub script_path: String,
    pub git_repo_resource_path: String,
    pub use_individual_branch: Option<bool>,
}
