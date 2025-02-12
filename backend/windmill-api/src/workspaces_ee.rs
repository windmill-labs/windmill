use crate::{
    db::{ApiAuthed, DB},
    workspaces::EditAutoInvite,
};

pub async fn edit_auto_invite(
    _authed: ApiAuthed,
    _db: DB,
    _w_id: String,
    _ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented on OSS".to_string(),
    ))
}

pub async fn get_github_app_token(_authed: ApiAuthed) -> windmill_common::error::Result<String> {
    Err(windmill_common::error::Error::internal_err(
        "Not implemented on OSS".to_string(),
    ))
}
