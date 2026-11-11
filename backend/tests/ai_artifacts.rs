//! AI session artifacts on the server: items whose access list decides who reads them, which
//! only their owner writes, and which nobody else reads while they are in the owner's trash.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const OWNER: &str = "Bearer SECRET_TOKEN_2";
const VIEWER: &str = "Bearer SECRET_TOKEN_3";

fn fingerprint(name: &str, text: &str) -> String {
    hex::encode(
        Sha256::new()
            .chain_update(name)
            .chain_update([0u8])
            .chain_update(text)
            .finalize(),
    )
}

/// A version as ai_artifacts_cases.json names it: its text under the name "Plan", or
/// `[name, text]`.
fn version_hash(version: &Value) -> String {
    match version {
        Value::Array(pair) => fingerprint(pair[0].as_str().unwrap(), pair[1].as_str().unwrap()),
        text => fingerprint("Plan", text.as_str().unwrap()),
    }
}

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
    assert!(
        body.get("latest_hash").is_none(),
        "a viewer read the fingerprint of the owner's latest version"
    );
    let (_, body) = api
        .post(
            "ai/artifacts/write/a1",
            VIEWER,
            json!({ "base_hash": fingerprint("Plan", "draft"), "version": 2, "name": "Plan", "content": "vandalized" }),
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

/// The rules in ai_artifacts_cases.json, which the browser's in-memory server copy is tested
/// against too.
#[sqlx::test(fixtures("base"))]
async fn versions_follow_the_shared_cases(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = Api::new(&server);
    let cases: Value = serde_json::from_str(include_str!("ai_artifacts_cases.json"))?;
    for (n, case) in cases["cases"].as_array().unwrap().iter().enumerate() {
        let case_name = case["name"].as_str().unwrap();
        let id = format!("case{n}");
        for (s, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let (op, args) = step
                .as_object()
                .unwrap()
                .iter()
                .find(|(key, _)| *key != "expect")
                .unwrap();
            let name = args.get("name").and_then(Value::as_str).unwrap_or("Plan");
            let base = args.get("base").map(version_hash).unwrap_or_default();
            let (status, body) = match op.as_str() {
                "publish" => {
                    let body = json!({ "artifact_id": id, "name": name, "kind": "md", "version": args["version"], "content": args["content"] });
                    api.post("ai/artifacts/publish", OWNER, body).await?
                }
                "write" => {
                    let body = json!({ "base_hash": base, "version": args["version"], "name": name, "content": args["content"] });
                    api.post(&format!("ai/artifacts/write/{id}"), OWNER, body)
                        .await?
                }
                "show" => {
                    let body = json!({ "base_hash": base, "pinned": args["pinned"], "version": args["version"], "name": name, "content": args["content"] });
                    api.post(&format!("ai/artifacts/show/{id}"), OWNER, body)
                        .await?
                }
                "get" => api.get(&format!("ai/artifacts/get/{id}"), OWNER).await?,
                "trash" | "restore" => {
                    api.post(&format!("ai/artifacts/{op}/{id}"), OWNER, json!({}))
                        .await?
                }
                other => panic!("{case_name}: unknown step {other}"),
            };
            assert_eq!(status, 200, "{case_name}, step {s}: {body}");
            for (key, want) in step["expect"].as_object().into_iter().flatten() {
                let got = body.get(key).unwrap_or(&body["artifact"][key]);
                let want = match key.as_str() {
                    "latest_hash" => json!(version_hash(want)),
                    _ => want.clone(),
                };
                assert_eq!(got, &want, "{case_name}, step {s}, {key}: {body}");
            }
        }
    }
    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn only_a_shared_copy_is_held_to_the_name_limit(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = Api::new(&server);
    api.publish("draft").await?;

    let write = json!({ "base_hash": fingerprint("Plan", "draft"), "version": 2, "name": "n".repeat(300), "content": "x" });
    let (_, body) = api
        .post("ai/artifacts/write/never-shared", OWNER, write.clone())
        .await?;
    assert_eq!(body["outcome"], "not_shared", "{body}");
    let (status, body) = api.post("ai/artifacts/write/a1", OWNER, write).await?;
    assert_eq!(status, 400, "{body}");

    Ok(())
}

#[sqlx::test(fixtures("base"))]
async fn search_shows_the_matching_line_of_a_long_document(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let api = Api::new(&server);
    let line = format!("{}Needle{}", "a".repeat(5000), "b".repeat(5000));
    api.publish(&format!("# Plan\n{line}\nlast")).await?;

    let (status, listed) = api.get("ai/artifacts/list?search=needle", OWNER).await?;
    assert_eq!(status, 200, "{listed}");
    let shown = listed[0]["match_line"].as_str().unwrap_or_default();
    assert!(
        shown.starts_with('…') && shown.contains("aNeedleb") && shown.ends_with('…'),
        "{shown}"
    );

    Ok(())
}
