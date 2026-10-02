use serde_json::json;
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

fn assert_2xx(status: u16, body: &str, endpoint: &str) {
    assert!(
        (200..300).contains(&status),
        "{endpoint} returned {status}: {body}",
    );
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_settings_2xx(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/settings");

    let resp = authed(client().get(format!("{base}/envs"))).send().await?;
    assert_2xx(resp.status().as_u16(), &resp.text().await?, "GET /envs");

    let resp = authed(client().get(format!("{base}/global/hub_base_url")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /global/hub_base_url",
    );

    let resp = authed(client().post(format!("{base}/global/test_key")))
        .json(&json!({"value": "test"}))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /global/test_key",
    );

    let resp = authed(client().get(format!("{base}/instance_config")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config",
    );

    let resp = authed(client().get(format!("{base}/instance_config/yaml")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /instance_config/yaml",
    );

    let resp = authed(client().get(format!("{base}/latest_key_renewal_attempt")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "GET /latest_key_renewal_attempt",
    );

    let resp = authed(client().post(format!("{base}/sync_cached_resource_types")))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /sync_cached_resource_types",
    );

    // --- Reachability only (need external services) ---

    let resp = authed(
        client()
            .post(format!("{base}/test_smtp"))
            .json(&json!({"to": "test@test.com", "subject": "test", "content": "test"})),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for POST /test_smtp"
    );

    let resp = authed(
        client()
            .post(format!("{base}/test_license_key"))
            .json(&json!({"license_key": "fake"})),
    )
    .send()
    .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert!(
        status != 404 || !body.is_empty(),
        "Router-level 404 for POST /test_license_key"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_alert_job_queue_waiting_in_global_settings(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let settings_base = format!("http://localhost:{port}/api/settings");
    let configs_base = format!("http://localhost:{port}/api/configs");

    let alert_payload = json!({
        "alerts": [
            {
                "name": "Test Alert",
                "tags_to_monitor": ["default", "gpu"],
                "jobs_num_threshold": 5,
                "alert_cooldown_seconds": 300,
                "alert_time_threshold_seconds": 60
            }
        ]
    });

    // Set alert_job_queue_waiting in global_settings
    let resp = authed(client().post(format!("{settings_base}/global/alert_job_queue_waiting")))
        .json(&json!({"value": alert_payload}))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /settings/global/alert_job_queue_waiting",
    );

    // Read it back
    let resp = authed(client().get(format!("{settings_base}/global/alert_job_queue_waiting")))
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body = resp.text().await?;
    assert_2xx(
        status,
        &body,
        "GET /settings/global/alert_job_queue_waiting",
    );
    let value: serde_json::Value = serde_json::from_str(&body)?;
    assert_eq!(value["alerts"][0]["name"], "Test Alert");
    assert_eq!(value["alerts"][0]["jobs_num_threshold"], 5);

    // Verify alert_job_queue_waiting does NOT appear in list_worker_groups
    let resp = authed(client().get(format!("{configs_base}/list_worker_groups")))
        .send()
        .await?;
    let body = resp.text().await?;
    let groups: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    let alert_in_groups = groups.iter().find(|g| {
        let name = g["name"].as_str().unwrap_or("");
        name.contains("alert")
    });
    assert!(
        alert_in_groups.is_none(),
        "alert_job_queue_waiting should not appear in worker groups"
    );

    // Delete by setting to null
    let resp = authed(client().post(format!("{settings_base}/global/alert_job_queue_waiting")))
        .json(&json!({"value": null}))
        .send()
        .await?;
    assert_2xx(
        resp.status().as_u16(),
        &resp.text().await?,
        "POST /settings/global/alert_job_queue_waiting (null)",
    );

    // Verify it reads back as null
    let resp = authed(client().get(format!("{settings_base}/global/alert_job_queue_waiting")))
        .send()
        .await?;
    let body = resp.text().await?;
    let value: serde_json::Value = serde_json::from_str(&body)?;
    assert!(
        value.is_null(),
        "alert_job_queue_waiting should be null after deletion"
    );

    Ok(())
}

#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_signing_secrets_never_returned(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let secrets = [
        ("jwt_secret", json!("planted-jwt-secret")),
        ("rsa_keys", json!({"private_key": "planted-rsa-key"})),
        (
            "custom_instance_replication_pwd",
            json!("planted-replication-pwd"),
        ),
        (
            "external_instance_pg_state",
            json!({"admin_pwd": "planted-pg-state"}),
        ),
    ];
    for (name, value) in &secrets {
        sqlx::query(
            "INSERT INTO global_settings (name, value) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(name)
        .bind(value)
        .execute(&db)
        .await?;
    }
    let server = ApiServer::start(db.clone()).await?;
    let base = format!("http://localhost:{}/api/settings", server.addr.port());

    let mut bodies = vec![];
    for path in ["instance_config", "instance_config/yaml"] {
        let resp = authed(client().get(format!("{base}/{path}")))
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        assert_2xx(status, &body, path);
        bodies.push((path.to_string(), body));
    }
    #[cfg(feature = "enterprise")]
    {
        let resp = authed(client().get(format!("{base}/list_global")))
            .send()
            .await?;
        let status = resp.status().as_u16();
        let body = resp.text().await?;
        assert_2xx(status, &body, "list_global");
        bodies.push(("list_global".to_string(), body));
    }
    for (name, _) in &secrets {
        let resp = authed(client().get(format!("{base}/global/{name}")))
            .send()
            .await?;
        let status = resp.status().as_u16();
        assert_eq!(status, 400, "GET /global/{name} returned {status}");
        bodies.push((format!("global/{name}"), resp.text().await?));
    }

    for (path, body) in bodies {
        assert!(
            !body.contains("planted-"),
            "{path} returned a server secret: {body}"
        );
    }
    Ok(())
}
