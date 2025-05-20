#[cfg(feature = "private")]
use crate::workspaces_ee;

use crate::{
    db::{ApiAuthed, DB},
    workspaces::EditAutoInvite,
};

pub async fn edit_auto_invite(
    authed: ApiAuthed,
    db: DB,
    w_id: String,
    ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    #[cfg(feature = "private")]
    {
        return workspaces_ee::edit_auto_invite(authed, db, w_id, ea).await;
    }
    #[cfg(not(feature = "private"))]
    {
        let _ = (authed, db, w_id, ea); // Mark params as used, as original had _
        Err(windmill_common::error::Error::internal_err(
            "Not implemented on OSS".to_string(),
        ))
    }
}
