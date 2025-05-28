use crate::db::DB;
use windmill_common::users::ApiAuthed;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EditAutoInvite;

pub async fn edit_auto_invite(
    _authed: ApiAuthed,
    _db: DB,
    _w_id: String,
    _ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    crate::workspaces_ee::edit_auto_invite(_authed, _db, _w_id, _ea).await
}