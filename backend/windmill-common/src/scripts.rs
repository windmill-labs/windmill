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
        _ => return None,
    }
}

pub async fn prefetch_cached_script(
    script: Script<ScriptRunnableSettingsHandle>,
    db: &DB,
) -> crate::error::Result<Script<ScriptRunnableSettingsInline>> {
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
        draft_only: script.draft_only,
        envs: script.envs,
        dedicated_worker: script.dedicated_worker,
        ws_error_handler_muted: script.ws_error_handler_muted,
        priority: script.priority,
        cache_ttl: script.cache_ttl,
        cache_ignore_s3_path: script.cache_ignore_s3_path,
        timeout: script.timeout,
        delete_after_use: script.delete_after_use,
        restart_unless_cancelled: script.restart_unless_cancelled,
        visible_to_runner_only: script.visible_to_runner_only,
        no_main_func: script.no_main_func,
        codebase: script.codebase,
        has_preprocessor: script.has_preprocessor,
        on_behalf_of_email: script.on_behalf_of_email,
        assets: script.assets,
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

    let hub_base_url = HUB_BASE_URL.read().await.clone();

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
    let cache_path = format!("{HUB_CACHE_DIR}/{version}");
    let script;
    if tokio::fs::metadata(&cache_path).await.is_err() {
        script = get_full_hub_script_by_path_inner(path, http_client, db).await?;
        if let Err(e) = crate::worker::write_file(
            HUB_CACHE_DIR,
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
    let hub_base_url = HUB_BASE_URL.read().await.clone();

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
        "SELECT
            workspace_id,
            hash,
            path,
            parent_hashes,
            summary,
            description,
            content,
            created_by,
            created_at,
            archived,
            schema,
            deleted,
            is_template,
            extra_perms,
            lock,
            lock_error_logs,
            language,
            kind,
            tag,
            draft_only,
            envs,
            concurrency_key,
            concurrent_limit,
            concurrency_time_window_s,
            debounce_key,
            debounce_delay_s,
            dedicated_worker,
            runnable_settings_handle,
            ws_error_handler_muted,
            priority,
            cache_ttl,
            cache_ignore_s3_path,
            timeout,
            delete_after_use,
            restart_unless_cancelled,
            visible_to_runner_only,
            no_main_func,
            codebase,
            has_preprocessor,
            on_behalf_of_email,
            assets
         FROM script WHERE path = $1 AND workspace_id = $2 AND archived = false ORDER BY created_at DESC LIMIT 1 FOR UPDATE",
    )
    .bind(path)
    .bind(w_id)
    .fetch_optional(e)
    .await
    .map_err(crate::error::Error::from)
}

pub struct ClonedScript {
    pub old_script: NewScript,
    pub new_hash: i64,
}
// TODO: What if dependency job fails, there is script with NULL in the lock
pub async fn clone_script<'c>(
    path: &str,
    w_id: &str,
    deployment_message: Option<String>,
    db: &DB,
) -> crate::error::Result<ClonedScript> {
    let mut tx = db.begin().await?;
    let s = if let Some(s) = fetch_script_for_update(path, w_id, &mut *tx).await? {
        s
    } else {
        return Err(crate::error::Error::NotFound(format!(
            "Non-archived script with path '{}' not found",
            path
        )));
    };

    let rs =
        runnable_settings::from_handle(s.runnable_settings.runnable_settings_handle, db).await?;
    let (debouncing_settings, concurrency_settings) =
        runnable_settings::prefetch_cached(&rs, db).await?;

    let ns = NewScript {
        path: s.path.clone(),
        parent_hash: Some(s.hash),
        summary: s.summary,
        description: s.description,
        content: s.content,
        schema: s.schema,
        is_template: s.is_template,
        lock: None,
        language: s.language,
        kind: Some(s.kind),
        tag: s.tag,
        draft_only: s.draft_only,
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
        restart_unless_cancelled: s.restart_unless_cancelled,
        deployment_message,
        visible_to_runner_only: s.visible_to_runner_only,
        no_main_func: s.no_main_func,
        codebase: s.codebase,
        has_preprocessor: s.has_preprocessor,
        on_behalf_of_email: s.on_behalf_of_email,
        preserve_on_behalf_of: None,
        assets: s.assets,
    };

    let new_hash = hash_script(&ns);

    tracing::debug!(
        "cloning script at path {} from '{}' to '{}'",
        s.path,
        *s.hash,
        new_hash
    );

    sqlx::query!("
    INSERT INTO script
    (workspace_id, hash, path, parent_hashes, summary, description, content, \
    created_by, schema, is_template, extra_perms, lock, language, kind, tag, \
    draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
    dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
    delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
    codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle)

    SELECT  workspace_id, $1, path, array_prepend($2::bigint, COALESCE(parent_hashes, '{}'::bigint[])), summary, description, \
            content, created_by, schema, is_template, extra_perms, NULL, language, kind, tag, \
            draft_only, envs, concurrent_limit, concurrency_time_window_s, cache_ttl, cache_ignore_s3_path, \
            dedicated_worker, ws_error_handler_muted, priority, restart_unless_cancelled, \
            delete_after_use, timeout, concurrency_key, visible_to_runner_only, no_main_func, \
            codebase, has_preprocessor, on_behalf_of_email, schema_validation, assets, debounce_key, debounce_delay_s, runnable_settings_handle

    FROM script WHERE hash = $2 AND workspace_id = $3;
            ", new_hash, s.hash.0, w_id).execute(&mut *tx).await?;

    // Archive base.
    sqlx::query!(
        "UPDATE script SET archived = true WHERE hash = $1 AND workspace_id = $2",
        *s.hash,
        w_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(ClonedScript { old_script: ns, new_hash })
}
