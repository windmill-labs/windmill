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
        tx.commit().await?;

        let wait_for_cache_timeout = prev_description.is_none() && self.name.is_none();
        let rebuild_map_first = self.name.is_none();
        let workspace_id = self.workspace_id;
        let language = self.language;
        // The first-time map rebuild walks every runnable in the workspace and can outlast a
        // client's timeout. Inside the request, a disconnect would cancel it before the flag is
        // set, so every retry would start it over.
        tokio::spawn(async move {
            if rebuild_map_first {
                rebuild_map_once(&workspace_id, &db).await;
            }
            recompute_dependents(
                wait_for_cache_timeout,
                workspace_id,
                language,
                metadata,
                path,
                db,
            )
            .await;
        });

        Ok(new_id)
    }
}

lazy_static::lazy_static! {
    static ref MAP_REBUILD_LOCKS: std::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> =
        std::sync::Mutex::new(HashMap::new());
}

/// Dependents are read from the dependency map, so a workspace whose map predates workspace
/// dependencies gets it rebuilt once before its first default dependencies are acted on.
async fn rebuild_map_once(workspace_id: &str, db: &sqlx::Pool<sqlx::Postgres>) {
    // Pushes for several languages arrive back to back: the later ones wait for this rebuild
    // and then see the flag, rather than each walking the whole workspace again.
    let lock = MAP_REBUILD_LOCKS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .entry(workspace_id.to_string())
        .or_default()
        .clone();
    let _guard = lock.lock().await;

    let setting_name = format!("workspace_dependencies_map_rebuilt:{}", workspace_id);
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
        let rebuilt = ScopedDependencyMap::rebuild_map_unchecked(workspace_id, db).await;
        // The rebuild runs after the new dependencies' hash is in `lock_hash` and records it as
        // already consumed, which would let every dependent skip its relock and keep the old
        // lock. Clearing the recorded hashes forces those relocks; at worst one is redundant.
        sqlx::query!(
            "UPDATE dependency_map SET imported_lockfile_hash = NULL
             WHERE workspace_id = $1 AND imported_path LIKE 'dependencies/%'
                AND imported_lockfile_hash IS NOT NULL",
            workspace_id
        )
        .execute(db)
        .await?;
        rebuilt?;
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
    if let Err(e) = result {
        tracing::error!(
            workspace_id = %workspace_id,
            error = %e,
            "failed to rebuild the dependency map for workspace dependencies; dependents are read from the existing map and the rebuild is retried on the next default dependencies push"
        );
    }
}

pub async fn trigger_dependents_to_recompute_dependencies_in_the_background(
    wait_for_cache_timeout: bool,
    workspace_id: String,
    language: ScriptLang,
    metadata: (String, String, String),
    path: String,
    db: sqlx::Pool<sqlx::Postgres>,
) {
    tokio::spawn(recompute_dependents(
        wait_for_cache_timeout,
        workspace_id,
        language,
        metadata,
        path,
        db,
    ));
}

async fn recompute_dependents(
    wait_for_cache_timeout: bool,
    workspace_id: String,
    language: ScriptLang,
    (email, permissioned_as, created_by): (String, String, String),
    path: String,
    db: sqlx::Pool<sqlx::Postgres>,
) {
    if wait_for_cache_timeout {
        tracing::debug!(
            workspace_id = %workspace_id,
            language = ?language,
            "waiting for cache timeout after creating first unnamed workspace dependencies"
        );
        tokio::time::sleep(*EXISTS_CACHE_TIMEOUT).await;
    }

    if let Err(e) = trigger_dependents_to_recompute_dependencies(
                &workspace_id,
                match ScopedDependencyMap::get_dependents(
                    path.as_str(),
                    &workspace_id,
                    &db,
                )
                .await
                {
                    Ok(importers) => importers,
                    Err(e) => {
                        tracing::error!(
                            workspace_id = %workspace_id,
                            path = %path,
                            error = %e,
                            "CRITICAL: failed to get dependents for workspace dependencies - dependent runnables are not being redeployed. Please contact the Windmill team"
                        );
                        return;
                    }
                },
                None,
                None,
                email.as_str(),
                created_by.as_str(),
                permissioned_as.as_str(),
                &db,
                vec![],
                false,
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
}

pub type RawRequirements = WorkspaceDependencies;
pub type NewRawRequirements = NewWorkspaceDependencies;
