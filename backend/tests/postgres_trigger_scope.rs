//! Regression test for the Postgres-trigger ancillary routes (slot / publication
//! / version management) scope enforcement.
//!
//! The route-level middleware only checks that a token carries *some*
//! `postgres_triggers` scope; per-resource-path enforcement is delegated to each
//! handler. These handlers previously made no `check_scopes` call, so a token
//! scoped to one resource path could drive slot/publication management (including
//! the destructive `drop_slot_name`, which terminates the active backend and
//! drops the replication slot) against any Postgres resource in the workspace.
//!
//! Each handler now calls `check_scopes` before touching the database, so a
//! path-mismatched scoped token is rejected before any connection is opened —
//! which is exactly why this test needs no real Postgres resource.

use axum::{extract::Path, Extension, Json};
use sqlx::{Pool, Postgres};
use windmill_api_auth::ApiAuthed;
use windmill_common::{db::UserDB, error::Error};
use windmill_trigger_postgres::{handler, Slot};

fn scoped_authed(scopes: Vec<&str>) -> ApiAuthed {
    ApiAuthed {
        email: "alice@windmill.dev".to_string(),
        username: "alice".to_string(),
        is_admin: false,
        is_operator: false,
        groups: vec![],
        folders: vec![],
        scopes: Some(scopes.into_iter().map(str::to_string).collect()),
        username_override: None,
        token_prefix: None,
        read_only: false,
    }
}

// A token scoped to `u/alice/db` must not reach a read handler for `u/bob/db`.
#[sqlx::test]
async fn read_handler_rejects_path_mismatched_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    let authed = scoped_authed(vec!["postgres_triggers:read:u/alice/db"]);
    let user_db = UserDB::new(db.clone());

    let res = handler::get_postgres_version(
        authed,
        Extension(db),
        Extension(user_db),
        Path(("test-workspace".to_string(), "u/bob/db".to_string())),
    )
    .await;

    assert!(
        matches!(res, Err(Error::PermissionDenied(_))),
        "expected PermissionDenied, got {res:?}"
    );
    Ok(())
}

// The destructive slot-drop handler must reject a write token scoped to another path.
#[sqlx::test]
async fn drop_slot_rejects_path_mismatched_scope(db: Pool<Postgres>) -> anyhow::Result<()> {
    let authed = scoped_authed(vec!["postgres_triggers:write:u/alice/db"]);
    let user_db = UserDB::new(db.clone());

    let res = handler::drop_slot_name(
        authed,
        Extension(user_db),
        Extension(db),
        Path(("test-workspace".to_string(), "u/bob/db".to_string())),
        Json(Slot { name: "some_slot".to_string() }),
    )
    .await;

    assert!(
        matches!(res, Err(Error::PermissionDenied(_))),
        "expected PermissionDenied, got {res:?}"
    );
    Ok(())
}
