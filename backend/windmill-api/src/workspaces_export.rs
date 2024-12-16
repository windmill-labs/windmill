/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::collections::HashMap;

use crate::db::ApiAuthed;

use crate::{
    apps::AppWithLastVersion,
    db::DB,
    folders::Folder,
    resources::{Resource, ResourceType},
};

use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
};

use http::HeaderName;
use itertools::Itertools;

use windmill_common::db::UserDB;
use windmill_common::schedule::Schedule;
use windmill_common::variables::build_crypt;

use windmill_common::{
    error::{to_anyhow, Error, Result},
    flows::Flow,
    scripts::{Schema, Script, ScriptLang},
    variables::ExportableListableVariable,
};

use crate::variables::decrypt;
use hyper::header;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tempfile::TempDir;
use tokio::fs::File;
use tokio_util::io::ReaderStream;

#[derive(Serialize)]
struct ScriptMetadata {
    summary: String,
    description: String,
    schema: Option<Schema>,
    lock: Option<String>,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    envs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrent_limit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    concurrency_time_window_s: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_ttl: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dedicated_worker: Option<bool>,
    #[serde(skip_serializing_if = "is_none_or_false")]
    ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_after_use: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart_unless_cancelled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible_to_runner_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_main_func: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codebase: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_preprocessor: Option<bool>,
}

pub fn is_none_or_false(val: &Option<bool>) -> bool {
    match val {
        Some(val) => !val,
        None => true,
    }
}

enum ArchiveImpl {
    #[cfg(feature = "zip")]
    Zip(async_zip::tokio::write::ZipFileWriter<tokio::fs::File>),
    Tar(tokio_tar::Builder<File>),
}

impl ArchiveImpl {
    async fn write_to_archive(&mut self, content: &str, path: &str) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => {
                let bytes = content.as_bytes();
                let mut header = tokio_tar::Header::new_gnu();
                header.set_size(bytes.len() as u64);
                header.set_mtime(0);
                header.set_uid(0);
                header.set_gid(0);
                header.set_mode(0o777);
                header.set_cksum();
                t.append_data(&mut header, path, bytes).await?;
            }
            #[cfg(feature = "zip")]
            ArchiveImpl::Zip(z) => {
                let header =
                    async_zip::ZipEntryBuilder::new(path.into(), async_zip::Compression::Deflate)
                        .last_modification_date(Default::default())
                        .unix_permissions(0o777)
                        .build();
                z.write_entry_whole(header, content.as_bytes())
                    .await
                    .map_err(to_anyhow)?;
            }
        }
        Ok(())
    }
    async fn finish(self) -> Result<()> {
        match self {
            ArchiveImpl::Tar(t) => t.into_inner().await?,
            #[cfg(feature = "zip")]
            ArchiveImpl::Zip(z) => z.close().await.map_err(to_anyhow)?.into_inner(),
        }
        .sync_all()
        .await?;

        Ok(())
    }
}

#[derive(Deserialize)]
pub(crate) struct ArchiveQueryParams {
    archive_type: Option<String>,
    plain_secret: Option<bool>,
    plain_secrets: Option<bool>,
    skip_secrets: Option<bool>,
    skip_variables: Option<bool>,
    skip_resources: Option<bool>,
    include_schedules: Option<bool>,
    include_users: Option<bool>,
    include_groups: Option<bool>,
    include_settings: Option<bool>,
    include_key: Option<bool>,
    default_ts: Option<String>,
}

#[inline]
pub fn to_string_without_metadata<T>(
    value: &T,
    preserve_extra_perms: bool,
    ignore_keys: Option<Vec<&str>>,
) -> Result<String>
where
    T: ?Sized + Serialize,
{
    let mut value = serde_json::to_value(value).map_err(to_anyhow)?;
    value
        .as_object_mut()
        .map(|obj| {
            let keys = [
                vec![
                    "workspace_id",
                    "path",
                    "name",
                    "versions",
                    "id",
                    "created_at",
                    "updated_at",
                    "created_by",
                    "updated_by",
                    "edited_at",
                    "edited_by",
                    "archived",
                    "has_draft",
                    "draft_only",
                    "error",
                ],
                ignore_keys.unwrap_or(vec![]),
            ]
            .concat();

            for key in keys {
                if obj.contains_key(key) {
                    obj.remove(key);
                }
            }

            if let Some(o2) = obj.get_mut("policy").and_then(|x| x.as_object_mut()) {
                o2.remove("on_behalf_of");
                o2.remove("on_behalf_of_email");
            }
            if !preserve_extra_perms && obj.contains_key("extra_perms") {
                obj.remove("extra_perms");
            }

            serde_json::to_string_pretty(&obj).ok()
        })
        .flatten()
        .ok_or_else(|| Error::BadRequest("Impossible to serialize value".to_string()))
}

