/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub use windmill_types::scripts::*;

use crate::{
    error::{to_anyhow, Error},
    runnable_settings::{self},
    utils::http_get_from_hub,
    workspace_dependencies::WorkspaceDependenciesAnnotatedRefs,
    DB, DEFAULT_HUB_BASE_URL, HUB_BASE_URL, PRIVATE_HUB_MIN_VERSION,
};

use crate::worker::HUB_CACHE_DIR;
use anyhow::Context;
use backon::ConstantBuilder;
use backon::{BackoffBuilder, Retryable};
use regex::Regex;

use crate::utils::StripPath;

pub fn extract_workspace_dependencies_annotated_refs(
    lang: &ScriptLang,
    code: &str,
    runnable_path: &str,
) -> Option<WorkspaceDependenciesAnnotatedRefs<String>> {
    use ScriptLang::*;
    lazy_static::lazy_static! {
        static ref RE_PYTHON: Regex = Regex::new(r"^\#\s?(\S+)\s*$").unwrap();
    }
    match lang {
        // TODO: Maybe use regex
        Bun | Bunnative | Nativets => WorkspaceDependenciesAnnotatedRefs::parse(
            "//",
            "package_json",
            code,
            None,
            runnable_path,
        ),
        Python3 => WorkspaceDependenciesAnnotatedRefs::parse(
            "#",
            "requirements",
            code,
            Some(&RE_PYTHON),
            runnable_path,
        ),
        Go => WorkspaceDependenciesAnnotatedRefs::parse("//", "go_mod", code, None, runnable_path),
        Php => WorkspaceDependenciesAnnotatedRefs::parse(
            "//",
            "composer_json",
            code,
            None,
            runnable_path,
        ),
        Powershell => WorkspaceDependenciesAnnotatedRefs::parse(
            "#",
            "modules_json",
            code,
            None,
            runnable_path,
        ),
        _ => return None,
    }
}

pub async fn prefetch_cached_script(
    script: Script<ScriptRunnableSettingsHandle>,
    db: &DB,
) -> crate::error::Result<Script<ScriptRunnableSettingsInline>> {
    prefetch_cached_script_inner(script, db, true).await
}

/// [`prefetch_cached_script`] without deriving the address, for callers that resolve it
/// themselves — the workspace export memoizes one lookup per distinct principal, and skips it
/// altogether for clients that only want the marker.
pub async fn prefetch_cached_script_without_email(
    script: Script<ScriptRunnableSettingsHandle>,
    db: &DB,
) -> crate::error::Result<Script<ScriptRunnableSettingsInline>> {
    prefetch_cached_script_inner(script, db, false).await
}

