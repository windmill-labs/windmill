//! AI session artifacts on the server: items whose access list decides who reads them, which
//! only their owner writes, and which nobody else reads while they are in the owner's trash.

use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const OWNER: &str = "Bearer SECRET_TOKEN_2";
const VIEWER: &str = "Bearer SECRET_TOKEN_3";

struct Api {
    client: reqwest::Client,
    base: String,
}

impl Api {
    fn new(server: &ApiServer) -> Self {
        Api {
            client: reqwest::Client::new(),
            base: format!(
                "http://localhost:{}/api/w/test-workspace",
                server.addr.port()
            ),
        }
    }

    async fn get(&self, route: &str, token: &str) -> anyhow::Result<(u16, Value)> {
        let resp = self
            .client
            .get(format!("{}/{route}", self.base))
            .header("Authorization", token)
            .send()
            .await?;
        Self::read(resp).await
    }

    async fn post(&self, route: &str, token: &str, body: Value) -> anyhow::Result<(u16, Value)> {
        let resp = self
            .client
            .post(format!("{}/{route}", self.base))
            .header("Authorization", token)
            .json(&body)
            .send()
            .await?;
        Self::read(resp).await
    }

    async fn read(resp: reqwest::Response) -> anyhow::Result<(u16, Value)> {
        let status = resp.status().as_u16();
        let text = resp.text().await?;
        Ok((
            status,
            serde_json::from_str(&text).unwrap_or(Value::String(text)),
        ))
    }

    async fn publish(&self, content: &str) -> anyhow::Result<Value> {
        let (status, body) = self
            .post(
                "ai/artifacts/publish",
                OWNER,
                json!({ "artifact_id": "a1", "name": "Plan", "kind": "md", "version": 1, "content": content }),
            )
            .await?;
        assert_eq!(status, 200, "{body}");
        Ok(body)
    }
}

#[sqlx::test(fixtures("base"))]
async fn an_artifact_reaches_its_access_list_until_it_is_trashed(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = Api::new(&server);

    let path = api.publish("draft").await?["path"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(api.get("ai/artifacts/get/a1", VIEWER).await?.0, 404);

    let viewer = json!({ "owner": "u/test-user-3", "write": false });
    let (status, _) = api
        .post(
            &format!("acls/add/ai_artifact/{path}"),
            VIEWER,
            viewer.clone(),
        )
        .await?;
    assert!(
        status >= 400,
        "someone else changed the owner's access list"
    );
    let (status, body) = api
        .post(&format!("acls/add/ai_artifact/{path}"), OWNER, viewer)
        .await?;
    assert_eq!(status, 200, "{body}");

    let (status, body) = api.get("ai/artifacts/get/a1", VIEWER).await?;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["content"], "draft");
    let (_, body) = api
        .post(
            "ai/artifacts/write/a1",
            VIEWER,
            json!({ "base_version": 1, "version": 2, "name": "Plan", "content": "vandalized" }),
        )
        .await?;
    assert_eq!(body["outcome"], "not_shared", "a viewer wrote the artifact");

    api.post("ai/artifacts/trash/a1", OWNER, json!({})).await?;
    assert_eq!(api.get("ai/artifacts/get/a1", VIEWER).await?.0, 404);
    let (_, listed) = api.get("ai/artifacts/list", VIEWER).await?;
    assert_eq!(
        listed,
        json!([]),
        "a trashed artifact was listed to a viewer"
    );
    let (status, body) = api.get("ai/artifacts/get/a1", OWNER).await?;
    assert_eq!(status, 200, "{body}");
    assert!(body["deleted_at"].is_string());

    // Restore brings back who it was shared with; sharing it again from the trash starts from
    // nobody, as for an artifact never shared.
    api.post("ai/artifacts/restore/a1", OWNER, json!({}))
        .await?;
    assert_eq!(api.get("ai/artifacts/get/a1", VIEWER).await?.0, 200);
    api.post("ai/artifacts/trash/a1", OWNER, json!({})).await?;
    let published = api.publish("draft").await?;
    assert_eq!(published["extra_perms"], json!({}));
    assert_eq!(api.get("ai/artifacts/get/a1", VIEWER).await?.0, 404);

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn a_write_based_on_an_older_version_is_refused(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = Api::new(&server);
    api.publish("draft").await?;

    let write = |content: &str| json!({ "base_version": 1, "version": 2, "name": "Plan", "content": content });
    let (_, body) = api
        .post("ai/artifacts/write/a1", OWNER, write("first"))
        .await?;
    assert_eq!(body["outcome"], "saved", "{body}");
    let (_, body) = api
        .post("ai/artifacts/write/a1", OWNER, write("second"))
        .await?;
    assert_eq!(body["outcome"], "stale", "{body}");
    assert_eq!(body["latest_version"], 2);

    let (_, body) = api.get("ai/artifacts/get/a1", OWNER).await?;
    assert_eq!(
        body["content"], "first",
        "a stale write overwrote a newer version"
    );

    Ok(())
}
