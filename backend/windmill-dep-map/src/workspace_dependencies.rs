use std::{collections::HashMap, sync::Arc};

use serde::{Deserialize, Serialize};
use windmill_common::{
    cache::workspace_dependencies::EXISTS_CACHE_TIMEOUT, error, scripts::ScriptLang,
    workspace_dependencies::WorkspaceDependencies,
};

use crate::{
    scoped_dependency_map::ScopedDependencyMap,
    trigger_dependents::trigger_dependents_to_recompute_dependencies,
};

#[derive(sqlx::FromRow, Clone, Serialize, Deserialize, Hash, Debug)]
pub struct NewWorkspaceDependencies {
    /// Trusted as-is by `create`: a caller deserializing this from a request must first check it
    /// against the workspace the caller is authorized in.
    pub workspace_id: String,
    pub language: ScriptLang,
    pub name: Option<String>,
    /// If None, will use description of previous version
    /// If there is no older versions, will set to default
    pub description: Option<String>,
    pub content: String,
}

impl NewWorkspaceDependencies {
    /// Creates a new workspace dependencies entry in the database.
    ///
    /// Archives any existing dependencies with the same name/language/workspace,
    /// then inserts the new dependencies. In the background, rebuilds the dependency map if this
    /// is the workspace's first unnamed dependency, then triggers recomputation of dependents.
    pub async fn create<'c>(
        self,
        metadata: (String, String, String),
        db: sqlx::Pool<sqlx::Postgres>,
    ) -> error::Result<i64> {
        windmill_common::workspace_dependencies::min_version_supports_v0_workspace_dependencies()
            .await?;

        let path = WorkspaceDependencies::to_path(&self.name, self.language)?;

        let mut tx = db.begin().await?;
        let prev_description = sqlx::query_scalar!(
            "
                UPDATE workspace_dependencies
                SET archived = true
                WHERE archived = false
                    AND name IS NOT DISTINCT FROM $1
                    AND workspace_id = $2
                    AND language = $3
                RETURNING description
            ",
            self.name,
            self.workspace_id,
            self.language as ScriptLang
        )
        .fetch_optional(&mut *tx)
        .await?;

        let content_hash = windmill_common::scripts::hash_script(&self.content);
        let new_id = sqlx::query_scalar!(
            "
            WITH ins AS (
                INSERT INTO workspace_dependencies(name, workspace_id, content, language, description)
                VALUES ($1, $2, $3, $4, $5)
                RETURNING id
            ), lock_ins AS (
                INSERT INTO lock_hash (workspace_id, path, lockfile_hash)
                VALUES ($2, $6, $7)
                ON CONFLICT (workspace_id, path) DO UPDATE SET lockfile_hash = $7
            )
            SELECT id FROM ins
            ",
            self.name.clone(),
            self.workspace_id,
            self.content,
            self.language as ScriptLang,
            self.description
                .or(prev_description.clone())
                .unwrap_or("Default Workspace Dependencies".to_owned()),
            path,
            content_hash
        )
        .fetch_one(&mut *tx)
        .await?;

        let wait_for_cache_timeout = prev_description.is_none() && self.name.is_none();

        // The first-time map rebuild walks every runnable in the workspace and can outlast a
        // client's timeout, so it runs after the response. The marker commits with the
        // dependencies, so a rebuild or redeploy lost to a restart or an error is run again
        // by `resume_pending_rebuilds` instead of leaving dependents on their old locks.
        let pending = if self.name.is_none()
            && !sqlx::query_scalar!(
                r#"SELECT EXISTS(SELECT 1 FROM global_settings WHERE name = $1) AS "exists!""#,
                map_rebuilt_setting(&self.workspace_id)
            )
            .fetch_one(&mut *tx)
            .await?
        {
            let (email, permissioned_as, created_by) = metadata.clone();
            let marker = pending_rebuild_setting(&self.workspace_id, &path);
            let value = serde_json::to_value(PendingRebuild {
                workspace_id: self.workspace_id.clone(),
                language: self.language,
                path: path.clone(),
                email,
                permissioned_as,
                created_by,
                wait_for_cache_timeout,
                dependencies_id: new_id,
            })
            .map_err(|e| error::Error::internal_err(e.to_string()))?;
            sqlx::query!(
                "INSERT INTO global_settings (name, value) VALUES ($1, $2)
                 ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
                marker,
                value
            )
            .execute(&mut *tx)
            .await?;
            Some(marker)
        } else {
            None
        };
        tx.commit().await?;

        match pending {
            Some(marker) => {
                tokio::spawn(run_pending_rebuild(marker, db));
            }
            None => {
                trigger_dependents_to_recompute_dependencies_in_the_background(
                    wait_for_cache_timeout,
                    self.workspace_id,
                    self.language,
                    metadata,
                    path,
                    db,
                )
                .await;
            }
        }

        Ok(new_id)
    }
}

