//! A dev workspace may itself be paired with one (a "dev of a dev"). The guards that keep that
//! shape well-formed are what these tests pin: a chain must stay acyclic, and no two dev workspaces
//! in it may carry the same environment label — they inherit the same git-sync repositories, so an
//! equal label means both deploy to one branch.

use serde_json::json;
use sqlx::{Pool, Postgres};

use windmill_test_utils::*;

const ADMIN_TOKEN: &str = "SECRET_TOKEN";

async fn attach(port: u16, prod: &str, body: serde_json::Value) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{prod}/workspaces/attach_dev_workspace"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .json(&body)
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

async fn detach(port: u16, prod: &str, dev: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{prod}/workspaces/detach_dev_workspace"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .json(&json!({ "dev_workspace_id": dev }))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_nested_dev_workspace_attach_guards(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // The family root is an ancestor of the dev workspace: reparenting it below would close a
    // parent<->child cycle and hang every hierarchy walk.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "test-workspace", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "cycle attach returned {status}: {body}"
    );
    assert!(body.contains("ancestor"), "unexpected error: {body}");

    // `tw-dev` is itself the 'dev' workspace of the root, so a dev nested under it cannot be one too.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "spare", "dev_workspace_label": "dev" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "label reuse returned {status}: {body}"
    );
    assert!(body.contains("tw-dev"), "unexpected error: {body}");

    // `standalone` brings its own 'dev'-labelled dev workspace into the chain, which collides with
    // `tw-dev` whatever label the candidate itself is given.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "standalone", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_client_error(),
        "subtree label reuse returned {status}: {body}"
    );
    assert!(body.contains("standalone-dev"), "unexpected error: {body}");

    // With a free label and nothing conflicting underneath, the nested pairing goes through.
    let (status, body) = attach(
        port,
        "tw-dev",
        json!({ "dev_workspace_id": "spare", "dev_workspace_label": "staging" }),
    )
    .await;
    assert!(
        status.is_success(),
        "nested attach returned {status}: {body}"
    );
    // Runtime-checked (not `query!`): a macro here would need its own `.sqlx` entry, which
    // `cargo sqlx prepare --workspace` does not produce for test targets.
    let (parent, is_dev, label): (Option<String>, bool, Option<String>) = sqlx::query_as(
        "SELECT parent_workspace_id, is_dev_workspace, dev_workspace_label FROM workspace WHERE id = 'spare'",
    )
    .fetch_one(&db)
    .await?;
    assert_eq!(parent.as_deref(), Some("tw-dev"));
    assert!(is_dev);
    assert_eq!(label.as_deref(), Some("staging"));

    // A chain runs as deep as there are distinct labels to give it, not two:
    // `test-workspace` -> `tw-dev` ('dev') -> `spare` ('staging') -> `e-cand` ('uat').
    let (status, body) = attach(
        port,
        "spare",
        json!({ "dev_workspace_id": "e-cand", "dev_workspace_label": "uat" }),
    )
    .await;
    assert!(
        status.is_success(),
        "third-label attach returned {status}: {body}"
    );

    Ok(())
}

async fn archive(port: u16, w_id: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .post(format!(
            "http://localhost:{port}/api/w/{w_id}/workspaces/archive"
        ))
        .header("Authorization", format!("Bearer {ADMIN_TOKEN}"))
        .send()
        .await
        .unwrap();
    let status = resp.status();
    (status, resp.text().await.unwrap())
}

#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_teardown_refuses_to_strand_a_nested_dev(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // A `wm-fork-` workspace keeps its parent when it stops being a dev workspace, so it returns to
    // being a throwaway fork — which hosts no pairing, leaving `redev-dev` attached with no way to
    // reach it. Detach and archive both clear the flag, so both have to refuse.
    let (status, body) = detach(port, "prod-b", "wm-fork-redev").await;
    assert!(
        status.is_client_error(),
        "stranding detach returned {status}: {body}"
    );
    assert!(body.contains("redev-dev"), "unexpected error: {body}");

    let (status, body) = archive(port, "wm-fork-redev").await;
    assert!(
        status.is_client_error(),
        "stranding archive returned {status}: {body}"
    );
    assert!(body.contains("redev-dev"), "unexpected error: {body}");

    // Archive soft-deletes whatever the id looks like, so a prefix-less dev workspace strands its
    // own dev too — even though detaching that same workspace is fine (it returns to standalone).
    let (status, body) = archive(port, "c-dev").await;
    assert!(
        status.is_client_error(),
        "prefix-less stranding archive returned {status}: {body}"
    );
    assert!(body.contains("c-dev-dev"), "unexpected error: {body}");
    let (status, body) = detach(port, "prod-c", "c-dev").await;
    assert!(
        status.is_success(),
        "prefix-less detach returned {status}: {body}"
    );

    // Bottom-up is the supported order.
    let (status, body) = detach(port, "wm-fork-redev", "redev-dev").await;
    assert!(status.is_success(), "leaf detach returned {status}: {body}");
    let (status, body) = detach(port, "prod-b", "wm-fork-redev").await;
    assert!(
        status.is_success(),
        "detach after cleanup returned {status}: {body}"
    );

    Ok(())
}