async fn prefetch_cached_script_inner(
    script: Script<ScriptRunnableSettingsHandle>,
    db: &DB,
    derive_email: bool,
) -> crate::error::Result<Script<ScriptRunnableSettingsInline>> {
    let derived_email = match script.on_behalf_of.as_deref().filter(|_| derive_email) {
        // Uncached: the client preserves this pair and sends it back, where the write path
        // validates it against an uncached lookup. A cached address would pair a live principal
        // with an address the account no longer holds, and the redeploy would be rejected.
        Some(permissioned_as) => Some(
            crate::users::get_email_from_permissioned_as_uncached(
                permissioned_as,
                &script.workspace_id,
                db,
            )
            .await?,
        ),
        None => None,
    };
    let rs = runnable_settings::from_handle(script.runnable_settings.runnable_settings_handle, db)
        .await?;
    let (debouncing_settings, concurrency_settings) =
        runnable_settings::prefetch_cached(&rs, db).await?;

    Ok(Script {
        workspace_id: script.workspace_id,
        hash: script.hash,
        path: script.path,
        parent_hashes: script.parent_hashes,
        summary: script.summary,
        description: script.description,
        content: script.content,
        created_by: script.created_by,
        created_at: script.created_at,
        archived: script.archived,
        schema: script.schema,
        deleted: script.deleted,
        is_template: script.is_template,
        extra_perms: script.extra_perms,
        lock: script.lock,
        lock_error_logs: script.lock_error_logs,
        language: script.language,
        kind: script.kind,
        tag: script.tag,
        envs: script.envs,
        dedicated_worker: script.dedicated_worker,
        ws_error_handler_muted: script.ws_error_handler_muted,
        priority: script.priority,
        cache_ttl: script.cache_ttl,
        cache_ignore_s3_path: script.cache_ignore_s3_path,
        timeout: script.timeout,
        delete_after_use: script.delete_after_use,
        delete_after_secs: script.delete_after_secs,
        restart_unless_cancelled: script.restart_unless_cancelled,
        visible_to_runner_only: script.visible_to_runner_only,
        auto_kind: script.auto_kind,
        codebase: script.codebase,
        has_preprocessor: script.has_preprocessor,
        // Derived, not stored: see the field's doc comment.
        on_behalf_of_email: derived_email,
        on_behalf_of: script.on_behalf_of,
        assets: script.assets,
        modules: script.modules,
        labels: script.labels,
        inherited_labels: script.inherited_labels,
        runnable_settings: ScriptRunnableSettingsInline {
            concurrency_settings: concurrency_settings.maybe_fallback(
                script.runnable_settings.concurrency_key,
                script.runnable_settings.concurrent_limit,
                script.runnable_settings.concurrency_time_window_s,
            ),
            debouncing_settings: debouncing_settings.maybe_fallback(
                script.runnable_settings.debounce_key,
                script.runnable_settings.debounce_delay_s,
            ),
        },
    })
}

pub async fn prefetch_cached_script_with_starred(
    sws: ScriptWithStarred<ScriptRunnableSettingsHandle>,
    db: &DB,
) -> crate::error::Result<ScriptWithStarred<ScriptRunnableSettingsInline>> {
    Ok(ScriptWithStarred {
        script: prefetch_cached_script(sws.script, db).await?,
        starred: sws.starred,
    })
}

pub async fn get_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: &DB,
) -> crate::error::Result<String> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let hub_base_url = (**HUB_BASE_URL.load()).clone();

    //
    let result = http_get_from_hub(
        http_client,
        &format!("{}/raw/{}.ts", hub_base_url, path),
        true,
        None,
        Some(db),
    )
    .await?
    .error_for_status()
    .map_err(to_anyhow)?
    .text()
    .await
    .map_err(to_anyhow);

    match result {
        Ok(result) => Ok(result),
        Err(e) => {
            if hub_base_url != DEFAULT_HUB_BASE_URL
                && path
                    .split("/")
                    .next()
                    .is_some_and(|x| x.parse::<i32>().is_ok_and(|x| x < PRIVATE_HUB_MIN_VERSION))
            {
                tracing::info!(
                    "Not found on private hub, fallback to default hub for {}",
                    path
                );
                let content = http_get_from_hub(
                    http_client,
                    &format!("{}/raw/{}.ts", DEFAULT_HUB_BASE_URL, path),
                    true,
                    None,
                    Some(db),
                )
                .await?
                .error_for_status()
                .map_err(to_anyhow)?
                .text()
                .await
                .map_err(to_anyhow)?;

                Ok(content)
            } else {
                Err(e)?
            }
        }
    }
}

