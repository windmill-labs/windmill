use crate::{
    db::{ApiAuthed, DB},
    workspaces::EditAutoInvite,
};

pub async fn edit_auto_invite(
    _authed: ApiAuthed,
    _db: DB,
    _rsmq: Option<rsmq_async::MultiplexedRsmq>,
    _w_id: String,
    _ea: EditAutoInvite,
) -> windmill_common::error::Result<String> {
    Err(windmill_common::error::Error::InternalErr(
        "Not implemented on OSS".to_string(),
    ))
}
