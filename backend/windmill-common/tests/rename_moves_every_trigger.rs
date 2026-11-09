//! A runnable rename must carry every trigger table along.
//!
//! The tables are discovered from the schema, not listed here, so a trigger kind added later
//! fails this test until `update_triggers_script_path` updates it too. A table left out keeps
//! its triggers on the old path, where nothing runs them and no runnable page lists them.

use sqlx::{Pool, Postgres};

/// Statement-level triggers fire even when no row matches, so each table only has to be
/// updated, not populated.
#[sqlx::test(migrations = "../migrations")]
async fn rename_updates_every_trigger_table(db: Pool<Postgres>) -> anyhow::Result<()> {
    // `schedule` also targets a runnable but is moved by the rename handlers themselves,
    // since a schedule at the runnable's own path is renamed with it.
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT c.table_name::text FROM information_schema.columns c
         JOIN information_schema.columns f
           ON f.table_schema = c.table_schema AND f.table_name = c.table_name AND f.column_name = 'is_flow'
         WHERE c.table_schema = 'public' AND c.column_name = 'script_path' AND c.table_name <> 'schedule'
         ORDER BY 1",
    )
    .fetch_all(&db)
    .await?;
    assert!(!tables.is_empty());

    let mut tx = db.begin().await?;
    sqlx::raw_sql(
        "CREATE TEMP TABLE updated_tables (name text);
         CREATE FUNCTION pg_temp.record_update() RETURNS trigger LANGUAGE plpgsql AS
           $$ BEGIN INSERT INTO updated_tables VALUES (TG_TABLE_NAME); RETURN NULL; END $$;",
    )
    .execute(&mut *tx)
    .await?;
    for t in &tables {
        sqlx::query(&format!(
            "CREATE TRIGGER record_update AFTER UPDATE ON {t} \
             FOR EACH STATEMENT EXECUTE FUNCTION pg_temp.record_update()"
        ))
        .execute(&mut *tx)
        .await?;
    }

    windmill_common::triggers::update_triggers_script_path(
        &mut tx,
        "u/test/new",
        "u/test/old",
        "test",
        false,
    )
    .await?;
    let updated: Vec<String> = sqlx::query_scalar("SELECT name FROM updated_tables")
        .fetch_all(&mut *tx)
        .await?;
    let missed: Vec<&String> = tables.iter().filter(|t| !updated.contains(t)).collect();
    assert!(
        missed.is_empty(),
        "a rename leaves these trigger tables on the old path: {missed:?}. \
         Add them to `update_triggers_script_path`."
    );

    Ok(())
}