pub async fn get_full_hub_script_by_path(
    path: StripPath,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubScript> {
    let path = path
        .to_path()
        .strip_prefix("hub/")
        .ok_or_else(|| Error::BadRequest("Impossible to remove prefix hex".to_string()))?;

    let mut path_iterator = path.split("/");
    let version = path_iterator
        .next()
        .ok_or_else(|| Error::internal_err(format!("expected hub path to have version number")))?;
    let cache_path = format!("{}/{version}", *HUB_CACHE_DIR);
    let script;
    if tokio::fs::metadata(&cache_path).await.is_err() {
        script = get_full_hub_script_by_path_inner(path, http_client, db).await?;
        if let Err(e) = crate::worker::write_file(
            &HUB_CACHE_DIR,
            &version,
            &serde_json::to_string(&script).map_err(to_anyhow)?,
        ) {
            tracing::error!("failed to write hub script {path} to cache: {e}");
        } else {
            tracing::info!("wrote hub script {path} to cache");
        }
    } else {
        let cache_content = tokio::fs::read_to_string(cache_path).await?;
        script = serde_json::from_str(&cache_content).unwrap();
        tracing::info!("read hub script {path} from cache");
    }
    Ok(script)
}

async fn get_full_hub_script_by_path_inner(
    path: &str,
    http_client: &reqwest::Client,
    db: Option<&DB>,
) -> crate::error::Result<HubScript> {
    let hub_base_url = (**HUB_BASE_URL.load()).clone();

    let response = (|| async {
        let response = http_get_from_hub(
            http_client,
            &format!("{}/raw2/{}", hub_base_url, path),
            true,
            None,
            db,
        )
        .await
        .and_then(|r| r.error_for_status().map_err(|e| to_anyhow(e).into()));

        match response {
            Ok(response) => Ok(response),
            Err(e) => {
                if hub_base_url != DEFAULT_HUB_BASE_URL
                    && path.split("/").next().is_some_and(|x| {
                        x.parse::<i32>().is_ok_and(|x| x < PRIVATE_HUB_MIN_VERSION)
                    })
                {
                    // TODO: should only fallback to default hub if status is 404 (hub returns 500 currently)
                    tracing::info!(
                        "Not found on private hub, fallback to default hub for {}",
                        path
                    );
                    http_get_from_hub(
                        http_client,
                        &format!("{}/raw2/{}", DEFAULT_HUB_BASE_URL, path),
                        true,
                        None,
                        db,
                    )
                    .await?
                    .error_for_status()
                    .map_err(|e| to_anyhow(e).into())
                } else {
                    Err(e)
                }
            }
        }
    })
    .retry(
        ConstantBuilder::default()
            .with_delay(std::time::Duration::from_secs(5))
            .with_max_times(2)
            .build(),
    )
    .notify(|err, dur| {
        tracing::warn!(
            "Could not get hub script at path {path}, retrying in {dur:#?}, err: {err:#?}"
        );
    })
    .sleep(tokio::time::sleep)
    .await?;

    let script = response
        .json::<HubScript>()
        .await
        .context(format!("Decoding hub response for script at path {path}"))?;

    Ok(script)
}

pub async fn fetch_script_for_update<'a>(
    path: &str,
    w_id: &str,
    e: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
) -> crate::error::Result<Option<Script<ScriptRunnableSettingsHandle>>> {
    sqlx::query_as::<_, Script<ScriptRunnableSettingsHandle>>(
        &format!(
            "SELECT {} FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
            SCRIPT_COLUMNS,
        ),
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(e)
    .await
    .map_err(crate::error::Error::from)
}

/// Deploys the outcome of a relative-import relock as a new version of `head`, the path's live
/// version that the caller holds `FOR UPDATE`, and archives `head`. A `lock` of `None` records
/// a failed generation: the version carries `lock_error_logs` instead and runs keep resolving
/// to the last version that has a lock. A `modules` of `None` keeps the head's module locks.
///
/// Writes whatever `head` names and checks nothing: callers are responsible for having
/// established access to its workspace and path, as a dependency job's push already has.
///
/// `created_at` is stamped when the insert runs, not at transaction start. The row lock on
/// `head` is what orders one relock after another, and with `now()` a transaction that began
/// first but locked second commits a live child older than its archived parent, which every
/// "latest version" read then mis-orders.
pub async fn deploy_relocked_version(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    head: Script<ScriptRunnableSettingsHandle>,
    deployment_message: Option<String>,
    lock: Option<&str>,
    modules: Option<&std::collections::HashMap<String, ScriptModule>>,
    lock_error_logs: Option<&str>,
) -> crate::error::Result<i64> {
    let s = head;
    let w_id = s.workspace_id.as_str();

    let rs =
        runnable_settings::from_handle(s.runnable_settings.runnable_settings_handle, &mut **tx)
            .await?;
    let (debouncing_settings, concurrency_settings) =
        runnable_settings::prefetch_cached_tx(&rs, &mut *tx).await?;

    // What the row stores is what the hash covers: the new module locks when there are any.
    let modules = modules.cloned().or(s.modules);
    let modules_json = modules.as_ref().map(serde_json::to_value).transpose()?;

    let ns = NewScript {
        path: s.path.clone(),
        parent_hash: Some(s.hash),
        summary: s.summary,
        description: s.description,
        content: s.content,
        schema: s.schema,
        is_template: s.is_template,
        lock: lock.map(str::to_string),
        language: s.language,
        kind: Some(s.kind),
        tag: s.tag,
        envs: s.envs,
        concurrency_settings: concurrency_settings.maybe_fallback(
            s.runnable_settings.concurrency_key,
            s.runnable_settings.concurrent_limit,
            s.runnable_settings.concurrency_time_window_s,
        ),
        debouncing_settings: debouncing_settings.maybe_fallback(
            s.runnable_settings.debounce_key,
            s.runnable_settings.debounce_delay_s,
        ),
        cache_ttl: s.cache_ttl,
        cache_ignore_s3_path: s.cache_ignore_s3_path,
        dedicated_worker: s.dedicated_worker,
        ws_error_handler_muted: s.ws_error_handler_muted,
        priority: s.priority,
        timeout: s.timeout,
        delete_after_use: s.delete_after_use,
        delete_after_secs: s.delete_after_secs,
        restart_unless_cancelled: s.restart_unless_cancelled,
        deployment_message,
        visible_to_runner_only: s.visible_to_runner_only,
        auto_kind: s.auto_kind,
        codebase: s.codebase,
        has_preprocessor: s.has_preprocessor,
        on_behalf_of_email: s.on_behalf_of_email,
        on_behalf_of: s.on_behalf_of,
        preserve_on_behalf_of: None,
        assets: s.assets,
        modules,
        auto_parent: None,
        labels: s.labels,
        skip_draft_deletion: None,
    };

    let new_hash = hash_script(&ns);

    tracing::debug!(
        "deploying relocked version of script at path {} from '{}' to '{}'",
        s.path,
        *s.hash,
        new_hash
    );

    sqlx::query!("
    INSERT INTO script
    (workspace_id, hash, path, parent_hashes, summary, description, content, \
    created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
    envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
    dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
    delete_after_use, delete_after_secs, timeout, concurrency_key, visible_to_runner_only, auto_kind, \
    codebase, has_preprocessor, on_behalf_of, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle, modules, labels, \
    lock_error_logs, created_at)

    SELECT  workspace_id, $1, path, array_prepend($2::bigint, COALESCE(parent_hashes, '{}'::bigint[])), summary, description, \
            content, created_by, schema, is_template, extra_perms, $4::text, language, kind, tag, \
            envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
            dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
            delete_after_use, delete_after_secs, timeout, concurrency_key, visible_to_runner_only, auto_kind, \
            codebase, has_preprocessor, on_behalf_of, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle, COALESCE($5::jsonb, modules), labels, \
            $6::text, clock_timestamp()

    FROM script WHERE hash = $2 AND workspace_id = $3;
            ", new_hash, s.hash.0, w_id, lock, modules_json, lock_error_logs).execute(&mut **tx).await?;

    // Archive base.
    sqlx::query!(
        "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2",
        *s.hash,
        w_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(new_hash)
}
