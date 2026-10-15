//! A preview's `on_behalf_of` runs request-supplied code as another account, so it must
//! change the job's identity for admins and be refused for everyone else.

use serde_json::json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_common::auth::create_jwt_token;
use windmill_common::db::Authed;
use windmill_test_utils::*;

/// `route` is under `/api/w/test-workspace/`.
async fn post(port: u16, token: &str, route: &str, body: serde_json::Value) -> (u16, String) {
    let resp = reqwest::Client::new()
        .post(format!("http://localhost:{port}/api/w/test-workspace/{route}"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    (resp.status().as_u16(), resp.text().await.unwrap())
}

async fn identity(db: &Pool<Postgres>, id: &str) -> (String, String) {
    sqlx::query_as("SELECT permissioned_as, permissioned_as_email FROM v2_job WHERE id = $1")
        .bind(Uuid::parse_str(id).unwrap())
        .fetch_one(db)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_preview_on_behalf_of(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let script = |obo: &str| {
        json!({
            "content": "export function main() { return 1; }",
            "language": "deno",
            "args": {},
            "on_behalf_of": obo,
        })
    };
    let flow = |obo: &str| json!({ "value": {"modules": []}, "args": {}, "on_behalf_of": obo });
    // The app editor's preview of an inline runnable.
    let app = |obo: &str| {
        json!({
            "args": {},
            "component": "c",
            "raw_code": { "language": "deno", "content": "export function main() { return 1; }" },
            "force_viewer_static_fields": {},
            "on_behalf_of": obo,
        })
    };
    let app_route = "apps_u/execute_component/u/test-user/app";

    for (route, body) in [
        ("jobs/run/preview", script("test2@windmill.dev")),
        ("jobs/run/preview_flow", flow("u/test-user-2")),
        (app_route, app("u/test-user-2")),
    ] {
        let (status, id) = post(port, "SECRET_TOKEN", route, body).await;
        assert!((200..300).contains(&status), "{route}: {status} {id}");
        assert_eq!(
            identity(&db, &id).await,
            (
                "u/test-user-2".to_string(),
                "test2@windmill.dev".to_string()
            ),
            "{route}"
        );
    }

    for (route, body) in [
        ("jobs/run/preview", script("test@windmill.dev")),
        ("jobs/run/preview_flow", flow("u/test-user")),
        (app_route, app("u/test-user")),
    ] {
        let (status, body) = post(port, "SECRET_TOKEN_2", route, body).await;
        assert_eq!(status, 403, "{route}: {body}");
    }

    let (status, id) = post(port, "SECRET_TOKEN_2", "jobs/run/preview", script("u/test-user-2")).await;
    assert_eq!(status, 201, "{id}");
    assert_eq!(identity(&db, &id).await.0, "u/test-user-2");

    // Reading data as the end user is the direct caller's own choice. A job token's end user is
    // whoever ran that job, so a job's code must not be able to ask for it.
    set_jwt_secret().await;
    let job_token = create_jwt_token(
        Authed {
            email: "test@windmill.dev".to_string(),
            username: "test-user".to_string(),
            is_admin: true,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: None,
            token_prefix: None,
        },
        "test-workspace",
        3600,
        Some(Uuid::new_v4()),
        Some("ephemeral-script".to_string()),
        None,
        None,
    )
    .await?;
    let viewer_data = json!({
        "content": "export function main() { return 1; }",
        "language": "deno",
        "args": {},
        "viewer_data_access": true,
    });
    for (token, expected) in [("SECRET_TOKEN", true), (job_token.as_str(), false)] {
        let (status, id) = post(port, token, "jobs/run/preview", viewer_data.clone()).await;
        assert_eq!(status, 201, "{id}");
        let stamped: bool =
            sqlx::query_scalar("SELECT viewer_data_access FROM job_perms WHERE job_id = $1")
                .bind(Uuid::parse_str(&id).unwrap())
                .fetch_one(&db)
                .await?;
        assert_eq!(stamped, expected, "job token: {}", !expected);
    }

    Ok(())
}