/// Giving a workspace a dev and clearing its own dev flag each decide on state the other mutates, so
/// checked outside a common lock both commit and leave `e-cand` under a throwaway fork. Fired
/// together: whichever lands second must see the first and be rejected.
///
/// Repeated, because how far each handler gets before the other's mutation lands is timing-dependent
/// — one pass caught an unlocked build only about a fifth of the time, and the runs are cheap.
#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_nested_attach_and_detach_cannot_both_commit(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for round in 0..12 {
        // Back to `prod-e -> wm-fork-edev ('dev')` with `e-cand` standalone, the state in which both
        // requests pass their own checks.
        sqlx::query(
            "UPDATE workspace SET parent_workspace_id = 'prod-e', is_dev_workspace = true,
                    dev_workspace_label = 'dev' WHERE id = 'wm-fork-edev'",
        )
        .execute(&db)
        .await?;
        sqlx::query(
            "UPDATE workspace SET parent_workspace_id = NULL, is_dev_workspace = false,
                    dev_workspace_label = NULL WHERE id = 'e-cand'",
        )
        .execute(&db)
        .await?;

        let (attached, detached) = tokio::join!(
            attach(
                port,
                "wm-fork-edev",
                json!({ "dev_workspace_id": "e-cand", "dev_workspace_label": "staging" }),
            ),
            detach(port, "prod-e", "wm-fork-edev"),
        );
        assert!(
            attached.0.is_success() != detached.0.is_success(),
            "round {round}: exactly one must win, got attach={} detach={}\n{}\n{}",
            attached.0,
            detached.0,
            attached.1,
            detached.1
        );

        // Whichever won, `wm-fork-edev` is never left a throwaway fork with a dev workspace beneath it.
        let (is_dev, cand_parent): (bool, Option<String>) = sqlx::query_as(
            "SELECT (SELECT is_dev_workspace FROM workspace WHERE id = 'wm-fork-edev'),
                    (SELECT parent_workspace_id FROM workspace WHERE id = 'e-cand')",
        )
        .fetch_one(&db)
        .await?;
        assert!(
            is_dev || cand_parent.is_none(),
            "round {round}: stranded — wm-fork-edev is_dev={is_dev}, e-cand parent={cand_parent:?}"
        );
    }
    Ok(())
}

/// Two adjacent attaches — `f-mid` under `prod-f` and `f-leaf` under `f-mid` — each see a chain that
/// does not yet contain the other's dev workspace, so both pass their label check. Committing both
/// puts two `dev` workspaces in one chain, deploying to the same branch. Repeated for the same
/// reason as the detach race above.
#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_adjacent_attaches_cannot_both_claim_a_label(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for round in 0..12 {
        sqlx::query(
            "UPDATE workspace SET parent_workspace_id = NULL, is_dev_workspace = false,
                    dev_workspace_label = NULL WHERE id IN ('f-mid', 'f-leaf')",
        )
        .execute(&db)
        .await?;

        let (upper, lower) = tokio::join!(
            attach(
                port,
                "prod-f",
                json!({ "dev_workspace_id": "f-mid", "dev_workspace_label": "dev" }),
            ),
            attach(
                port,
                "f-mid",
                json!({ "dev_workspace_id": "f-leaf", "dev_workspace_label": "dev" }),
            ),
        );
        assert!(
            upper.0.is_success() != lower.0.is_success(),
            "round {round}: exactly one must win, got upper={} lower={}\n{}\n{}",
            upper.0,
            lower.0,
            upper.1,
            lower.1
        );

        // Never both: that is the chain prod-f -> f-mid('dev') -> f-leaf('dev').
        let chained: bool = sqlx::query_scalar(
            "SELECT EXISTS(
                 SELECT 1 FROM workspace mid
                 JOIN workspace leaf ON leaf.parent_workspace_id = mid.id
                 WHERE mid.id = 'f-mid' AND leaf.id = 'f-leaf'
                   AND mid.is_dev_workspace AND leaf.is_dev_workspace
                   AND mid.parent_workspace_id = 'prod-f'
             )",
        )
        .fetch_one(&db)
        .await?;
        assert!(!chained, "round {round}: both attaches committed");
    }
    Ok(())
}