const PENDING_REBUILD_PREFIX: &str = "workspace_dependencies_rebuild_pending:";

fn map_rebuilt_setting(workspace_id: &str) -> String {
    format!("workspace_dependencies_map_rebuilt:{}", workspace_id)
}

fn pending_rebuild_setting(workspace_id: &str, path: &str) -> String {
    format!("{PENDING_REBUILD_PREFIX}{workspace_id}:{path}")
}

/// What a first default dependencies push still owes its workspace: a map rebuild, then a
/// redeploy of the runnables that import `path`.
#[derive(Serialize, Deserialize)]
struct PendingRebuild {
    workspace_id: String,
    language: ScriptLang,
    path: String,
    email: String,
    permissioned_as: String,
    created_by: String,
    wait_for_cache_timeout: bool,
    /// Makes each push's marker value distinct, so a run only clears the push it handled.
    dependencies_id: i64,
}

/// Runs every pending rebuild that no server is running, such as one cut off by a restart or
/// one whose last attempt failed.
pub async fn resume_pending_rebuilds(db: &sqlx::Pool<sqlx::Postgres>) {
    match sqlx::query_scalar!(
        "SELECT name FROM global_settings WHERE left(name, length($1)) = $1",
        PENDING_REBUILD_PREFIX
    )
    .fetch_all(db)
    .await
    {
        Ok(markers) => {
            for marker in markers {
                tokio::spawn(run_pending_rebuild(marker, db.clone()));
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "failed to list pending workspace dependencies rebuilds")
        }
    }
}

async fn run_pending_rebuild(marker: String, db: sqlx::Pool<sqlx::Postgres>) {
    if let Err(e) = run_pending_rebuild_inner(&marker, &db).await {
        tracing::error!(
            marker = %marker,
            error = %e,
            "failed to rebuild the dependency map or redeploy the dependents of workspace dependencies; retrying in the background"
        );
    }
}

