use serial_test::serial;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const WS: &str = "test-workspace";

/// `run` is enforced by a layer on the jobs router, so this asserts through the router: a route
/// missing the layer, or a layer refusing too much, is what would regress. Serial because the
/// roles cache is process-global.
#[sqlx::test(migrations = "../migrations", fixtures("base", "permissions_test"))]
#[serial]
async fn test_withdrawn_run_refuses_runs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    windmill_common::roles::invalidate_all_roles_cache();
    let server = ApiServer::start(db.clone()).await?;
    let api = format!("http://localhost:{}/api/w/{WS}", server.addr.port());

    let run = async |token: &str| -> anyhow::Result<Vec<u16>> {
        let c = reqwest::Client::new();
        let auth = format!("Bearer {token}");
        let post = c
            .post(format!("{api}/jobs/run/p/u/operator/missing"))
            .header("Authorization", &auth)
            .json(&serde_json::json!({}))
            .send()
            .await?
            .status()
            .as_u16();
        // Runs over GET too.
        let get = c
            .get(format!("{api}/jobs/run_wait_result/p/u/operator/missing"))
            .header("Authorization", &auth)
            .send()
            .await?
            .status()
            .as_u16();
        Ok(vec![post, get])
    };

    // Unset, the operator runs: the route answers for the missing script, not the role.
    assert!(run("OPERATOR_TOKEN_1").await?.iter().all(|s| *s != 403));

    sqlx::query(
        "UPDATE workspace_settings SET role_overrides = '{\"operator\": {\"capabilities\": {\"run\": false}}}'
         WHERE workspace_id = $1",
    )
    .bind(WS)
    .execute(&db)
    .await?;
    windmill_common::roles::invalidate_all_roles_cache();

    // 403, not 401, which the frontend reads as a dead session.
    assert_eq!(run("OPERATOR_TOKEN_1").await?, vec![403, 403]);
    assert!(run("SECRET_TOKEN").await?.iter().all(|s| *s != 403));

    Ok(())
}
