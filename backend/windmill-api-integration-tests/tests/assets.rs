use serde_json::{json, Value};
use sqlx::{Pool, Postgres};
use uuid::Uuid;
use windmill_test_utils::*;

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

fn bearer(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder.header("Authorization", format!("Bearer {token}"))
}

async fn insert_job(db: &Pool<Postgres>, parent: Option<Uuid>, tag: &str) -> anyhow::Result<Uuid> {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO v2_job (id, workspace_id, tag, created_by, permissioned_as, \
         permissioned_as_email, kind, parent_job, same_worker, visible_to_owner) \
         VALUES ($1, 'test-workspace', $3, 'test-user', 'u/test-user', \
         'test@windmill.dev', 'script', $2, false, true)",
    )
    .bind(id)
    .bind(parent)
    .bind(tag)
    .execute(db)
    .await?;
    Ok(id)
}

/// `access_type` is `None` for a detection that could not tell read from write
/// — how a resource passed in a job's arguments is recorded.
async fn insert_job_asset(
    db: &Pool<Postgres>,
    job: Uuid,
    path: &str,
    access_type: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind) \
         VALUES ('test-workspace', $1, 's3object', $2::text::asset_access_type, $3, 'job')",
    )
    .bind(path)
    .bind(access_type)
    .bind(job.to_string())
    .execute(db)
    .await?;
    Ok(())
}

/// A run reports what its whole job tree touched: runtime detection records
/// against the job that did the operation, which for a flow step or a
/// workflow-as-code task is never the job the user opened.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_run_assets_covers_child_jobs(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    let parent = insert_job(&db, None, "other").await?;
    let child = insert_job(&db, Some(parent), "other").await?;
    let unrelated = insert_job(&db, None, "other").await?;

    insert_job_asset(&db, parent, "/data/shared.json", Some("r")).await?;
    insert_job_asset(&db, child, "/data/shared.json", Some("w")).await?;
    insert_job_asset(&db, child, "/data/child_only.json", Some("w")).await?;
    insert_job_asset(&db, parent, "/data/from_args.json", None).await?;
    insert_job_asset(&db, child, "/data/from_args.json", Some("w")).await?;
    insert_job_asset(&db, unrelated, "/data/unrelated.json", Some("w")).await?;

    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{parent}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(body["truncated"], json!(false));
    assert_eq!(
        body["assets"],
        json!([
            { "path": "/data/child_only.json", "kind": "s3object", "access_type": "w" },
            // The parent recorded no access type for this one; that must not erase
            // the child's.
            { "path": "/data/from_args.json", "kind": "s3object", "access_type": "w" },
            { "path": "/data/shared.json", "kind": "s3object", "access_type": "rw" },
        ]),
        "parent should report its own and its child's assets, with access types merged"
    );

    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{child}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(
        body["assets"],
        json!([
            { "path": "/data/child_only.json", "kind": "s3object", "access_type": "w" },
            { "path": "/data/from_args.json", "kind": "s3object", "access_type": "w" },
            { "path": "/data/shared.json", "kind": "s3object", "access_type": "w" },
        ]),
        "a child should report only what it touched itself"
    );

    // `asset` has no RLS of its own, so the job read gate is the only thing
    // standing between another member and these paths.
    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{parent}")),
        "SECRET_TOKEN_2",
    )
    .send()
    .await?;
    assert_eq!(
        resp.status().as_u16(),
        403,
        "a member who cannot read the run must not read its assets"
    );

    // ...and a share link is what lets that same member in. The tree is walked
    // outside the caller's RLS precisely so this works, since the token grants
    // access their own permissions do not.
    let view_token: String = bearer(
        client().get(format!("{ws}/jobs/job_view_token/{parent}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?
    .text()
    .await?;
    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{parent}")),
        "SECRET_TOKEN_2",
    )
    .header("X-View-Token", view_token)
    .send()
    .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(
        body["assets"].as_array().map(|a| a.len()),
        Some(3),
        "a share-link viewer should see the whole tree's assets"
    );

    Ok(())
}

/// The read gate only checks the root job's tag, so the walk has to keep a
/// tag-scoped token out of descendants outside its scope — without hiding the
/// ones below them, which the token could have asked for directly.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_run_assets_scopes_descendants_by_tag(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    sqlx::query(
        "INSERT INTO token (token_hash, token_prefix, token, email, label, super_admin, scopes) \
         VALUES (encode(sha256('TAG_TOKEN'::bytea), 'hex'), 'TAG_TOK', 'TAG_TOKEN', \
         'test@windmill.dev', 'tag-only', true, ARRAY['if_jobs:filter_tags:other'])",
    )
    .execute(&db)
    .await?;

    let parent = insert_job(&db, None, "other").await?;
    let in_scope = insert_job(&db, Some(parent), "other").await?;
    let out_of_scope = insert_job(&db, Some(parent), "deno").await?;
    let below_out_of_scope = insert_job(&db, Some(out_of_scope), "other").await?;
    insert_job_asset(&db, in_scope, "/data/in_scope.json", Some("w")).await?;
    insert_job_asset(&db, out_of_scope, "/data/out_of_scope.json", Some("w")).await?;
    insert_job_asset(&db, below_out_of_scope, "/data/nested.json", Some("w")).await?;

    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{parent}")),
        "TAG_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(
        body["assets"],
        json!([
            { "path": "/data/in_scope.json", "kind": "s3object", "access_type": "w" },
            { "path": "/data/nested.json", "kind": "s3object", "access_type": "w" },
        ]),
        "a tag-scoped token must not read assets of descendants outside its tags, \
         but must still reach in-scope jobs below them"
    );

    Ok(())
}

/// A fan-out run can touch more assets than one response should carry, and the
/// cut must be reported rather than served as if it were the whole list.
#[sqlx::test(migrations = "../migrations", fixtures("base"))]
async fn test_list_run_assets_caps_the_list(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();
    let ws = format!("http://localhost:{port}/api/w/test-workspace");

    // Three child jobs touching the same 1200 assets: the cap counts assets, and
    // the retention allows ten job rows per asset, so a row-counted cap would cut
    // this at a third of the list.
    let parent = insert_job(&db, None, "other").await?;
    for _ in 0..3 {
        let child = insert_job(&db, Some(parent), "other").await?;
        sqlx::query(
            "INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind) \
             SELECT 'test-workspace', '/out/' || lpad(g::text, 6, '0') || '.json', 's3object', 'w', \
             $1, 'job' FROM generate_series(1, 1200) g",
        )
        .bind(child.to_string())
        .execute(&db)
        .await?;
    }

    let resp = bearer(
        client().get(format!("{ws}/jobs/run_assets/{parent}")),
        "SECRET_TOKEN",
    )
    .send()
    .await?;
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await?;
    assert_eq!(body["truncated"], json!(true));
    let assets = body["assets"].as_array().expect("assets array");
    assert_eq!(assets.len(), 1000);
    assert_eq!(
        assets[0],
        json!({ "path": "/out/000001.json", "kind": "s3object", "access_type": "w" }),
        "the cap keeps the head of the ordered list, with its access type merged"
    );
    // Three jobs touched each asset, so a cap counting rows rather than assets
    // would fill the list with repeats and stop around /out/000334.json.
    assert_eq!(
        assets[999]["path"], "/out/001000.json",
        "the cap counts assets, not asset rows"
    );
    Ok(())
}
