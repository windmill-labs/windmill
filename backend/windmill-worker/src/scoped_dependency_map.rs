use std::collections::HashSet;
use windmill_common::error::Result;

// TODO: To be removed in future versions
lazy_static::lazy_static! {
    pub static ref WMDEBUG_NO_DMAP_DISSOLVE: bool = std::env::var("WMDEBUG_NO_DMAP_DISSOLVE").is_ok();
}
#[derive(Debug)]
pub struct ScopedDependencyMap {
    dm: HashSet<(String, String)>,
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

            let dm = sqlx::query_as::<_, (String, String)>(
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
                dm: HashSet::from_iter(dm.into_iter()),
                w_id: w_id.to_owned(),
                importer_path: importer_path.to_owned(),
                importer_kind: importer_kind.to_owned(),
            })
        } else {
            Self::fetch(w_id, importer_path, importer_kind, executor).await
        }
    }

    /// Almost same as [[Self::fetch_maybe_rearranged]], however only reads values, thus a bit faster.
    pub(crate) async fn fetch<'a>(
        w_id: &str,
        importer_path: &str,
        importer_kind: &str,
        executor: impl sqlx::Executor<'a, Database = sqlx::Postgres>,
    ) -> Result<Self> {
        let dm = sqlx::query_as::<_, (String, String)>(
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
            dm: HashSet::from_iter(dm.into_iter()),
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
        node_id: &str, // Flow Step/Node ID
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
    ) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
        let Some(mut relative_imports) = relative_imports else {
            tracing::info!("relative imports are not found for: importer - {}, importer_node_id - {}, importer_kind - {}",
                &self.importer_path,
                &node_id,
                &self.importer_kind,
            );
            return Ok(tx);
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
        relative_imports.retain(|imported_path| {
            !self
                .dm
                // As dm is HashSet, removing is O(1) operation
                // thus making entire process very efficient
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
            .execute(&mut *tx)
            .await?;

            tracing::info!("added entry to dependency_map: {import:?}");
        }
        Ok(tx)
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
        for (importer_node_id, imported_path) in self.dm.into_iter() {
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
    pub(crate) async fn clear_map_for_item<'c>(
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
            "discovered orphan entry in `dependency_map`. It will be healed automatically, however please report this issue to Windmill Team.",
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
}
