use sqlx::{Pool, Postgres};
use windmill_trigger_http::{refresh_routers, HttpMethod, RoutersCache};

async fn insert_trigger(db: &Pool<Postgres>, path: &str, route_path: &str) {
    sqlx::query(
        "INSERT INTO http_trigger (
            path, route_path, route_path_key, script_path, is_flow, workspace_id, edited_by,
            permissioned_as, http_method, authentication_method, request_type, is_static_website,
            workspaced_route, wrap_body, raw_string, mode
        ) VALUES ($1, $2, $2, 'f/test/handler', false, 'test-workspace', 'test-user',
            'u/test-user', 'get', 'none', 'async', false, false, false, false, 'enabled')",
    )
    .bind(path)
    .bind(route_path)
    .execute(db)
    .await
    .expect("insert http_trigger");
}

fn routes(cache: &RoutersCache, path: &str) -> bool {
    cache.routers[&HttpMethod::Get].at(path).is_ok()
}

// A trigger row can commit without advancing http_trigger_version_seq past what the cache
// already holds, because `nextval` runs ahead of the commit it belongs to. Only a forced
// refresh recovers the route from there.
#[sqlx::test(migrations = "../migrations")]
async fn refresh_routers_force_rebuilds_on_an_unchanged_version(db: Pool<Postgres>) {
    insert_trigger(&db, "f/test/first", "first").await;
    let (rebuilt, cache) = refresh_routers(&db, false).await.unwrap();
    assert!(rebuilt);
    assert!(routes(&cache, "/first"));
    drop(cache);

    insert_trigger(&db, "f/test/second", "second").await;

    let (rebuilt, cache) = refresh_routers(&db, false).await.unwrap();
    assert!(!rebuilt, "an unchanged version must not rebuild");
    assert!(!routes(&cache, "/second"));
    drop(cache);

    let (rebuilt, cache) = refresh_routers(&db, true).await.unwrap();
    assert!(rebuilt, "force must rebuild whatever the version says");
    assert!(routes(&cache, "/second"));
}
