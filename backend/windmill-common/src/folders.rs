use globset::Glob;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderDefaultPermissionedAsRule {
    pub path_glob: String,
    pub permissioned_as: String,
}

/// Returns the first matching default `permissioned_as` for `path` in its folder, if any.
///
/// `path` is the full item path (e.g. `f/prod/jobs/run_a`). The glob is matched against
/// the path *relative* to the folder root (`jobs/run_a` in the example) so rules can
/// never match outside their own folder. Returns `None` if the path is not under a
/// folder, if the folder has no rules, or if no rule matches.
///
/// If a rule matches but its `permissioned_as` resolves to a user or group that no
/// longer exists, returns `Error::BadRequest` so the caller surfaces a clean error
/// rather than silently creating an item owned by nobody.
pub async fn resolve_folder_default_permissioned_as(
    db: &Pool<Postgres>,
    w_id: &str,
    path: &str,
) -> Result<Option<String>> {
    let Some(rest) = path.strip_prefix("f/") else {
        return Ok(None);
    };
    let Some((folder_name, relative_path)) = rest.split_once('/') else {
        return Ok(None);
    };
    if relative_path.is_empty() {
        return Ok(None);
    }

    let rules_value = sqlx::query_scalar!(
        "SELECT default_permissioned_as FROM folder WHERE workspace_id = $1 AND name = $2",
        w_id,
        folder_name
    )
    .fetch_optional(db)
    .await?;

    let Some(rules_value) = rules_value else {
        return Ok(None);
    };

    let rules: Vec<FolderDefaultPermissionedAsRule> =
        serde_json::from_value(rules_value).unwrap_or_default();

    for rule in rules {
        let Ok(glob) = Glob::new(&rule.path_glob) else {
            continue;
        };
        if !glob.compile_matcher().is_match(relative_path) {
            continue;
        }
        ensure_permissioned_as_exists(db, w_id, folder_name, &rule).await?;
        return Ok(Some(rule.permissioned_as));
    }

    Ok(None)
}

/// Email-valued variant of [`resolve_folder_default_permissioned_as`]. Used by
/// flows and scripts which store `on_behalf_of_email` rather than `permissioned_as`.
pub async fn resolve_folder_default_on_behalf_of_email(
    db: &Pool<Postgres>,
    w_id: &str,
    path: &str,
) -> Result<Option<String>> {
    let Some(permissioned_as) = resolve_folder_default_permissioned_as(db, w_id, path).await?
    else {
        return Ok(None);
    };
    let email = crate::users::get_email_from_permissioned_as(&permissioned_as, w_id, db).await?;
    Ok(Some(email))
}

async fn ensure_permissioned_as_exists(
    db: &Pool<Postgres>,
    w_id: &str,
    folder_name: &str,
    rule: &FolderDefaultPermissionedAsRule,
) -> Result<()> {
    if let Some(username) = rule.permissioned_as.strip_prefix("u/") {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND username = $2)",
            w_id,
            username
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::BadRequest(format!(
                "Folder '{folder_name}' default_permissioned_as rule '{}' resolves to user '{}' which does not exist in this workspace. Fix the folder rule and try again.",
                rule.path_glob, rule.permissioned_as
            )));
        }
    } else if let Some(group) = rule.permissioned_as.strip_prefix("g/") {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_ WHERE workspace_id = $1 AND name = $2)",
            w_id,
            group
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::BadRequest(format!(
                "Folder '{folder_name}' default_permissioned_as rule '{}' resolves to group '{}' which does not exist in this workspace. Fix the folder rule and try again.",
                rule.path_glob, rule.permissioned_as
            )));
        }
    } else if rule.permissioned_as.contains('@') {
        let exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM usr WHERE workspace_id = $1 AND email = $2)",
            w_id,
            &rule.permissioned_as
        )
        .fetch_one(db)
        .await?
        .unwrap_or(false);
        if !exists {
            return Err(Error::BadRequest(format!(
                "Folder '{folder_name}' default_permissioned_as rule '{}' resolves to email '{}' which does not exist in this workspace. Fix the folder rule and try again.",
                rule.path_glob, rule.permissioned_as
            )));
        }
    } else {
        return Err(Error::BadRequest(format!(
            "Folder '{folder_name}' default_permissioned_as rule '{}' has an unrecognised permissioned_as format '{}'. Expected u/<username>, g/<groupname>, or an email.",
            rule.path_glob, rule.permissioned_as
        )));
    }
    Ok(())
}
