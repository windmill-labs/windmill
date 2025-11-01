use serde::Serialize;
use tokio::sync::RwLock;
use windmill_common::{
    apps::traverse_app_inline_scripts,
    cache,
    error::{Error, Result},
    flows::{FlowModuleValue, FlowValue},
    scripts::ScriptLang,
};

use std::collections::HashSet;

use crate::worker_lockfiles::{
    extract_relative_imports, is_generated_from_raw_requirements,
    LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT,
};

// TODO: To be removed in future versions
lazy_static::lazy_static! {
    pub static ref WMDEBUG_NO_DMAP_DISSOLVE: bool = std::env::var("WMDEBUG_NO_DMAP_DISSOLVE").is_ok();
}

#[derive(Serialize)]
pub struct DependencyMap {
    pub workspace_id: Option<String>,
    pub importer_path: Option<String>,
    pub importer_kind: Option<String>,
    pub imported_path: Option<String>,
    pub importer_node_id: Option<String>,
}

#[derive(Debug)]
pub struct ScopedDependencyMap {
    dmap: HashSet<(String, String)>,
    w_id: String,
    importer_path: String,
    importer_kind: String,
}

impl ScopedDependencyMap {
    /// Calls DB, however is assumed to be called once per dependency job
    /// AND is scoped to smaller subset of data
    /// So it is not too expensive
    pub(crate) async fn fetch_maybe_rearranged<'a>(
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

            let dmap = sqlx::query_as::<_, (String, String)>(
                "
UPDATE dependency_map
    SET importer_path = $1
    WHERE importer_path = $2
        AND importer_kind = $3::text::IMPORTER_KIND
        AND workspace_id = $4
RETURNING importer_node_id, imported_path
        ",
            )
            .bind(importer_path)
            .bind(parent_path.clone().unwrap())
            .bind(importer_kind)
            .bind(w_id)
            .fetch_all(executor)
            .await?;
            Ok(Self {
                dmap: HashSet::from_iter(dmap.into_iter()),
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
        let dmap = sqlx::query_as::<_, (String, String)>(
            "
SELECT importer_node_id, imported_path
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
            dmap: HashSet::from_iter(dmap.into_iter()),
            w_id: w_id.to_owned(),
            importer_path: importer_path.to_owned(),
            importer_kind: importer_kind.to_owned(),
        })
    }

