use serde::Serialize;
use sqlx::PgExecutor;
use tokio::sync::RwLock;
use windmill_common::{
    apps::traverse_app_inline_scripts,
    cache,
    error::{Error, Result},
    flows::{FlowModuleValue, FlowValue},
    scripts::ScriptLang,
};

use std::collections::HashSet;

lazy_static::lazy_static! {
    pub static ref WMDEBUG_NO_DMAP_DISSOLVE: bool = std::env::var("WMDEBUG_NO_DMAP_DISSOLVE").is_ok();
}

type PathString = String;
type LockHash = i64;

#[derive(Serialize)]
pub struct DependencyMap {
    pub workspace_id: Option<String>,
    pub importer_path: Option<String>,
    pub importer_kind: Option<String>,
    pub imported_path: Option<String>,
    pub importer_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct DependencyDependent {
    pub importer_path: String,
    pub importer_kind: String,
    pub importer_node_ids: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ScopedDependencyMap {
    /// (importer_node_id, imported_path, imported_lockfile_hash)
    to_delete: HashSet<(String, PathString, Option<LockHash>)>,
    w_id: String,
    importer_path: String,
    importer_kind: String,
}

impl ScopedDependencyMap {
    /// Calls DB, however is assumed to be called once per dependency job
    /// AND is scoped to smaller subset of data
    /// So it is not too expensive
    pub async fn fetch_maybe_rearranged<'a>(
        w_id: &str,
        importer_path: &str,
        importer_kind: &str,
        parent_path: &Option<String>,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    ) -> Result<Self> {
        if parent_path
            .as_ref()
            .is_some_and(|x| !x.is_empty() && x != importer_path)
        {
            tracing::info!(
                workspace_id = %w_id,
                "detected top level rename from: {} to: {importer_path} on object of kind: {importer_kind}. reflecting in dependency_map.",
                parent_path.clone().unwrap_or_default(),
            );

            let dmap = sqlx::query_as::<_, (String, String, Option<i64>)>(
                "
UPDATE dependency_map
    SET importer_path = $1
    WHERE importer_path = $2
        AND importer_kind = $3::text::IMPORTER_KIND
        AND workspace_id = $4
RETURNING importer_node_id, imported_path, imported_lockfile_hash
        ",
            )
            .bind(importer_path)
            .bind(parent_path.clone().unwrap())
            .bind(importer_kind)
            .bind(w_id)
            .fetch_all(executor)
            .await?;
            Ok(Self {
                to_delete: HashSet::from_iter(dmap.into_iter()),
                w_id: w_id.to_owned(),
                importer_path: importer_path.to_owned(),
                importer_kind: importer_kind.to_owned(),
            })
        } else {
            Self::fetch(w_id, importer_path, importer_kind, executor).await
        }
    }

    /// Almost same as [[Self::fetch_maybe_rearranged]], however only reads values, thus a bit faster.
    pub async fn fetch<'a>(
        w_id: &str,
        importer_path: &str,
        importer_kind: &str,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    ) -> Result<Self> {
        let dmap = sqlx::query_as::<_, (String, String, Option<i64>)>(
            "
SELECT importer_node_id, imported_path, imported_lockfile_hash
    FROM dependency_map
    WHERE workspace_id = $1
        AND importer_path = $2
        AND importer_kind = $3::text::IMPORTER_KIND",
        )
        .bind(w_id)
        .bind(importer_path)
        .bind(importer_kind)
        .fetch_all(executor)
        .await?;

        Ok(Self {
            to_delete: HashSet::from_iter(dmap.into_iter()),
            w_id: w_id.to_owned(),
            importer_path: importer_path.to_owned(),
            importer_kind: importer_kind.to_owned(),
        })
    }

