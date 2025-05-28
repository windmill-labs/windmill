/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2042
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use argon2::Argon2;
use serde::Serialize;
use std::sync::Arc;
use windmill_common::error;

pub async fn create_user<T: Serialize>(
    db: DB,
    w_id: String,
    authed: ApiAuthed,
    email: String,
    password: String,
    super_admin: Option<bool>,
    name: Option<String>,
    company: Option<String>,
    username: String,
    invite_authed: ApiAuthed,
    is_admin: Option<bool>,
    is_operator: Option<bool>,
    role: Option<String>,
    groups: Option<Vec<String>>,
    oidc_only: Option<bool>,
) -> error::Result<(String, T)> {
    crate::users_ee::create_user(
        db,
        w_id,
        authed,
        email,
        password,
        super_admin,
        name,
        company,
        username,
        invite_authed,
        is_admin,
        is_operator,
        role,
        groups,
        oidc_only,
    )
    .await
}

pub async fn set_password(
    db: DB,
    w_id: String,
    authed: ApiAuthed,
    username: String,
    password: String,
    argon2: Arc<Argon2<'_>>,
) -> error::Result<String> {
    crate::users_ee::set_password(db, w_id, authed, username, password, argon2).await
}

pub fn send_email_if_possible(subject: &str, content: &str, to: &str) {
    crate::users_ee::send_email_if_possible(subject, content, to)
}