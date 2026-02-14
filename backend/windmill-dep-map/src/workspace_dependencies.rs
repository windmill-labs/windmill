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
    /// then inserts the new dependencies. Triggers recomputation of dependent scripts
    /// and rebuilds the dependency map if this is the first unnamed dependency for the workspace.
    pub async fn create<'c>(
        self,
        metadata: (String, String, String),
        db: sqlx::Pool<sqlx::Postgres>,
    ) -> error::Result<i64> {
        windmill_common::workspace_dependencies::min_version_supports_v0_workspace_dependencies()
            .await?;

        let path = WorkspaceDependencies::to_path(&self.name, self.language)?;

        if self.name.is_none() {
            let setting_name = format!("workspace_dependencies_map_rebuilt:{}", self.workspace_id);
            let already_rebuilt =
                windmill_common::global_settings::load_value_from_global_settings(
                    &db,
                    &setting_name,
                )
                .await?
                .is_some();

            if !already_rebuilt {
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Rebuilding workspace dependencies map for first unnamed workspace dependencies"
                );
                ScopedDependencyMap::rebuild_map_unchecked(&self.workspace_id, &db).await?;

                windmill_common::global_settings::set_value_in_global_settings(
                    &db,
                    &setting_name,
                    serde_json::json!({}),
                )
                .await?;
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Marked workspace dependencies map as rebuilt"
                );
            } else {
                tracing::info!(
                    workspace_id = %self.workspace_id,
                    "Skipping workspace dependencies map rebuild - already rebuilt for this workspace"
                );
            }
        };

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

        trigger_dependents_to_recompute_dependencies_in_the_background(
            prev_description.is_none() && self.name.is_none(),
            self.workspace_id,
            self.language,
            metadata,
            path,
            db,
        )
        .await;

        Ok(new_id)
    }
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

pub type RawRequirements = WorkspaceDependencies;
pub type NewRawRequirements = NewWorkspaceDependencies;