    /// Add missing entries to `dependency_map`
    /// Remove matching entries
    pub async fn patch<'c>(
        &mut self,
        referenced_paths: Option<Vec<String>>,
        node_id: String, // Flow Step/Node ID
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
        self.patch_tx_ref(referenced_paths, &node_id, &mut tx)
            .await?;
        Ok(tx)
    }

    pub async fn patch_tx_ref<'c>(
        &mut self,
        // NOTE: Referenced_paths should include all of the paths.
        referenced_paths: Option<Vec<String>>,
        node_id: &str, // Flow Step/Node ID
        tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> Result<()> {
        let Some(mut referenced_paths) = referenced_paths else {
            tracing::info!("relative imports are not found for: importer - {}, importer_node_id - {}, importer_kind - {}",
                &self.importer_path,
                &node_id,
                &self.importer_kind,
            );
            return Ok(());
        };

        // Fetch lock hashes for all referenced paths
        let lock_hashes: std::collections::HashMap<String, Option<i64>> = sqlx::query_as(
            "SELECT path, lockfile_hash FROM lock_hash WHERE workspace_id = $1 AND path = ANY($2)",
        )
        .bind(&self.w_id)
        .bind(&referenced_paths)
        .fetch_all(&mut **tx)
        .await?
        .into_iter()
        .collect();

        referenced_paths.retain(|path| {
            let hash = lock_hashes.get(path).cloned().flatten();
            !self
                .to_delete
                .remove(&(node_id.to_owned(), path.clone(), hash))
        });

        if !referenced_paths.is_empty() {
            tracing::info!("adding missing entries to dependency_map: importer_node_id - {}, importer_kind - {}, new_imported_paths - {:?}",
                &node_id,
                &self.importer_kind,
                &lock_hashes,
            );
        }

        for import in referenced_paths {
            let lock_hash = lock_hashes.get(&import).copied().flatten();
            sqlx::query!(
                "INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id, imported_lockfile_hash)
                VALUES ($1, $2, $3::text::IMPORTER_KIND, $4, $5, $6)
                ON CONFLICT (workspace_id, importer_node_id, importer_kind, importer_path, imported_path)
                DO UPDATE SET imported_lockfile_hash = EXCLUDED.imported_lockfile_hash",
                &self.w_id,
                &self.importer_path,
                &self.importer_kind,
                &import,
                node_id,
                lock_hash
            )
            .execute(&mut **tx)
            .await?;

            tracing::info!("added entry to dependency_map: {import:?}");
        }
        Ok(())
    }

    /// clean orphan entries from `dependency_map`
    pub async fn dissolve<'a>(
        self,
        mut tx: sqlx::Transaction<'a, sqlx::Postgres>,
    ) -> sqlx::Transaction<'a, sqlx::Postgres> {
        if *WMDEBUG_NO_DMAP_DISSOLVE {
            tracing::warn!(
                "WMDEBUG_NO_DMAP_DISSOLVE usually should not be used. Behavior might be unstable."
            );
            return tx;
        }

        tracing::info!("dissolving dependency_map: {:?}", &self);

        for (importer_node_id, imported_path, imported_lockfile_hash) in self.to_delete.into_iter()
        {
            tracing::info!("cleaning orphan entry from dependency_map: importer_kind - {}, imported_path - {}, importer_node_id - {}, imported_lockfile_hash - {:?}",
                &self.importer_kind,
                &imported_path,
                &importer_node_id,
                &imported_lockfile_hash,
            );

            if let Err(err) = sqlx::query!(
                "
        DELETE FROM dependency_map
        WHERE workspace_id = $1
            AND importer_path = $2
            AND importer_kind = $3::text::IMPORTER_KIND
            AND importer_node_id = $4
            AND imported_path = $5
            AND imported_lockfile_hash IS NOT DISTINCT FROM $6 -- we don't want to delete other entries with other lockfile hash.
            ",
                &self.w_id,
                &self.importer_path,
                &self.importer_kind,
                &importer_node_id,
                &imported_path,
                imported_lockfile_hash
            )
            .execute(&mut *tx)
            .await
            {
                tracing::error!(
                    "error while cleaning dependency_map for: importer_node_id - {}, imported_path - {}, importer_path - {}: {err}",
                    importer_node_id,
                    imported_path,
                    self.importer_path,
                );
            }
        }
        tx
    }

    /// Selectively clean dependency_map for object
    /// If `importer_node_id` is None will clear all nodes.
    pub async fn clear_map_for_item<'c>(
        item_path: &str,
        w_id: &str,
        importer_kind: &str,
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
        importer_node_id: &Option<String>,
    ) -> sqlx::Transaction<'c, sqlx::Postgres> {
        tracing::warn!(
            importer = item_path,
            kind = importer_kind,
            node_id = importer_node_id,
            workspace_id = w_id,
            "discovered orphan entry in `dependency_map`. It will be healed automatically, however please report this issue to Windmill Team. It is also advised to rebuild maps in workspace settings in troubleshooting.",
        );

        if let Err(err) = sqlx::query!(
            "DELETE FROM dependency_map
                 WHERE importer_path = $1 AND importer_kind = $3::text::IMPORTER_KIND
                 AND workspace_id = $2 AND ($4::text IS NULL OR importer_node_id = $4::text)",
            item_path,
            w_id,
            importer_kind,
            importer_node_id.clone(),
        )
        .execute(&mut *tx)
        .await
        {
            tracing::error!(
                workspace_id = w_id,
                "error while clearing discovered orphan: {err}"
            );
        }
        tx
    }

    pub async fn rebuild_map_unchecked(
        w_id: &str,
        db: &sqlx::Pool<sqlx::Postgres>,
    ) -> Result<String> {
        // Scripts
        tracing::info!(workspace_id = w_id, "Rebuilding dependency map for scripts");
        for r in sqlx::query!(
                r#"SELECT path, hash, language AS "language: ScriptLang" FROM script WHERE workspace_id = $1 AND archived = false AND deleted = false"#,
                w_id
            )
            .fetch_all(db)
            .await?
            {
                let (sd, smd) = cache::script::fetch(&db.clone().into(), r.hash.into()).await?;
                let mut dmap = ScopedDependencyMap::fetch(w_id, &r.path, "script", db).await?;
                let mut tx = db.begin().await?;

                tx = dmap
                .patch(
                    crate::extract_referenced_paths(&sd.code, &r.path, smd.language),
                    "".into(),
                    tx,
                )
                .await?;

                if !*WMDEBUG_NO_DMAP_DISSOLVE {
                    dmap.dissolve(tx).await.commit().await?;
                }
                tracing::info!(workspace_id = w_id, "Rebuilt for script {}", &r.path);
            }

        // Fetch only top level versions and paths
        tracing::info!(workspace_id = w_id, "Rebuilding dependency map for flows");
        for r in sqlx::query!("SELECT path, versions[array_upper(versions, 1)] as version FROM flow WHERE workspace_id = $1 AND archived = false", w_id).fetch_all(db).await? {
                if let Some(version) = r.version {
                    let flow_data = cache::flow::fetch_version(&db.clone().into(), version).await?;

                    let mut dmap = ScopedDependencyMap::fetch(w_id, &r.path, "flow", db).await?;

                    let mut tx = db.begin().await?;
                    let mut to_process = vec![];
                    let mut modules_to_check = flow_data.flow.modules.iter().collect::<Vec<_>>();
                    if let Some(failure_module) = flow_data.flow.failure_module.as_ref() {
                        modules_to_check.push(failure_module.as_ref());
                    }
                    if let Some(preprocessor_module) = flow_data.flow.preprocessor_module.as_ref() {
                        modules_to_check.push(preprocessor_module.as_ref());
                    }

                    FlowValue::traverse_leafs(modules_to_check, &mut |fmv, id| {
                        match fmv {
                            FlowModuleValue::RawScript { content, language, .. } => {
                                to_process.push((
                                    crate::extract_referenced_paths(
                                        content,
                                        &(r.path.clone() + "/flow"),
                                        Some(*language),
                                    ),
                                    id.clone(),
                                ));
                            }
                            FlowModuleValue::FlowScript { .. } => {
                                return Err(Error::internal_err("FlowScript is not supposed to be in flow.").into());
                            }
                            _ => {}
                        }
                        Ok(())
                    })?;

                    for (rp, id) in to_process {
                        tx = dmap.patch(rp, id, tx).await?;
                    }

                    if !*WMDEBUG_NO_DMAP_DISSOLVE {
                        dmap.dissolve(tx).await.commit().await?;
                    }

                    tracing::info!(workspace_id = w_id, "Rebuilt for flow {}", &r.path);
                } else {
                    tracing::error!(workspace_id = w_id, "version is never supposed to be none. skipping flow.");
                    return Err(Error::internal_err("version was none"));
                }
            }

        // Apps
        tracing::info!(workspace_id = w_id, "Rebuilding dependency map for apps");
        for r in sqlx::query!("SELECT path, versions[array_upper(versions, 1)] as version FROM app WHERE workspace_id = $1", w_id).fetch_all(db).await? {
                if let Some(version) = r.version {
                    let value = sqlx::query_scalar!(
                        "SELECT value FROM app_version WHERE id = $1 LIMIT 1",
                        version
                    )
                    .fetch_one(db)
                    .await?;

                    let mut dmap = ScopedDependencyMap::fetch(w_id, &r.path, "app", db).await?;
                    let mut tx = db.begin().await?;
                    let mut to_process = vec![];
                    traverse_app_inline_scripts(&value, None, &mut |ais, id| {
                        to_process.push((
                            crate::extract_referenced_paths(
                                &ais.content,
                                &(r.path.clone() + "/app"),
                                ais.language,
                            ),
                            id,
                        ));

                        Ok(())
                    })?;
                    for (ri, id) in to_process {
                        tx = dmap.patch(ri, id.unwrap_or_default(), tx).await?;
                    }
                    if !*WMDEBUG_NO_DMAP_DISSOLVE {
                        dmap.dissolve(tx).await.commit().await?;
                    }
                    tracing::info!(workspace_id = w_id, "Rebuilt for app {}", &r.path);
                } else {
                    tracing::error!(
                        workspace_id = w_id,
                        "version is never supposed to be none. skipping app."
                    );
                    return Err(Error::internal_err("version was none"));
                }
            }

        Ok("Success".into())
    }

    /// Run if you want to rebuild maps on specific workspace.
    /// Potentially takes much time
    pub async fn rebuild_map(w_id: &str, db: &sqlx::Pool<sqlx::Postgres>) -> Result<String> {
        lazy_static::lazy_static! {
            pub static ref LOCKED: RwLock<bool> = RwLock::new(false);
        }

        if *LOCKED.read().await {
            tracing::warn!(
                workspace_id = w_id,
                "Tried to rebuild dependency map. However rebuild is already in progress."
            );
            Ok("There is already one task pending, try again later.".into())
        } else {
            tracing::info!(workspace_id = w_id, "Rebuilding dependency map");

            if *WMDEBUG_NO_DMAP_DISSOLVE {
                tracing::warn!("WMDEBUG_NO_DMAP_DISSOLVE usually should not be used. Behavior might be unstable. Please contact Windmill Team for support.")
            }

            *LOCKED.write().await = true;
            let r = Self::rebuild_map_unchecked(w_id, db).await;
            *LOCKED.write().await = false;
            r
        }
    }

    /// Get dependents of any imported path - returns scripts/flows/apps that depend on it
    pub async fn get_dependents<'c>(
        imported_path: &str,
        workspace_id: &str,
        e: impl PgExecutor<'c>,
    ) -> Result<Vec<DependencyDependent>> {
        sqlx::query_as!(
            DependencyDependent,
            r#"
            SELECT
                importer_path,
                importer_kind::text as "importer_kind!",
                array_agg(importer_node_id) as importer_node_ids
            FROM dependency_map
            WHERE workspace_id = $1 AND imported_path = $2
            GROUP BY importer_path, importer_kind
            "#,
            workspace_id,
            imported_path
        )
        .fetch_all(e)
        .await
        .map_err(Error::from)
    }
}
