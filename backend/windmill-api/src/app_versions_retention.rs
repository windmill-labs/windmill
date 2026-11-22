//! Retention of raw app versions, per the `raw_app_versions_to_keep` instance setting.

use windmill_common::{
    error::{Error, Result},
    global_settings::{
        load_value_from_global_settings, parse_raw_app_versions_to_keep,
        RAW_APP_VERSIONS_TO_KEEP_SETTING,
    },
    DB,
};

/// Versions deleted per transaction, which holds the app's row and so its deploys.
const DELETE_BATCH: usize = 100;

/// Deletes the raw app versions past the `raw_app_versions_to_keep` newest of each app, with
/// their bundles, and returns how many went. Safe to run concurrently with itself and deploys.
///
/// Checks no caller and spans every workspace: for the server's monitor, never for a route.
pub async fn prune_raw_app_versions(db: &DB) -> Result<u64> {
    let setting = load_value_from_global_settings(db, RAW_APP_VERSIONS_TO_KEEP_SETTING).await?;
    let Some(keep) = parse_raw_app_versions_to_keep(setting.as_ref())
        .map_err(|e| Error::internal_err(format!("{RAW_APP_VERSIONS_TO_KEEP_SETTING}: {e}")))?
    else {
        return Ok(0);
    };

    // Newest is last in `app.versions`, as in the history, not latest `created_at`, which is
    // when the deploy started: a version was superseded no earlier than both its own and its
    // successor's. The hour after that is for the dependency jobs and page loads still using it.
    let apps = sqlx::query!(
        r#"WITH ranked AS (
            SELECT av.id, av.app_id,
                row_number() OVER newest_first AS rn,
                GREATEST(av.created_at, lag(av.created_at) OVER newest_first) AS superseded_at
            FROM app_version av
            LEFT JOIN (
                SELECT app.id AS app_id, v.id, v.ord
                FROM app, unnest(app.versions) WITH ORDINALITY AS v(id, ord)
            ) pos ON pos.app_id = av.app_id AND pos.id = av.id
            WHERE av.raw_app
            WINDOW newest_first AS (
                PARTITION BY av.app_id ORDER BY COALESCE(pos.ord, 0) DESC, av.id DESC
            )
        )
        SELECT a.id AS app_id, a.workspace_id, array_agg(r.id) AS "versions!"
        FROM ranked r
        JOIN app a ON a.id = r.app_id
        WHERE r.rn > $1
            AND r.superseded_at < now() - interval '1 hour'
        GROUP BY a.id"#,
        keep,
    )
    .fetch_all(db)
    .await?;

    let mut pruned = 0;
    for app in apps {
        for candidates in app.versions.chunks(DELETE_BATCH) {
            let versions = without_draft_bases(db, &app.workspace_id, candidates).await?;
            if versions.is_empty() {
                continue;
            }
            // Objects first: once the rows are gone nothing names them to a later pass.
            #[cfg(all(feature = "enterprise", feature = "parquet"))]
            delete_stored_bundles(db, &app.workspace_id, &versions).await?;
            pruned += delete_versions(db, &app.workspace_id, app.app_id, &versions).await?;
        }
    }
    Ok(pruned)
}

/// A draft's base stays, for the diff against it. Read per batch, as a first pass over a long
/// history runs for minutes. Best effort, so no lock against draft saves: an editor can save a
/// draft on a version deleted long before, and a rebase then shows the whole draft as the diff.
async fn without_draft_bases(db: &DB, w_id: &str, versions: &[i64]) -> Result<Vec<i64>> {
    Ok(sqlx::query_scalar!(
        r#"SELECT c.id AS "id!" FROM unnest($2::bigint[]) AS c(id)
        WHERE NOT EXISTS (
            SELECT 1 FROM draft d
            WHERE d.workspace_id = $1 AND d.typ IN ('app', 'raw_app') AND d.base = c.id::text
        )"#,
        w_id,
        versions,
    )
    .fetch_all(db)
    .await?)
}

/// `app_script` rows stay: versions share them, and past jobs show their code from them.
async fn delete_versions(db: &DB, w_id: &str, app_id: i64, versions: &[i64]) -> Result<u64> {
    let mut tx = db.begin().await?;
    // The app row before its versions, the order deploys and relocks lock them in. The array is
    // filtered inside the UPDATE so that a version appended while it waited is kept.
    sqlx::query!(
        "UPDATE app SET versions = ARRAY(
            SELECT v.id FROM unnest(versions) WITH ORDINALITY AS v(id, ord)
            WHERE v.id <> ALL($2) ORDER BY v.ord
        ) WHERE id = $1",
        app_id,
        versions,
    )
    .execute(&mut *tx)
    .await?;
    let deleted = sqlx::query!(
        "DELETE FROM app_version WHERE app_id = $1 AND id = ANY($2)",
        app_id,
        versions,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    sqlx::query!(
        "DELETE FROM app_bundles WHERE app_version_id = ANY($1)",
        versions,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "DELETE FROM deployment_metadata WHERE workspace_id = $1 AND app_version = ANY($2)",
        w_id,
        versions,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(deleted)
}

#[cfg(all(feature = "enterprise", feature = "parquet"))]
async fn delete_stored_bundles(db: &DB, w_id: &str, versions: &[i64]) -> Result<()> {
    use futures::StreamExt;
    use windmill_object_store::object_store_reexports::{ObjectStoreError, Path};

    let Some(store) = windmill_object_store::get_object_store().await else {
        if windmill_object_store::instance_object_store_configured(db).await? {
            return Err(Error::internal_err(
                "instance object storage is unavailable, keeping the versions to retry",
            ));
        }
        return Ok(());
    };
    let paths: Vec<_> = versions
        .iter()
        .flat_map(|id| {
            ["js", "css"].map(|file_type| {
                Ok(Path::from(crate::apps::app_bundle_object_path(
                    w_id, *id, file_type,
                )))
            })
        })
        .collect();
    let mut results = store.delete_stream(futures::stream::iter(paths).boxed());
    while let Some(result) = results.next().await {
        match result {
            // An app without styles has no css, and a bundle may be in `app_bundles` instead.
            Ok(_) | Err(ObjectStoreError::NotFound { .. }) => {}
            Err(e) => return Err(windmill_object_store::object_store_error_to_error(e)),
        }
    }
    Ok(())
}