async fn run_pending_rebuild_inner(
    marker: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<()> {
    // Held for the whole run so that a monitor pass never runs a marker a live task owns.
    // Detached: closing the connection, also when the process dies, releases the lock.
    let mut lock_conn = db.acquire().await?.detach();
    let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(hashtextextended($1, 0))")
        .bind(marker)
        .fetch_one(&mut lock_conn)
        .await?;
    if !locked {
        return Ok(());
    }

    // A push for the same path while this runs replaces the marker's value; the delete then
    // misses, and the loop redeploys again for that push.
    loop {
        let Some(value) =
            sqlx::query_scalar!("SELECT value FROM global_settings WHERE name = $1", marker)
                .fetch_optional(db)
                .await?
        else {
            return Ok(());
        };
        let pending: PendingRebuild = serde_json::from_value(value.clone())
            .map_err(|e| error::Error::internal_err(format!("invalid pending rebuild: {e}")))?;

        rebuild_map_once(&pending.workspace_id, db).await?;
        recompute_dependents(
            pending.wait_for_cache_timeout,
            &pending.workspace_id,
            pending.language,
            (
                &pending.email,
                &pending.permissioned_as,
                &pending.created_by,
            ),
            &pending.path,
            db,
        )
        .await?;

        let cleared = sqlx::query!(
            "DELETE FROM global_settings WHERE name = $1 AND value = $2",
            marker,
            value
        )
        .execute(db)
        .await?
        .rows_affected();
        if cleared > 0 {
            return Ok(());
        }
    }
}

lazy_static::lazy_static! {
    static ref MAP_REBUILD_LOCKS: std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> =
        std::sync::Mutex::new(HashMap::new());
}

/// Dependents are read from the dependency map, so a workspace whose map predates workspace
/// dependencies gets it rebuilt once before its first default dependencies are acted on.
async fn rebuild_map_once(
    workspace_id: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<()> {
    // Pushes for several languages arrive back to back: the later ones wait for this rebuild
    // and then see the flag, rather than each walking the whole workspace again.
    let lock = MAP_REBUILD_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(workspace_id.to_string())
        .or_default()
        .clone();
    let guard = lock.lock().await;

    let setting_name = map_rebuilt_setting(workspace_id);
    let result: error::Result<()> = async {
        if windmill_common::global_settings::load_value_from_global_settings(db, &setting_name)
            .await?
            .is_some()
        {
            return Ok(());
        }
        tracing::info!(
            workspace_id = %workspace_id,
            "Rebuilding workspace dependencies map for first unnamed workspace dependencies"
        );
        ScopedDependencyMap::rebuild_map_unchecked(workspace_id, db).await?;
        windmill_common::global_settings::set_value_in_global_settings(
            db,
            &setting_name,
            serde_json::json!({}),
        )
        .await?;
        tracing::info!(
            workspace_id = %workspace_id,
            "Marked workspace dependencies map as rebuilt"
        );
        Ok(())
    }
    .await;

    drop(guard);
    let mut locks = MAP_REBUILD_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    // Only the map and this task hold it: no other push is waiting, so the entry can go.
    if Arc::strong_count(&lock) == 2 {
        locks.remove(workspace_id);
    }
    drop(locks);

    result
}

pub async fn trigger_dependents_to_recompute_dependencies_in_the_background(
    wait_for_cache_timeout: bool,
    workspace_id: String,
    language: ScriptLang,
    (email, permissioned_as, created_by): (String, String, String),
    path: String,
    db: sqlx::Pool<sqlx::Postgres>,
) {
    tokio::spawn(async move {
        if let Err(e) = recompute_dependents(
            wait_for_cache_timeout,
            &workspace_id,
            language,
            (&email, &permissioned_as, &created_by),
            &path,
            &db,
        )
        .await
        {
            tracing::error!(
                workspace_id = %workspace_id,
                path = %path,
                error = %e,
                "CRITICAL: failed to trigger dependents to recompute dependencies - dependent runnables are not being redeployed. Please contact the Windmill team"
            );
        }
    });
}

async fn recompute_dependents(
    wait_for_cache_timeout: bool,
    workspace_id: &str,
    language: ScriptLang,
    (email, permissioned_as, created_by): (&str, &str, &str),
    path: &str,
    db: &sqlx::Pool<sqlx::Postgres>,
) -> error::Result<()> {
    if wait_for_cache_timeout {
        tracing::debug!(
            workspace_id = %workspace_id,
            language = ?language,
            "waiting for cache timeout after creating first unnamed workspace dependencies"
        );
        tokio::time::sleep(*EXISTS_CACHE_TIMEOUT).await;
    }

    let dependents = ScopedDependencyMap::get_dependents(path, workspace_id, db).await?;
    trigger_dependents_to_recompute_dependencies(
        workspace_id,
        dependents,
        None,
        None,
        email,
        created_by,
        permissioned_as,
        db,
        vec![],
        false,
    )
    .await
}

pub type RawRequirements = WorkspaceDependencies;
pub type NewRawRequirements = NewWorkspaceDependencies;
