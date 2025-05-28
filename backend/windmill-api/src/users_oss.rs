use axum::http::StatusCode;
use windmill_common::error::Error;

pub async fn create_user(
    _authed: windmill_common::users::Authed,
    _extract_path: axum::extract::Path<String>,
    _db: crate::db::DB,
    _new_user: axum::extract::Json<serde_json::Value>,
) -> Result<(StatusCode, String), Error> {
    crate::users_ee::create_user(_authed, _extract_path, _db, _new_user).await
}

pub async fn set_password(
    _authed: windmill_common::users::Authed,
    _extract_path: axum::extract::Path<String>,
    _db: crate::db::DB,
    _set_password: axum::extract::Json<serde_json::Value>,
) -> Result<String, Error> {
    crate::users_ee::set_password(_authed, _extract_path, _db, _set_password).await
}

pub fn send_email_if_possible(_subject: &str, _content: &str, _to: &str) {
    crate::users_ee::send_email_if_possible(_subject, _content, _to)
}