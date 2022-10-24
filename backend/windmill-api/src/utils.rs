pub use windmill_common::utils::*;

pub async fn require_super_admin<'c>(
    db: &mut Transaction<'c, Postgres>,
    email: Option<String>,
) -> Result<()> {
    let is_admin = sqlx::query_scalar!(
        "SELECT super_admin FROM password WHERE email = $1",
        email.as_ref()
    )
    .fetch_one(db)
    .await
    .map_err(|e| Error::InternalErr(format!("fetching super admin: {e}")))?;
    if !is_admin {
        Err(Error::NotAuthorized(
            "This endpoint require caller to be a super admin".to_owned(),
        ))
    } else {
        Ok(())
    }
}
