pub mod folder_history;
pub mod folders;
pub mod granular_acls;
pub mod groups;

use windmill_api_auth::ApiAuthed;
use windmill_common::{error::Error, worker::CLOUD_HOSTED};

/// The public demo workspace on the managed cloud is kept clean and consistent by
/// restricting folder creation, item sharing, and group creation for non-admins.
/// `action` is a short noun phrase completing "… is disabled …" (e.g.
/// "Folder creation", "Sharing"). Returns `Err(BadRequest)` when the caller is blocked.
pub fn check_demo_workspace_restriction(
    authed: &ApiAuthed,
    w_id: &str,
    action: &str,
) -> Result<(), Error> {
    if *CLOUD_HOSTED && w_id == "demo" && !authed.is_admin {
        return Err(Error::BadRequest(format!(
            "{action} is disabled in the demo workspace. Create your own workspace to keep the demo clean and consistent."
        )));
    }
    Ok(())
}
