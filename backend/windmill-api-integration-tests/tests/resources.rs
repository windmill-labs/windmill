use serde_json::json;
use sqlx::{Pool, Postgres};
#[cfg(feature = "mcp")]
use uuid::Uuid;

use windmill_test_utils::*;

fn resource_url(port: u16, endpoint: &str, path: &str) -> String {
    format!("http://localhost:{port}/api/w/test-workspace/resources/{endpoint}/{path}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn authed(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    builder.header("Authorization", "Bearer SECRET_TOKEN")
}

async fn authed_get(port: u16, endpoint: &str, path: &str) -> reqwest::Response {
    authed(client().get(resource_url(port, endpoint, path)))
        .send()
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "resources_test"))]
async fn test_resource_endpoints(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let base = format!("http://localhost:{port}/api/w/test-workspace/resources");

    // --- exists ---
    let resp = authed_get(port, "exists", "u/test-user/simple_resource").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "exists", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // --- get ---
    let resp = authed_get(port, "get", "u/test-user/simple_resource").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["path"], "u/test-user/simple_resource");
    assert_eq!(body["resource_type"], "object");
    assert_eq!(body["description"], "Simple resource");
    assert_eq!(body["value"], json!({"host": "localhost", "port": 5432}));

    let resp = authed_get(port, "get", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get_value ---
    let resp = authed_get(port, "get_value", "u/test-user/simple_resource").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"host": "localhost", "port": 5432})
    );

    let resp = authed_get(port, "get_value", "u/test-user/null_resource").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        serde_json::Value::Null
    );

    let resp = authed_get(port, "get_value", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // --- get_value_interpolated ---
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/simple_resource",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"host": "localhost", "port": 5432})
    );

    // $var: interpolation
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/resource_with_var",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"host": "localhost", "password": "hunter2"})
    );

    // $res: interpolation
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/resource_with_res",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"host": "localhost", "credentials": {"user": "admin", "password": "secret123"}})
    );

    // mixed $var: and $res: refs
    let resp = authed_get(port, "get_value_interpolated", "u/test-user/resource_mixed").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"host": "localhost", "password": "hunter2", "credentials": {"user": "admin", "password": "secret123"}})
    );

    // chained $res: -> $var:
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/chained_resource",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"service": "myapi", "auth": {"key": "sk-abc123"}})
    );

    // null value
    let resp = authed_get(port, "get_value_interpolated", "u/test-user/null_resource").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        serde_json::Value::Null
    );

    // not found
    let resp = authed_get(port, "get_value_interpolated", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // array passthrough
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/resource_with_array",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"hosts": ["host1", "host2"], "port": 5432})
    );

    // scalar $var: ref
    let resp = authed_get(
        port,
        "get_value_interpolated",
        "u/test-user/scalar_var_resource",
    )
    .await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<serde_json::Value>().await?, json!("hunter2"));

    // --- list ---
    let resp = authed(client().get(format!("{base}/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(
        list.len() >= 10,
        "expected at least 10 resources from fixture, got {}",
        list.len()
    );
    assert!(list
        .iter()
        .any(|r| r["path"] == "u/test-user/simple_resource"));

    // list with resource_type filter
    let resp = authed(client().get(format!("{base}/list?resource_type=mcp_server")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().all(|r| r["resource_type"] == "mcp_server"));

    // --- list_search ---
    let resp = authed(client().get(format!("{base}/list_search")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(!list.is_empty());

    // --- list_names ---
    let resp = authed(client().get(format!("{base}/list_names/object")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    resp.json::<Vec<serde_json::Value>>().await?;

    // --- create ---
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/new_resource",
            "value": {"url": "https://example.com"},
            "description": "Created in test",
            "resource_type": "object"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    // verify it exists
    let resp = authed_get(port, "exists", "u/test-user/new_resource").await;
    assert_eq!(resp.json::<bool>().await?, true);

    // verify value
    let resp = authed_get(port, "get_value", "u/test-user/new_resource").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"url": "https://example.com"})
    );

    // create duplicate -> 400
    let resp = authed(client().post(format!("{base}/create")))
        .json(&json!({
            "path": "u/test-user/new_resource",
            "value": {},
            "resource_type": "object"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // create with update_if_exists -> 201
    let resp = authed(client().post(format!("{base}/create?update_if_exists=true")))
        .json(&json!({
            "path": "u/test-user/new_resource",
            "value": {"url": "https://updated.com"},
            "description": "Updated via upsert",
            "resource_type": "object"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = authed_get(port, "get_value", "u/test-user/new_resource").await;
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"url": "https://updated.com"})
    );

    // --- update (description) ---
    let resp = authed(client().post(resource_url(port, "update", "u/test-user/new_resource")))
        .json(&json!({"description": "Updated description"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "get", "u/test-user/new_resource").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["description"], "Updated description");

    // --- update_value ---
    let resp = authed(client().post(resource_url(
        port,
        "update_value",
        "u/test-user/new_resource",
    )))
    .json(&json!({"value": {"url": "https://final.com"}}))
    .send()
    .await
    .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "get_value", "u/test-user/new_resource").await;
    assert_eq!(
        resp.json::<serde_json::Value>().await?,
        json!({"url": "https://final.com"})
    );

    // --- delete ---
    let resp = authed(client().delete(resource_url(port, "delete", "u/test-user/new_resource")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "exists", "u/test-user/new_resource").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // delete nonexistent -> 404
    let resp = authed(client().delete(resource_url(port, "delete", "u/test-user/new_resource")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // --- file_resource_type_to_file_ext_map ---
    let resp = authed(client().get(format!("{base}/file_resource_type_to_file_ext_map")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let ext_map = resp.json::<serde_json::Value>().await?;
    // Verify the map includes fileset type info with is_fileset flag (no format_extension)
    let fileset_info = &ext_map["test_fileset"];
    assert_eq!(fileset_info["format_extension"], serde_json::Value::Null);
    assert_eq!(fileset_info["is_fileset"], true);
    // Verify non-fileset file type
    let file_info = &ext_map["test_file"];
    assert_eq!(file_info["format_extension"], "txt");
    assert_eq!(file_info["is_fileset"], false);

    // --- fileset resource value ---
    let resp = authed_get(port, "get_value", "u/test-user/fileset_resource").await;
    assert_eq!(resp.status(), 200);
    let fileset_val = resp.json::<serde_json::Value>().await?;
    assert_eq!(fileset_val["config.yaml"], "key: value");
    assert_eq!(fileset_val["data/input.json"], "{\"items\": []}");

    // --- resource types ---

    // type/exists
    let resp = authed_get(port, "type/exists", "test_db").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, true);

    let resp = authed_get(port, "type/exists", "nonexistent_type").await;
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.json::<bool>().await?, false);

    // type/get
    let resp = authed_get(port, "type/get", "test_db").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_db");
    assert_eq!(body["description"], "Test DB type");

    let resp = authed_get(port, "type/get", "nonexistent_type").await;
    assert_eq!(resp.status(), 404);

    // type/list
    let resp = authed(client().get(format!("{base}/type/list")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let list = resp.json::<Vec<serde_json::Value>>().await?;
    assert!(list.iter().any(|rt| rt["name"] == "test_db"));

    // type/listnames
    let resp = authed(client().get(format!("{base}/type/listnames")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let names = resp.json::<Vec<String>>().await?;
    assert!(names.contains(&"test_db".to_string()));

    // type/create
    let resp = authed(client().post(format!("{base}/type/create")))
        .json(&json!({
            "name": "new_test_type",
            "description": "A new type",
            "schema": {"type": "object"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = authed_get(port, "type/exists", "new_test_type").await;
    assert_eq!(resp.json::<bool>().await?, true);

    // type/create duplicate -> 400
    let resp = authed(client().post(format!("{base}/type/create")))
        .json(&json!({
            "name": "new_test_type",
            "description": "Duplicate",
            "schema": {"type": "object"}
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // type/update
    let resp = authed(client().post(resource_url(port, "type/update", "new_test_type")))
        .json(&json!({"description": "Updated type desc"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "type/get", "new_test_type").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["description"], "Updated type desc");

    // type/delete
    let resp = authed(client().delete(resource_url(port, "type/delete", "new_test_type")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "type/exists", "new_test_type").await;
    assert_eq!(resp.json::<bool>().await?, false);

    // --- fileset resource type CRUD ---

    // type/get for fileset type - verify is_fileset is returned
    let resp = authed_get(port, "type/get", "test_fileset").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["name"], "test_fileset");
    assert_eq!(body["is_fileset"], true);
    assert_eq!(body["format_extension"], serde_json::Value::Null);

    // type/get for non-fileset type - verify is_fileset is false
    let resp = authed_get(port, "type/get", "test_db").await;
    assert_eq!(resp.status(), 200);
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["is_fileset"], false);

    // type/create fileset type (no format_extension needed)
    let resp = authed(client().post(format!("{base}/type/create")))
        .json(&json!({
            "name": "new_fileset_type",
            "description": "A fileset type",
            "schema": {},
            "is_fileset": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = authed_get(port, "type/get", "new_fileset_type").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["is_fileset"], true);
    assert_eq!(body["format_extension"], serde_json::Value::Null);

    // type/update - set is_fileset on existing type
    let resp = authed(client().post(resource_url(port, "type/update", "new_fileset_type")))
        .json(&json!({"is_fileset": false}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = authed_get(port, "type/get", "new_fileset_type").await;
    let body = resp.json::<serde_json::Value>().await?;
    assert_eq!(body["is_fileset"], false);

    // cleanup
    let resp = authed(client().delete(resource_url(port, "type/delete", "new_fileset_type")))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    Ok(())
}

#[cfg(feature = "mcp")]
#[sqlx::test(migrations = "../migrations", fixtures("base", "resources_test"))]
async fn test_mcp_tools(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // unauthenticated -> 401
    let resp = client()
        .get(resource_url(port, "mcp_tools", "u/test-user/mcp_valid"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // not found -> 404
    let resp = authed_get(port, "mcp_tools", "u/test-user/nonexistent").await;
    assert_eq!(resp.status(), 404);

    // null value -> 400
    let resp = authed_get(port, "mcp_tools", "u/test-user/mcp_null").await;
    assert_eq!(resp.status(), 400);

    // invalid format -> 400 with parse error message
    let resp = authed_get(port, "mcp_tools", "u/test-user/mcp_invalid_format").await;
    assert_eq!(resp.status(), 400);
    let body = resp.text().await?;
    assert!(
        body.contains("Failed to parse MCP resource"),
        "expected parse error, got: {body}"
    );

    // valid MCP resource but unreachable server -> 500
    let resp = authed_get(port, "mcp_tools", "u/test-user/mcp_valid").await;
    assert_eq!(resp.status(), 500);
    let body = resp.text().await?;
    assert!(
        body.contains("Failed to connect to MCP server"),
        "expected connection error, got: {body}"
    );

    Ok(())
}

#[cfg(feature = "mcp")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_endpoint_tools_list(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let resp = authed(client().get(format!(
        "http://localhost:{port}/api/mcp/w/test-workspace/list_tools"
    )))
    .send()
    .await?;
    assert_eq!(resp.status(), 200);

    let tools: Vec<serde_json::Value> = resp.json().await?;

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    assert!(
        tool_names.contains(&"getJob"),
        "getJob not found in MCP endpoint tools: {tool_names:?}"
    );
    assert!(
        tool_names.contains(&"getJobLogs"),
        "getJobLogs not found in MCP endpoint tools: {tool_names:?}"
    );

    // Verify getJob has the expected path and method
    let get_job_tool = tools.iter().find(|t| t["name"] == "getJob").unwrap();
    assert_eq!(get_job_tool["path"], "/w/{workspace}/jobs_u/get/{id}");
    assert_eq!(get_job_tool["method"], "GET");

    // Verify getJobLogs has the expected path and method
    let get_job_logs_tool = tools.iter().find(|t| t["name"] == "getJobLogs").unwrap();
    assert_eq!(
        get_job_logs_tool["path"],
        "/w/{workspace}/jobs_u/get_logs/{id}"
    );
    assert_eq!(get_job_logs_tool["method"], "GET");

    Ok(())
}

#[cfg(feature = "mcp")]
async fn insert_completed_job_with_logs(db: &Pool<Postgres>) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, created_by, permissioned_as, kind, tag, args)
         VALUES ($1, 'test-workspace', 'test-user', 'u/test-user', 'script', 'deno', '{}'::jsonb)",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO v2_job_completed (id, workspace_id, duration_ms, result, status)
         VALUES ($1, 'test-workspace', 100, '42'::jsonb, 'success')",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO job_logs (job_id, workspace_id, logs, log_offset)
         VALUES ($1, 'test-workspace', 'hello world test log', 0)",
    )
    .bind(id)
    .execute(db)
    .await
    .unwrap();

    id
}

#[cfg(feature = "mcp")]
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_mcp_client_get_job_and_logs(db: Pool<Postgres>) -> anyhow::Result<()> {
    use rmcp::model::{
        CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation,
        InitializeRequestParams,
    };
    use rmcp::service::{RoleClient, RunningService};
    use rmcp::transport::streamable_http_client::{
        StreamableHttpClientTransport, StreamableHttpClientTransportConfig,
    };
    use rmcp::ServiceExt;

    initialize_tracing().await;
    set_jwt_secret().await;
    let server = ApiServer::start_mcp(db.clone()).await?;
    let port = server.addr.port();

    let job_id = insert_completed_job_with_logs(&db).await;

    // Create a token with MCP scopes
    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes)
         VALUES (encode(sha256('MCP_TOKEN'::bytea), 'hex'), 'MCP_TOK', 'MCP_TOKEN', 'test@windmill.dev', 'mcp token', true, ARRAY['mcp:all'])",
    )
    .execute(&db)
    .await?;

    // Connect as MCP client
    let config = StreamableHttpClientTransportConfig::with_uri(format!(
        "http://localhost:{port}/api/mcp/w/test-workspace/mcp"
    ))
    .auth_header("MCP_TOKEN");
    let transport = StreamableHttpClientTransport::from_config(config);

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test-client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            description: None,
            website_url: None,
            icons: None,
        },
        meta: None,
    };

    let client: RunningService<RoleClient, InitializeRequestParams> =
        client_info.serve(transport).await?;

    // --- Test getJob ---
    let result = client
        .call_tool(CallToolRequestParams {
            name: "getJob".into(),
            arguments: Some(serde_json::from_value(json!({ "id": job_id.to_string() }))?),
            task: None,
            meta: None,
        })
        .await?;

    let text = result
        .content
        .first()
        .and_then(|c| c.raw.as_text())
        .expect("getJob should return text content");
    let job: serde_json::Value = serde_json::from_str(&text.text)?;
    assert_eq!(job["id"], job_id.to_string());
    assert_eq!(job["workspace_id"], "test-workspace");
    assert_eq!(job["created_by"], "test-user");
    assert_eq!(job["job_kind"], "script");
    assert!(
        job["success"].as_bool().unwrap_or(false),
        "job should be successful: {job}"
    );

    // --- Test getJobLogs ---
    let result = client
        .call_tool(CallToolRequestParams {
            name: "getJobLogs".into(),
            arguments: Some(serde_json::from_value(json!({ "id": job_id.to_string() }))?),
            task: None,
            meta: None,
        })
        .await?;

    let text = result
        .content
        .first()
        .and_then(|c| c.raw.as_text())
        .expect("getJobLogs should return text content");
    // The logs endpoint returns text/plain, which gets wrapped as a JSON string by call_endpoint
    let logs: String = serde_json::from_str(&text.text)?;
    assert!(
        logs.contains("hello world test log"),
        "expected logs to contain test log, got: {logs}"
    );

    client.cancel().await?;
    Ok(())
}
