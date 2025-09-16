use std::collections::HashSet;
use windmill_common::error::Result;

// TODO: To be removed in future versions
lazy_static::lazy_static! {
    pub static ref WMDEBUG_NO_DMAP_DISSOLVE: bool = std::env::var("WMDEBUG_NO_DMAP_DISSOLVE").is_ok();
}
// TODO: Clean dependency_map table
// TODO: Completely messup the thing
// - Change referenced node.
// TODO: Resilience - What if this fails there?
// TODO: Scripts do some black magic to make raw reqs work
// TODO: Add more logs
// TODO: Check if works with CLI
// TODO: Make sure if used with CLI it is not calling db and is not modifying dm
// TODO: What if renamed the object but rearrange is not called?
// TODO: Check if using CLI also generates new versions of flows/scripts
// TODO: Same but if deploying imported script (CLI)
// TODO: CLI do not skip if action was to rename step or flow/app/script
/// self-heals even if dependency_map is deleted
#[derive(Debug)]
pub struct ScopedDependencyMap {
    dm: HashSet<(String, String)>,
    w_id: String,
    importer_path: String,
    importer_kind: String,
}

impl ScopedDependencyMap {
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

    /// Calls DB, however is assumed to be called once per dependency job
    /// AND is scoped to smaller subset of data
    /// So it is not too expensive
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
            return Ok(tx);
        };

        dbg!(&relative_imports);
        dbg!(node_id);
        dbg!(&self.importer_path);

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

    pub(crate) async fn clear_map_for_item<'c>(
        item_path: &str,
        w_id: &str,
        importer_kind: &str,
        mut tx: sqlx::Transaction<'c, sqlx::Postgres>,
        importer_node_id: &Option<String>,
    ) -> Result<sqlx::Transaction<'c, sqlx::Postgres>> {
        tracing::warn!("self-healed, but is the bug");
        sqlx::query!(
            "DELETE FROM dependency_map
                 WHERE importer_path = $1 AND importer_kind = $3::text::IMPORTER_KIND
                 AND workspace_id = $2 AND ($4::text IS NULL OR importer_node_id = $4::text)",
            item_path,
            w_id,
            importer_kind,
            importer_node_id.clone(),
        )
        .execute(&mut *tx)
        .await?;
        Ok(tx)
    }
}