    /// Add missing entries to `dependency_map`
    /// Remove matching entries
    pub(crate) async fn patch<'c>(
        &mut self,
        relative_imports: Option<Vec<String>>,
        node_id: String, // Flow Step/Node ID
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
        self.patch_tx_ref(relative_imports, &node_id, &mut tx)
            .await?;
        Ok(tx)
    }

    pub(crate) async fn patch_tx_ref<'c>(
        &mut self,
        relative_imports: Option<Vec<String>>,
        node_id: &str, // Flow Step/Node ID
        tx: &mut sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> Result<()> {
        let Some(mut relative_imports) = relative_imports else {
            tracing::info!("relative imports are not found for: importer - {}, importer_node_id - {}, importer_kind - {}",
                &self.importer_path,
                &node_id,
                &self.importer_kind,
            );
            return Ok(());
        };

        // This does:
        // 1. remove all relative imports from relative_imports that ARE tracked in dependency_map
        // 2. remove corresponding trackers from dependency_map
        //
        // After this operation `relative_imports` variable has only untracked imports.
        // We will handle those in the next expression.
        //
        // After all `reduce`'s called ScopedDependencyMap has only extra/orphan imports
        // these are going to be clean up by calling [dissolve]
        // NOTE: `retain` iterates over vec and remove the ones whose closures returned false.
        relative_imports.retain(|imported_path| {
            !self
                .dmap
                // As dmap is HashSet, removing is O(1) operation
                // thus making entire process very efficient
                // NOTE: `remove` returns true if item was removed and false if wasn't.
                .remove(&(node_id.to_owned(), imported_path.to_owned()))
        });

        // As mentioned above, usually this will always be empty.
        if !relative_imports.is_empty() {
            tracing::info!("adding missing entries to dependency_map: importer_node_id - {}, importer_kind - {}, new_imported_paths - {:?}",
                &node_id,
                &self.importer_kind,
                &relative_imports,
            );
        }

        for import in relative_imports {
            sqlx::query!(
                "INSERT INTO dependency_map (workspace_id, importer_path, importer_kind, imported_path, importer_node_id)
                     VALUES ($1, $2, $3::text::IMPORTER_KIND, $4, $5) ON CONFLICT DO NOTHING",
                &self.w_id,
                &self.importer_path,
                &self.importer_kind,
                import,
                node_id
            )
            .execute(&mut **tx)
            .await?;

            tracing::info!("added entry to dependency_map: {import:?}");
        }
        Ok(())
    }

    /// clean orphan entries from `dependency_map`
    pub(crate) async fn dissolve<'a>(
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

        // We _could_ shove it into single query, but this query is rarely called AND let's keep it simple for redability.
        for (importer_node_id, imported_path) in self.dmap.into_iter() {
            tracing::info!("cleaning orphan entry from dependency_map: importer_kind - {}, imported_path - {}, importer_node_id - {}",
                &self.importer_kind,
                &imported_path,
                &importer_node_id,
            );

            // Dissolve MUST succeed. Error in dissolve MUST not block the execution.
            if let Err(err) = sqlx::query!(
                "
        DELETE FROM dependency_map
        WHERE workspace_id = $1
            AND importer_path = $2
            AND importer_kind = $3::text::IMPORTER_KIND
            AND importer_node_id = $4
            AND imported_path = $5
            ",
                &self.w_id,
                &self.importer_path,
                &self.importer_kind,
                &importer_node_id,
                &imported_path,
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

        // MUST succeed. Error MUST not block the execution.
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

    /// Run if you want to rebuild maps on specific workspace.
    /// Potentially takes much time
    pub async fn rebuild_map(w_id: &str, db: &sqlx::Pool<sqlx::Postgres>) -> Result<String> {
        async fn inner<'c>(w_id: &str, db: &sqlx::Pool<sqlx::Postgres>) -> Result<String> {
            // Scripts
            tracing::info!(workspace_id = w_id, "Rebuilding dependency map for scripts");
            for r in sqlx::query!(
                "SELECT path, hash FROM script WHERE workspace_id = $1 AND archived = false AND deleted = false",
                w_id
            )
            .fetch_all(db)
            .await?
            {
                let (sd, smd) = cache::script::fetch(&db.clone().into(), r.hash.into()).await?;
                let mut dmap = ScopedDependencyMap::fetch(w_id, &r.path, "script", db).await?;
                let mut tx = db.begin().await?;

                if (smd.language.is_some_and(|v| v == ScriptLang::Bun)
                    && sd
                        .lock
                        .as_ref()
                        .is_some_and(|v| v.contains("generatedFromPackageJson")))
                    || (smd.language.is_some_and(|v| v == ScriptLang::Python3)
                        && sd.lock.as_ref().is_some_and(|v| {
                            v.starts_with(LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT)
                        }))
                {
                    // if the lock file is generated from a package.json/requirements.txt, we need to clear the dependency map
                    // because we do not want to have dependencies be recomputed automatically. Empty relative imports passed
                    // to update_script_dependency_map will clear the dependency map.
                } else {
                    tx = dmap
                        .patch(
                            extract_relative_imports(&sd.code, &r.path, &smd.language),
                            "".into(),
                            tx,
                        )
                        .await?;
                }
                if !*WMDEBUG_NO_DMAP_DISSOLVE {
                    dmap.dissolve(tx).await.commit().await?;
                }
                tracing::info!(workspace_id = w_id, "Rebuilt for script {}", &r.path);
            }

            // Fetch only top level versions and paths
            // It is not fetching value
            tracing::info!(workspace_id = w_id, "Rebuilding dependency map for flows");
            for r in sqlx::query!("SELECT path, versions[array_upper(versions, 1)] as version FROM flow WHERE workspace_id = $1", w_id).fetch_all(db).await? {
                if let Some(version) = r.version {
                    // To reduce stress on db try to fetch from cache
                    // Since our flow versions are immutable it is safe to assume if we have cache for specific version/id it is up to date.
                    let flow_data = cache::flow::fetch_version(&db.clone().into(), version).await?;

                    // Create map for specific flow
                    let mut dmap = ScopedDependencyMap::fetch(w_id, &r.path, "flow", db).await?;

                    // Traverse retrieved flow modules
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
                            // Since we fetched from flow_version it is safe to assume all inline scripts are in form of RawScript.
                            FlowModuleValue::RawScript { content, language, lock ,.. } => {
                                if !is_generated_from_raw_requirements(Some(*language), lock) {
                                    to_process.push((
                                        extract_relative_imports(
                                            content,
                                            &(r.path.clone() + "/flow"),
                                            &Some(language.clone()),
                                        ),
                                        id.clone(),
                                    ));
                                }
                            }
                            // But just in case we will also handle other cases.
                            FlowModuleValue::FlowScript { .. } => {
                                // Abort will cancel transaction.
                                return Err(Error::internal_err("FlowScript is not supposed to be in flow."));
                            }
                            _ => {}
                        }
                        Ok(())
                    })?;

                    for (ri, id) in to_process {
                        tx = dmap.patch(ri, id, tx).await?;
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
                    // TODO: Use cache when implemented.
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
                            extract_relative_imports(
                                &ais.content,
                                &(r.path.clone() + "/app"),
                                &ais.language,
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
            let r = inner(w_id, db).await;
            *LOCKED.write().await = false;
            r
        }
    }
}
