//! A token scoped to one folder's schedules or triggers may bind them only to runnables it could
//! run itself: what they name later runs as the owner, with no token scopes left to check.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

#[sqlx::test(fixtures("base"))]
async fn a_save_needs_run_scope_on_what_it_names(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    sqlx::raw_sql(
        r#"INSERT INTO folder(workspace_id, name, display_name, owners, extra_perms)
             VALUES ('test-workspace', 'a', 'a', '{}', '{}'), ('test-workspace', 'b', 'b', '{}', '{}');
           INSERT INTO token(token_hash, token_prefix, token, email, label, scopes) VALUES
             (encode(sha256('WRITE_ONLY'::bytea), 'hex'), 'WRITE_ONLY', 'WRITE_ONLY',
              'test@windmill.dev', 'w', '{schedules:write:f/a/*,http_triggers:write:f/a/*}'),
             (encode(sha256('WRITE_RUN'::bytea), 'hex'), 'WRITE_RUN_', 'WRITE_RUN',
              'test@windmill.dev', 'wr',
              '{schedules:write:f/a/*,http_triggers:write:f/a/*,jobs:run:scripts:f/b/*}');"#,
    )
    .execute(&db)
    .await?;
    let server = ApiServer::start(db.clone()).await?;
    let ws = format!(
        "http://localhost:{}/api/w/test-workspace",
        server.addr.port()
    );
    let client = reqwest::Client::new();
    let send = async |token: &str, route: &str, body: Value| {
        let resp = client
            .post(format!("{ws}/{route}"))
            .bearer_auth(token)
            .json(&body)
            .send()
            .await
            .unwrap();
        (resp.status().as_u16(), resp.text().await.unwrap())
    };

    // (route, body, the path the write-only token is refused on)
    #[allow(unused_mut)]
    let mut saves: Vec<(&str, Value, &str)> = vec![
        (
            "schedules/create",
            json!({"path": "f/a/sched", "schedule": "0 0 0 * * *", "timezone": "UTC",
                   "script_path": "f/b/x", "is_flow": false, "enabled": false}),
            "f/b/x",
        ),
        (
            "schedules/create",
            json!({"path": "f/a/sched_handler", "schedule": "0 0 0 * * *", "timezone": "UTC",
                   "script_path": "f/b/x", "is_flow": false, "enabled": false,
                   "on_failure": "script/f/b/y"}),
            "f/b/y",
        ),
    ];
    #[cfg(feature = "http_trigger")]
    saves.push((
        "http_triggers/create",
        json!({"path": "f/a/route", "script_path": "f/b/x", "is_flow": false,
               "mode": "disabled", "route_path": "a_route", "request_type": "async",
               "authentication_method": "none", "http_method": "post",
               "is_static_website": false, "workspaced_route": true, "wrap_body": false,
               "raw_string": false}),
        "f/b/x",
    ));

    for (route, body, refused) in saves {
        let (s, b) = send("WRITE_ONLY", route, body.clone()).await;
        assert_eq!(s, 403, "{route} {body}: {b}");
        assert!(b.contains(&format!("jobs:run:scripts:{refused}")), "{b}");
        let (s, b) = send("WRITE_RUN", route, body.clone()).await;
        assert!(s < 300, "{route} {body}: {b}");
    }

    // An edit sets the stored target's args, so it is held to the same scope.
    let edit = json!({"schedule": "0 0 1 * * *", "timezone": "UTC", "args": {"x": 1}});
    let (s, b) = send("WRITE_ONLY", "schedules/update/f/a/sched", edit.clone()).await;
    assert_eq!(s, 403, "{b}");
    let (s, b) = send("WRITE_RUN", "schedules/update/f/a/sched", edit).await;
    assert!(s < 300, "{b}");
    Ok(())
}
