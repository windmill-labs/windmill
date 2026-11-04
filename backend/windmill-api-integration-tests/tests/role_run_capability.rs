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
        // Previews skip the router layer and check `run` in their handler.
        let preview = c
            .post(format!("{api}/jobs/run/preview"))
            .header("Authorization", &auth)
            .json(&serde_json::json!({"content": "select 1", "language": "postgresql", "args": {}}))
            .send()
            .await?
            .status()
            .as_u16();
        Ok(vec![post, get, preview])
    };
    let set_overrides = async |caps: &str| -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE workspace_settings SET role_overrides = $2::jsonb WHERE workspace_id = $1",
        )
        .bind(WS)
        .bind(format!("{{\"operator\": {{\"capabilities\": {caps}}}}}"))
        .execute(&db)
        .await?;
        windmill_common::roles::invalidate_all_roles_cache();
        Ok(())
    };

    // `code` granted so a preview refusal can only come from `run`. Unset, the operator runs.
    set_overrides(r#"{"code": true}"#).await?;
    assert!(run("OPERATOR_TOKEN_1").await?.iter().all(|s| *s != 403));

    set_overrides(r#"{"code": true, "run": false}"#).await?;
    // 403, not 401, which the frontend reads as a dead session.
    assert_eq!(run("OPERATOR_TOKEN_1").await?, vec![403, 403, 403]);
    assert!(run("SECRET_TOKEN").await?.iter().all(|s| *s != 403));

    Ok(())
}
