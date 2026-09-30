//! Turning Windmill's own Postgres off as a data table substrate takes the data tables already on
//! it out of use, not only new ones: resolution is the one path every job, API call and trigger
//! reaches a data table through.

use sqlx::{Pool, Postgres};
use windmill_common::workspaces::get_datatable_resource_from_db_unchecked;

#[sqlx::test]
async fn test_instance_datatable_refused_once_instance_pg_disabled(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO workspace (id, name, owner) VALUES ('dtoff-ws', 'DTOFF', 'dtoff')")
        .execute(&db)
        .await?;
    sqlx::query(
        "INSERT INTO workspace_settings (workspace_id, datatable) VALUES ('dtoff-ws', \
         '{\"datatables\": {\"main\": {\"database\": {\"resource_type\": \"instance\", \
         \"resource_path\": \"dt_dtoff\"}}}}')",
    )
    .execute(&db)
    .await?;

    let resolved = get_datatable_resource_from_db_unchecked(&db, "dtoff-ws", "main").await?;
    assert_eq!(resolved["dbname"], "dt_dtoff");

    sqlx::query(
        "INSERT INTO global_settings (name, value) VALUES ('instance_pg_disabled', 'true')",
    )
    .execute(&db)
    .await?;

    let refused = get_datatable_resource_from_db_unchecked(&db, "dtoff-ws", "main")
        .await
        .expect_err("an instance data table must not resolve while Windmill's database is off");
    assert!(
        refused.to_string().contains("disabled on this instance"),
        "unexpected error: {refused}"
    );
    Ok(())
}
