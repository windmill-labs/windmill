use sqlx::{Pool, Postgres};

/// Every definer-rights function must have no PUBLIC EXECUTE grant and pin a search_path ending
/// in pg_temp. Checked over the catalog so that a new function, or one redefined with CREATE OR
/// REPLACE (which resets both), fails here.
#[sqlx::test(migrations = "../migrations")]
async fn definer_functions_are_pinned_and_closed_to_public(db: Pool<Postgres>) {
    let exposed: Vec<String> = sqlx::query_scalar(
        "SELECT p.oid::regprocedure::text
         FROM pg_proc p
         JOIN pg_namespace n ON n.oid = p.pronamespace
         WHERE n.nspname = current_schema()
           AND p.prosecdef
           AND (has_function_privilege('public', p.oid, 'EXECUTE')
                OR NOT EXISTS (
                    SELECT 1 FROM unnest(p.proconfig) AS c
                    WHERE c LIKE 'search_path=%' AND c LIKE '%, pg_temp'))
         ORDER BY 1",
    )
    .fetch_all(&db)
    .await
    .unwrap();
    assert!(
        exposed.is_empty(),
        "executable by PUBLIC, or without a search_path ending in pg_temp: {exposed:?}"
    );

    let mut tx = db.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE windmill_user")
        .execute(&mut *tx)
        .await
        .unwrap();
    let labels: Option<Vec<String>> =
        sqlx::query_scalar("SELECT folder_labels('admins', 'f/none/item')")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(labels, None, "windmill_user keeps EXECUTE on folder_labels");
}