/// Attaching `g-mid` under `prod-g` and attaching `g-leaf` under `g-sub` touch no workspace in
/// common — `g-sub` already sits under `g-mid`, so the two operations are two hops apart. Each sees
/// a two-workspace chain with a free label; together they make a four-deep one that repeats
/// `staging`. Locking the endpoints alone leaves them free to both commit, which is why the pairing
/// lock covers every workspace its checks read.
#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_attaches_two_hops_apart_cannot_both_claim_a_label(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    for round in 0..12 {
        sqlx::query(
            "UPDATE workspace SET parent_workspace_id = NULL, is_dev_workspace = false,
                    dev_workspace_label = NULL WHERE id IN ('g-mid', 'g-leaf')",
        )
        .execute(&db)
        .await?;

        let (upper, lower) = tokio::join!(
            attach(
                port,
                "prod-g",
                json!({ "dev_workspace_id": "g-mid", "dev_workspace_label": "staging" }),
            ),
            attach(
                port,
                "g-sub",
                json!({ "dev_workspace_id": "g-leaf", "dev_workspace_label": "staging" }),
            ),
        );
        assert!(
            upper.0.is_success() != lower.0.is_success(),
            "round {round}: exactly one must win, got upper={} lower={}\n{}\n{}",
            upper.0,
            lower.0,
            upper.1,
            lower.1
        );

        // No chain may carry one label twice. Walk every dev workspace up to its root and count.
        let duplicated: Option<String> = sqlx::query_scalar(
            "WITH RECURSIVE chain AS (
                 SELECT id AS leaf, id, parent_workspace_id, is_dev_workspace,
                        COALESCE(dev_workspace_label, 'dev') AS label, 0 AS depth
                 FROM workspace WHERE is_dev_workspace AND NOT deleted
                 UNION ALL
                 SELECT c.leaf, w.id, w.parent_workspace_id, w.is_dev_workspace,
                        COALESCE(w.dev_workspace_label, 'dev'), c.depth + 1
                 FROM workspace w JOIN chain c ON w.id = c.parent_workspace_id
                 WHERE c.depth < 20
             )
             SELECT leaf FROM chain WHERE is_dev_workspace
             GROUP BY leaf, label HAVING count(*) > 1 LIMIT 1",
        )
        .fetch_optional(&db)
        .await?;
        assert!(
            duplicated.is_none(),
            "round {round}: a chain repeats a label, below {duplicated:?}"
        );
    }
    Ok(())
}

/// The pairing lock covers the chains an operation touches, not every pairing on the instance: a
/// transaction holding one family's nodes must not hold up another family's. Pinned because the
/// obvious way to make the races above safe — one key for the whole operation class — would serialize
/// dev-workspace creation database-wide, and creation holds its transaction across a full clone.
#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_pairing_lock_does_not_span_unrelated_families(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    // Hold family F's nodes the way an in-flight attach on it would, then act on family E.
    let mut held = db.begin().await?;
    for node in ["prod-f", "f-mid", "f-leaf"] {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtext('dev_workspace_pairing:' || $1))")
            .bind(node)
            .execute(&mut *held)
            .await?;
    }
    // Also the un-suffixed key. Nothing takes it today, so holding it costs the passing case
    // nothing — but a lock narrowed back to one key for every family would take it, and without
    // this the test would sail through that exact regression.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('dev_workspace_pairing'))")
        .execute(&mut *held)
        .await?;

    let (status, body) = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        detach(port, "prod-e", "wm-fork-edev"),
    )
    .await
    .map_err(|_| anyhow::anyhow!("an unrelated family's pairing blocked on family F's locks"))?;
    assert!(
        status.is_success(),
        "unrelated detach returned {status}: {body}"
    );

    held.rollback().await?;
    Ok(())
}

/// Archive resolves the workspace's pairing state before its transaction, so it takes the pairing
/// lock before reading that state again rather than on the strength of it — an attach can be turning
/// the workspace into a dev in the meantime. `h-cand` is standalone, the shape whose resolved state
/// says no pairing is involved: its archive must wait on the lock all the same.
#[sqlx::test(migrations = "../migrations", fixtures("base", "nested_dev_workspace"))]
async fn test_archive_takes_the_pairing_lock_for_a_standalone_workspace(
    db: Pool<Postgres>,
) -> anyhow::Result<()> {
    initialize_tracing().await;
    let server = ApiServer::start(db.clone()).await?;
    let port = server.addr.port();

    let mut held = db.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('dev_workspace_pairing:' || $1))")
        .bind("h-cand")
        .execute(&mut *held)
        .await?;

    let finished =
        tokio::time::timeout(std::time::Duration::from_secs(5), archive(port, "h-cand")).await;
    assert!(
        finished.is_err(),
        "archive of a standalone workspace completed while its pairing lock was held: {finished:?}"
    );

    held.rollback().await?;
    Ok(())
}