#[derive(Serialize)]
struct SimplifiedUser {
    username: String,
    role: String,
    disabled: bool,
    email: String,
}

#[derive(Serialize)]
struct SimplifiedGroup {
    name: String,
    summary: Option<String>,
    members: Vec<String>,
    admins: Vec<String>,
}

#[derive(Serialize)]
struct SimplifiedSettings {
    // slack_team_id: Option<String>,
    // slack_name: Option<String>,
    // slack_command_script: Option<String>,
    // slack_email: Option<String>,
    auto_invite_enabled: bool,
    auto_invite_as: String,
    auto_invite_mode: String,
    webhook: Option<String>,
    deploy_to: Option<String>,
    error_handler: Option<String>,
    error_handler_extra_args: Option<Value>,
    error_handler_muted_on_cancel: bool,
    ai_resource: Option<serde_json::Value>,
    code_completion_enabled: bool,
    large_file_storage: Option<Value>,
    git_sync: Option<Value>,
    default_app: Option<String>,
    default_scripts: Option<Value>,
    name: String,
}

pub(crate) async fn tarball_workspace(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(ArchiveQueryParams {
        archive_type,
        plain_secret,
        plain_secrets,
        skip_resources,
        skip_secrets,
        skip_variables,
        include_schedules,
        include_users,
        include_groups,
        include_settings,
        include_key,
        default_ts,
    }): Query<ArchiveQueryParams>,
) -> Result<([(HeaderName, String); 2], impl IntoResponse)> {
    // require_admin(authed.is_admin, &authed.username)?;

    let mut tx = user_db.begin(&authed).await?;

    let tmp_dir = TempDir::new_in("/tmp/windmill/")?;

    let name = match archive_type.as_deref() {
        Some("tar") | None => Ok(format!("windmill-{w_id}.tar")),
        Some("zip") => Ok(format!("windmill-{w_id}.zip")),
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    let file_path = tmp_dir.path().join(&name);
    let mut archive = match archive_type.as_deref() {
        Some("tar") | None => {
            let file = File::create(&file_path).await?;
            Ok(ArchiveImpl::Tar(tokio_tar::Builder::new(file)))
        }
        #[cfg(feature = "zip")]
        Some("zip") => {
            let file = tokio::fs::File::create(&file_path).await?;
            Ok(ArchiveImpl::Zip(
                async_zip::tokio::write::ZipFileWriter::with_tokio(file),
            ))
        }
        Some(t) => Err(Error::BadRequest(format!("Invalid Archive Type {t}"))),
    }?;
    {
        let folders = sqlx::query_as::<_, Folder>("SELECT * FROM folder WHERE workspace_id = $1")
            .bind(&w_id)
            .fetch_all(&mut *tx)
            .await?;

        for folder in folders {
            archive
                .write_to_archive(
                    &to_string_without_metadata(&folder, true, None).unwrap(),
                    &format!("f/{}/folder.meta.json", folder.name),
                )
                .await?;
        }
    }

    {
        let scripts = sqlx::query_as::<_, Script>(
            "SELECT * FROM script as o WHERE workspace_id = $1 AND archived = false
            AND created_at = (select max(created_at) from script where path = o.path AND \
             workspace_id = $1)",
        )
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        for script in scripts {
            let ext = match script.language {
                ScriptLang::Python3 => "py",
                ScriptLang::Deno => {
                    if default_ts.as_ref().is_some_and(|x| x == "bun") {
                        "deno.ts"
                    } else {
                        "ts"
                    }
                }
                ScriptLang::Go => "go",
                ScriptLang::Bash => "sh",
                ScriptLang::Powershell => "ps1",
                ScriptLang::Postgresql => "pg.sql",
                ScriptLang::Mysql => "my.sql",
                ScriptLang::Bigquery => "bq.sql",
                ScriptLang::Snowflake => "sf.sql",
                ScriptLang::Mssql => "ms.sql",
                ScriptLang::Graphql => "gql",
                ScriptLang::Nativets => "fetch.ts",
                ScriptLang::Bun | ScriptLang::Bunnative => {
                    if default_ts.as_ref().is_some_and(|x| x == "bun") {
                        "ts"
                    } else {
                        "bun.ts"
                    }
                }
                ScriptLang::Php => "php",
                ScriptLang::Rust => "rs",
                ScriptLang::Ansible => "playbook.yml",
                ScriptLang::CSharp => "cs",
            };
            archive
                .write_to_archive(&script.content, &format!("{}.{}", script.path, ext))
                .await?;

            let metadata = ScriptMetadata {
                summary: script.summary,
                description: script.description,
                schema: script.schema,
                kind: script.kind.to_string(),
                lock: script.lock,
                envs: script.envs,
                concurrent_limit: script.concurrent_limit,
                concurrency_time_window_s: script.concurrency_time_window_s,
                cache_ttl: script.cache_ttl,
                dedicated_worker: script.dedicated_worker,
                ws_error_handler_muted: script.ws_error_handler_muted,
                priority: script.priority,
                tag: script.tag,
                timeout: script.timeout,
                delete_after_use: script.delete_after_use,
                restart_unless_cancelled: script.restart_unless_cancelled,
                visible_to_runner_only: script.visible_to_runner_only,
                no_main_func: script.no_main_func,
                codebase: script.codebase,
                concurrency_key: script.concurrency_key,
                has_preprocessor: script.has_preprocessor,
            };
            let metadata_str = serde_json::to_string_pretty(&metadata).unwrap();
            archive
                .write_to_archive(&metadata_str, &format!("{}.script.json", script.path))
                .await?;
        }
    }

    if !skip_resources.unwrap_or(false) {
        let resources = sqlx::query_as!(
            Resource,
            "SELECT * FROM resource WHERE workspace_id = $1 AND resource_type != 'state' AND resource_type != 'cache'",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for resource in resources {
            let resource_str = &to_string_without_metadata(&resource, false, None).unwrap();
            archive
                .write_to_archive(&resource_str, &format!("{}.resource.json", resource.path))
                .await?;
        }
    }

    if !skip_resources.unwrap_or(false) {
        let resource_types = sqlx::query_as!(
            ResourceType,
            "SELECT * FROM resource_type WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for resource_type in resource_types {
            let resource_str = &to_string_without_metadata(&resource_type, false, None).unwrap();
            archive
                .write_to_archive(
                    &resource_str,
                    &format!("{}.resource-type.json", resource_type.name),
                )
                .await?;
        }
    }

    {
        let flows = sqlx::query_as::<_, Flow>(
            "SELECT flow.workspace_id, flow.path, flow.summary, flow.description, flow.archived, flow.extra_perms, flow.draft_only, flow.dedicated_worker, flow.tag, flow.ws_error_handler_muted, flow.timeout, flow.visible_to_runner_only, flow_version.schema, flow_version.value, flow_version.created_at as edited_at, flow_version.created_by as edited_by
            FROM flow
            LEFT JOIN flow_version ON flow_version.id = flow.versions[array_upper(flow.versions, 1)]
            WHERE flow.workspace_id = $1 AND flow.archived = false",
        )
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        for flow in flows {
            let flow_str = &to_string_without_metadata(&flow, false, None).unwrap();
            archive
                .write_to_archive(&flow_str, &format!("{}.flow.json", flow.path))
                .await?;
        }
    }

    if !skip_variables.unwrap_or(false) {
        let variables =
            sqlx::query_as::<_, ExportableListableVariable>(if !skip_secrets.unwrap_or(false) {
                "SELECT * FROM variable WHERE workspace_id = $1 AND expires_at IS NULL"
            } else {
                "SELECT * FROM variable WHERE workspace_id = $1 AND is_secret = false AND expires_at IS NULL"
            })
            .bind(&w_id)
            .fetch_all(&mut *tx)
            .await?;

        let mc = build_crypt(&db, &w_id).await?;

        for mut var in variables {
            if plain_secret.or(plain_secrets).unwrap_or(false)
                && var.value.is_some()
                && var.is_secret
            {
                var.value = Some(decrypt(&mc, var.value.unwrap())?);
            }
            let var_str = &to_string_without_metadata(&var, false, None).unwrap();
            archive
                .write_to_archive(&var_str, &format!("{}.variable.json", var.path))
                .await?;
        }
    }

    {
        let apps = sqlx::query_as::<_, AppWithLastVersion>(
            "SELECT app.id, app.path, app.summary, app.versions, app.policy, app.custom_path,
            app.extra_perms, app_version.value, 
            app_version.created_at, app_version.created_by from app, app_version 
            WHERE app.workspace_id = $1 AND app_version.id = app.versions[array_upper(app.versions, 1)]",
        )
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        for app in apps {
            let app_str = &to_string_without_metadata(&app, false, None).unwrap();
            archive
                .write_to_archive(&app_str, &format!("{}.app.json", app.path))
                .await?;
        }
    }

    if include_schedules.unwrap_or(false) {
        let schedules = sqlx::query_as::<_, Schedule>(
            "SELECT * FROM schedule
            WHERE workspace_id = $1",
        )
        .bind(&w_id)
        .fetch_all(&mut *tx)
        .await?;

        for schedule in schedules {
            let app_str = &to_string_without_metadata(&schedule, false, None).unwrap();
            archive
                .write_to_archive(&app_str, &format!("{}.schedule.json", schedule.path))
                .await?;
        }
    }

    if include_users.unwrap_or(false) {
        let users = sqlx::query!(
            "SELECT * FROM usr
            WHERE workspace_id = $1",
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for user in users {
            let user = SimplifiedUser {
                username: user.username,
                role: if user.is_admin {
                    "admin".to_string()
                } else if user.operator {
                    "operator".to_string()
                } else {
                    "developer".to_string()
                },
                disabled: user.disabled,
                email: user.email,
            };
            let user_str = &to_string_without_metadata(
                &user,
                false,
                Some(vec!["is_admin", "operator", "email"]),
            )
            .unwrap();
            archive
                .write_to_archive(&user_str, &format!("users/{}.user.json", user.email))
                .await?;
        }
    }

    if include_groups.unwrap_or(false) {
        let groups = sqlx::query!(
            r#"SELECT g_.workspace_id, name, summary, extra_perms, array_agg(u2g.usr) filter (where u2g.usr is not null) as members 
            FROM usr u
            JOIN usr_to_group u2g ON u2g.usr = u.username AND u2g.workspace_id = u.workspace_id
            RIGHT JOIN group_ g_ ON g_.workspace_id = u.workspace_id AND g_.name = u2g.group_
            WHERE g_.workspace_id = $1 AND g_.name != 'all'
            GROUP BY g_.workspace_id, name, summary, extra_perms"#,
            &w_id
        )
        .fetch_all(&mut *tx)
        .await?;

        for group in groups {
            let extra_perms: HashMap<String, bool> = serde_json::from_value(group.extra_perms)
                .map_err(|e| {
                    Error::InternalErr(format!(
                        "Error parsing extra_perms for group {}: {}",
                        group.name, e
                    ))
                })?;
            tracing::info!("{:?}", extra_perms);
            let members = group.members.unwrap_or(vec![]);
            let admins: Vec<String> = extra_perms
                .iter()
                .filter_map(|(k, v)| {
                    // only consider extra_perms that concern actual members of the group
                    if members.contains(&k[2..].to_string()) && *v {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .sorted()
                .collect();
            let group = SimplifiedGroup {
                name: group.name,
                summary: group.summary,
                members: members
                    .iter()
                    .filter_map(|x| {
                        // remove members that are also admins as they are already in the admins list
                        let full_name = format!("u/{}", x);
                        if !admins.contains(&full_name) {
                            Some(full_name)
                        } else {
                            None
                        }
                    })
                    .collect(),
                admins,
            };

            let group_str = &to_string_without_metadata(&group, true, None).unwrap();
            archive
                .write_to_archive(&group_str, &format!("groups/{}.group.json", group.name))
                .await?;
        }
    }

    if include_settings.unwrap_or(false) {
        let settings = sqlx::query_as!(
            SimplifiedSettings,
            r#"SELECT
                -- slack_team_id, 
                -- slack_name, 
                -- slack_command_script, 
                -- CASE WHEN slack_email = 'missing@email.xyz' THEN NULL ELSE slack_email END AS slack_email,
                auto_invite_domain IS NOT NULL AS "auto_invite_enabled!",
                CASE WHEN auto_invite_operator IS TRUE THEN 'operator' ELSE 'developer' END AS "auto_invite_as!", 
                CASE WHEN auto_add IS TRUE THEN 'add' ELSE 'invite' END AS "auto_invite_mode!", 
                webhook, 
                deploy_to, 
                error_handler, 
                ai_resource, 
                code_completion_enabled, 
                error_handler_extra_args, 
                error_handler_muted_on_cancel, 
                large_file_storage, 
                git_sync,
                default_app,
                default_scripts,
                workspace.name
            FROM workspace_settings
            LEFT JOIN workspace ON workspace.id = workspace_settings.workspace_id
            WHERE workspace_id = $1"#,
            &w_id
        ).fetch_one(&mut *tx).await?;

        let settings_str = serde_json::to_value(settings)
            .map(|v| serde_json::to_string_pretty(&v).ok())
            .ok()
            .flatten()
            .ok_or_else(|| Error::InternalErr("Error serializing settings".to_string()))?;

        archive
            .write_to_archive(&settings_str, "settings.json")
            .await?;
    }

    if include_key.unwrap_or(false) {
        let key = sqlx::query_scalar!(
            "SELECT key FROM workspace_key WHERE workspace_id = $1",
            &w_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let key_json = serde_json::to_value(key)
            .map(|v| serde_json::to_string_pretty(&v).ok())
            .ok()
            .flatten()
            .ok_or_else(|| Error::InternalErr("Error serializing enryption key".to_string()))?;
        archive
            .write_to_archive(&key_json, "encryption_key.json")
            .await?;
    }

    archive.finish().await?;

    let file = tokio::fs::File::open(&file_path).await?;

    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    let headers = [
        (header::CONTENT_TYPE, "application/x-tar".to_string()),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{name}\""),
        ),
    ];
    Ok((headers, body))
}
