//! Regression test for the raw-script cache authorization bypass.
//!
//! `GET /scripts/raw/p/{path}` answers from two process-global caches *before* the
//! authed RLS transaction runs: `RAW_SCRIPT_CACHE` (script content, keyed partly on
//! the caller-supplied `cache_key` query parameter) and `CACHE_FOLDERS_PATH` (folder
//! existence). Both were populated by authorized reads but keyed without any caller
//! identity, so any workspace member could replay another caller's request and read
//! folder-protected content, or learn that a folder they cannot see exists.
//!
//! Both keys now lead with `auth_identity(&authed)`, so a hit is only ever returned
//! to the caller whose own authorized read populated it. This pins that the caches
//! still hit for the warming caller (a key change must not silently disable them)
//! while a different caller replaying the same URL falls through to the RLS path and
//! is denied — for content on both the pinned and unpinned routes.

use sqlx::{Pool, Postgres};
use windmill_test_utils::*;

const CANARY: &str = "leak-canary";
const IS_FOLDER: &str = "WINDMILL_IS_FOLDER";
/// Replayed verbatim by the attacker: `cache_key` is caller-controlled, so an
/// attacker who learns (or guesses) a warmed one must still not get a hit.
const CACHE_KEY: &str = "424200";

async fn get(base: &str, path: &str, token: &str) -> (reqwest::StatusCode, String) {
    let resp = reqwest::Client::new()
        .get(format!("{base}/{path}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("request");
    let status = resp.status();
    (status, resp.text().await.expect("body"))
}

#[sqlx::test(fixtures("base", "raw_script_cache_authz"))]
async fn test_raw_script_caches_are_scoped_to_caller(db: Pool<Postgres>) -> anyhow::Result<()> {
    initialize_tracing().await;

    let server = ApiServer::start(db.clone()).await?;
    let base = format!(
        "http://localhost:{}/api/w/test-workspace/scripts",
        server.addr.port()
    );

    for route in ["raw", "raw_unpinned"] {
        let url = format!("{route}/p/f/secret/lib.py?cache_key={CACHE_KEY}");

        // The folder owner populates the content cache, then hits it. Both must
        // return the content: a second miss would mean the identity-scoped key
        // broke caching.
        for attempt in ["warm", "hit"] {
            let (status, body) = get(&base, &url, "SECRET_TOKEN").await;
            assert_eq!(
                status,
                reqwest::StatusCode::OK,
                "owner {attempt} on /{route} must succeed: {body}"
            );
            assert!(
                body.contains(CANARY),
                "owner {attempt} on /{route} returned unexpected content: {body}"
            );
        }

        // CORE REGRESSION: a member with no access to folder `secret` replays the
        // identical URL. Pre-fix this returned 200 with the content off the warm
        // cache, before any RLS query ran.
        let (status, body) = get(&base, &url, "SECRET_TOKEN_2").await;
        assert_ne!(
            status,
            reqwest::StatusCode::OK,
            "non-member must not read f/secret/lib via /{route} (got {status}): {body}"
        );
        assert!(
            !body.contains(CANARY),
            "non-member response on /{route} leaked the script content: {body}"
        );
    }

    // Folder-existence cache: reading the leaf caches every ancestor folder, so a
    // later probe of the intermediate path answers WINDMILL_IS_FOLDER off the cache.
    let (status, _) = get(
        &base,
        "raw/p/f/secret/pkg/leaf.py?cache_folders=true",
        "SECRET_TOKEN",
    )
    .await;
    assert_eq!(status, reqwest::StatusCode::OK, "owner must read the leaf");

    let probe = "raw/p/f/secret/pkg.py?cache_folders=true";
    let (status, body) = get(&base, probe, "SECRET_TOKEN").await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(body, IS_FOLDER, "owner probe must hit the folder cache");

    // Pre-fix the non-member got that same 200/WINDMILL_IS_FOLDER, revealing that a
    // folder they cannot see exists.
    let (status, body) = get(&base, probe, "SECRET_TOKEN_2").await;
    assert_ne!(
        status,
        reqwest::StatusCode::OK,
        "non-member folder probe must not succeed (got {status}): {body}"
    );
    assert!(
        !body.contains(IS_FOLDER),
        "non-member folder probe leaked folder existence: {body}"
    );

    Ok(())
}
