/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_common::error::{Error, Result};
use windmill_common::DB;

use crate::ApiAuthed;

/// Check if the user is an owner of the given path.
pub fn require_owner_of_path(authed: &ApiAuthed, path: &str) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    if !path.is_empty() {
        let splitted = path.split("/").collect::<Vec<&str>>();
        if splitted[0] == "u" {
            if splitted[1] == authed.username {
                Ok(())
            } else {
                Err(Error::BadRequest(format!(
                    "only the owner {} is authorized to perform this operation",
                    splitted[1]
                )))
            }
        } else if splitted[0] == "f" {
            require_is_folder_owner(authed, splitted[1])
        } else {
            Err(Error::BadRequest(format!(
                "Not recognized path kind: {}",
                path
            )))
        }
    } else {
        Err(Error::BadRequest(format!(
            "Cannot be owner of an empty path"
        )))
    }
}

pub fn is_folder_owner(
    ApiAuthed { is_admin, folders, .. }: &ApiAuthed,
    name: &str,
) -> bool {
    if *is_admin {
        true
    } else {
        folders.into_iter().any(|x| x.0 == name && x.2)
    }
}

pub fn require_is_folder_owner(authed: &ApiAuthed, name: &str) -> Result<()> {
    if is_folder_owner(authed, name) {
        Ok(())
    } else {
        Err(Error::NotAuthorized(format!(
            "You are not owner of the folder {}",
            name
        )))
    }
}

pub fn get_perm_in_extra_perms_for_authed(
    v: serde_json::Value,
    authed: &ApiAuthed,
) -> Option<bool> {
    match v {
        serde_json::Value::Object(obj) => {
            let mut keys = vec![format!("u/{}", authed.username)];
            for g in authed.groups.iter() {
                keys.push(format!("g/{}", g));
            }
            let mut res = None;
            for k in keys {
                if let Some(v) = obj.get(&k) {
                    if let Some(v) = v.as_bool() {
                        if v {
                            return Some(true);
                        }
                        res = Some(v);
                    }
                }
            }
            res
        }
        _ => None,
    }
}

/// Generic require_is_writer with a configurable SQL query.
pub async fn require_is_writer(
    authed: &ApiAuthed,
    path: &str,
    w_id: &str,
    db: DB,
    query: &str,
    kind: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    if !path.is_empty() {
        if require_owner_of_path(authed, path).is_ok() {
            return Ok(());
        }
        if path.starts_with("f/") && path.split('/').count() >= 2 {
            let folder = path.split('/').nth(1).unwrap();
            let extra_perms = sqlx::query_scalar!(
                "SELECT extra_perms FROM folder WHERE name = $1 AND workspace_id = $2",
                folder,
                w_id
            )
            .fetch_optional(&db)
            .await?;
            if let Some(perms) = extra_perms {
                let is_folder_writer =
                    get_perm_in_extra_perms_for_authed(perms, authed).unwrap_or(false);
                if is_folder_writer {
                    return Ok(());
                }
            }
        }
        let extra_perms = sqlx::query_scalar(query)
            .bind(path)
            .bind(w_id)
            .fetch_optional(&db)
            .await?;
        if let Some(perms) = extra_perms {
            let perm = get_perm_in_extra_perms_for_authed(perms, authed);
            match perm {
                Some(true) => Ok(()),
                Some(false) => Err(Error::BadRequest(format!(
                    "User {} is not a writer of {kind} path {path}",
                    authed.username
                ))),
                None => Err(Error::BadRequest(format!(
                    "User {} has neither read or write permission on {kind} {path}",
                    authed.username
                ))),
            }
        } else {
            Err(Error::BadRequest(format!(
                "{path} does not exist yet and user {} is not an owner of the parent folder",
                authed.username
            )))
        }
    } else {
        Err(Error::BadRequest(format!(
            "Cannot be writer of an empty path"
        )))
    }
}
