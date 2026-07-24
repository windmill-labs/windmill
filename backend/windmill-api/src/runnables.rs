/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Unified, keyset-paginated listing of a workspace's runnables (scripts,
//! flows, apps) merged into one globally-ordered stream. The homepage uses it
//! so a chosen order (recently updated / oldest / name) is correct and complete
//! across all three kinds at any workspace size, instead of client-sorting a
//! per-kind capped window.
//!
//! Efficiency: each kind is a UNION ALL branch ordered by an index on
//! `(workspace_id, archived, <sort key>)` (created_at / edited_at, or the lowered
//! summary-or-path expression for name orders); Postgres merges the ordered
//! branches and stops at the page limit. Pagination is keyset — a
//! `(sort_key, path, kind, tiebreak)` cursor, where `tiebreak` is a script's hash
//! (0 for the one-row-per-path flow/app) so archived versions sharing a name and
//! path stay individually reachable — so deep pages don't re-scan. Visibility is
//! enforced in-SQL by RLS via the `user_db` transaction.

use crate::db::{ApiAuthed, DB};
use crate::utils::{build_scope_path_filter, ScopePathFilter};
use axum::{
    extract::{Extension, Path, Query},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
};
use windmill_types::scripts::ScriptHash;
use windmill_types::user_drafts::DraftUserRef;

pub fn workspaced_service() -> Router {
    Router::new().route("/list", get(list_runnables))
}

#[derive(Deserialize)]
struct ListRunnablesQuery {
    /// `updated` (default) or `name`.
    order_by: Option<String>,
    /// Descending when true (default true).
    order_desc: Option<bool>,
    /// Comma-separated subset of `script,flow,app`; omitted means all.
    kinds: Option<String>,
    show_archived: Option<bool>,
    /// Include library scripts (no runnable main). Ignored for flows/apps.
    include_without_main: Option<bool>,
    /// Restrict to paths under this prefix (owner/folder filter).
    path_start: Option<String>,
    /// Comma-separated labels; a row matches if it (or its folder) carries all.
    label: Option<String>,
    /// Case-insensitive substring match on summary or path.
    search: Option<String>,
    per_page: Option<usize>,
    /// Opaque keyset cursor from a previous page's `next_cursor`.
    cursor: Option<String>,
}

// Absent optional fields are omitted (not serialized as null) to match the
// per-kind list contract; the frontend row components expect `undefined`.
#[derive(Serialize, sqlx::FromRow)]
struct RunnableItem {
    #[serde(rename = "type")]
    kind: String, // 'script' | 'flow' | 'app'
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    workspace_id: String,
    extra_perms: serde_json::Value,
    starred: bool,
    archived: bool,
    is_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    draft_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    draft_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    draft_users: Option<sqlx::types::Json<Vec<DraftUserRef>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inherited_labels: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ws_error_handler_muted: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    edited_at: Option<chrono::DateTime<chrono::Utc>>,
    // script-only. ScriptHash serializes as the 16-char hex string that
    // /scripts/get/{hash} parses (a raw i64 would produce a broken link).
    #[serde(skip_serializing_if = "Option::is_none")]
    hash: Option<ScriptHash>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language: Option<String>,
    #[serde(rename = "kind", skip_serializing_if = "Option::is_none")]
    script_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    auto_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    use_codebase: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    has_deploy_errors: Option<bool>,
    // app-only
    #[serde(skip_serializing_if = "Option::is_none")]
    raw_app: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
    // sort keys, echoed into the cursor (not serialized to the client)
    #[serde(skip)]
    sort_time: chrono::DateTime<chrono::Utc>,
    #[serde(skip)]
    sort_name: String,
    // Final tiebreaker making the sort total: a script's hash (the archived view
    // lists several versions sharing path+summary), 0 for the one-row-per-path
    // flow/app. Without it a group of same-(name,path) versions crossing a page
    // boundary would be skipped by the strict keyset.
    #[serde(skip)]
    tiebreak: i64,
}

#[derive(Serialize)]
struct ListRunnablesResponse {
    items: Vec<RunnableItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Cursor {
    /// sort key of the last row: rfc3339 timestamp (updated) or lowered name.
    k: String,
    p: String,
    t: String,
    /// tiebreak (script hash / 0) of the last row.
    #[serde(default)]
    tb: i64,
}

fn encode_cursor(item: &RunnableItem, order_by_name: bool) -> String {
    let k = if order_by_name {
        item.sort_name.clone()
    } else {
        item.sort_time.to_rfc3339()
    };
    let c = Cursor { k, p: item.path.clone(), t: item.kind.clone(), tb: item.tiebreak };
    URL_SAFE_NO_PAD.encode(serde_json::to_vec(&c).unwrap_or_default())
}

fn decode_cursor(raw: &str) -> Result<Cursor, Error> {
    let bytes = URL_SAFE_NO_PAD
        .decode(raw)
        .map_err(|_| Error::BadRequest("invalid cursor".to_string()))?;
    serde_json::from_slice(&bytes).map_err(|_| Error::BadRequest("invalid cursor".to_string()))
}

/// Escape LIKE/ILIKE wildcards so a caller value (search term, path/scope
/// prefix) matches literally. Relies on the default `\` escape character.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// The three UNION-ALL branch SELECTs, each projecting the shared `RunnableItem`
/// column set (NULL for columns that don't apply to that kind). `$1`=workspace,
/// `$2`=username (favorites), `$3`=email (drafts). Kind-specific and per-request
/// WHERE fragments are appended by the caller.
struct Branches {
    script: String,
    flow: String,
    app: String,
}

fn branch_sqls() -> Branches {
    // draft_users subquery (correlated) mirrors the per-kind list endpoints; run
    // only for the returned page, so its cost is bounded by per_page.
    let draft_users = |typ_pred: &str| -> String {
        format!(
            "(SELECT json_agg(json_build_object('username', COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END)) ORDER BY COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END) NULLS LAST) \
              FROM draft d \
              LEFT JOIN usr u ON u.workspace_id = d.workspace_id AND u.email = d.email \
              LEFT JOIN password p ON p.email = d.email AND p.super_admin = true \
              WHERE d.workspace_id = o.workspace_id AND d.path = o.path AND {typ_pred}) as draft_users"
        )
    };

    let script = format!(
        "SELECT 'script' as kind, o.path, o.summary, o.workspace_id, o.extra_perms, \
                favorite.path IS NOT NULL as starred, o.archived, \
                draft.email IS NOT NULL as is_draft, NULL::bool as draft_only, NULL::text as draft_path, \
                {draft_users}, o.labels, folder_labels(o.workspace_id, o.path) as inherited_labels, \
                o.ws_error_handler_muted, o.created_at as edited_at, \
                o.hash, o.language::text as language, o.kind::text as script_kind, o.auto_kind, \
                o.codebase IS NOT NULL as use_codebase, \
                (o.lock_error_logs IS NOT NULL) as has_deploy_errors, \
                NULL::bool as raw_app, NULL::text as execution_mode, NULL::bigint as id, NULL::bigint as version, \
                o.created_at as sort_time, lower(COALESCE(NULLIF(o.summary, ''), o.path)) as sort_name, o.hash as tiebreak \
         FROM script o \
         LEFT JOIN favorite ON favorite.favorite_kind = 'script' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = $2 \
         LEFT JOIN draft ON draft.path = o.path AND draft.workspace_id = o.workspace_id AND draft.typ = 'script' AND draft.email = $3",
        draft_users = draft_users("d.typ = 'script'")
    );

    let flow = format!(
        "SELECT 'flow' as kind, o.path, o.summary, o.workspace_id, o.extra_perms, \
                favorite.path IS NOT NULL as starred, o.archived, \
                draft.email IS NOT NULL as is_draft, NULL::bool as draft_only, NULL::text as draft_path, \
                {draft_users}, o.labels, folder_labels(o.workspace_id, o.path) as inherited_labels, \
                o.ws_error_handler_muted, o.edited_at, \
                NULL::bigint as hash, NULL::text as language, NULL::text as script_kind, NULL::text as auto_kind, \
                NULL::bool as use_codebase, NULL::bool as has_deploy_errors, \
                NULL::bool as raw_app, NULL::text as execution_mode, NULL::bigint as id, NULL::bigint as version, \
                o.edited_at as sort_time, lower(COALESCE(NULLIF(o.summary, ''), o.path)) as sort_name, 0::bigint as tiebreak \
         FROM flow o \
         LEFT JOIN favorite ON favorite.favorite_kind = 'flow' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = $2 \
         LEFT JOIN draft ON draft.path = o.path AND draft.workspace_id = o.workspace_id AND draft.typ = 'flow' AND draft.email = $3",
        draft_users = draft_users("d.typ = 'flow'")
    );

    let app = format!(
        "SELECT 'app' as kind, o.path, o.summary, o.workspace_id, o.extra_perms, \
                favorite.path IS NOT NULL as starred, false as archived, \
                draft.path IS NOT NULL as is_draft, NULL::bool as draft_only, NULL::text as draft_path, \
                {draft_users}, o.labels, folder_labels(o.workspace_id, o.path) as inherited_labels, \
                NULL::bool as ws_error_handler_muted, av.created_at as edited_at, \
                NULL::bigint as hash, NULL::text as language, NULL::text as script_kind, NULL::text as auto_kind, \
                NULL::bool as use_codebase, NULL::bool as has_deploy_errors, \
                av.raw_app, o.policy->>'execution_mode' as execution_mode, o.id, \
                o.versions[array_upper(o.versions, 1)] as version, \
                COALESCE(av.created_at, 'epoch'::timestamptz) as sort_time, lower(COALESCE(NULLIF(o.summary, ''), o.path)) as sort_name, 0::bigint as tiebreak \
         FROM app o \
         LEFT JOIN favorite ON favorite.favorite_kind = 'app' AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = $2 \
         LEFT JOIN (SELECT DISTINCT path, workspace_id FROM draft WHERE typ IN ('app', 'raw_app') AND email = $3) draft ON draft.path = o.path AND draft.workspace_id = o.workspace_id \
         LEFT JOIN app_version av ON av.id = o.versions[array_upper(o.versions, 1)]",
        draft_users = draft_users("d.typ IN ('app', 'raw_app')")
    );

    Branches { script, flow, app }
}

async fn list_runnables(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Extension(_db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<ListRunnablesQuery>,
) -> JsonResult<ListRunnablesResponse> {
    let order_by_name = q.order_by.as_deref() == Some("name");
    let desc = q.order_desc.unwrap_or(true);
    let per_page = q.per_page.unwrap_or(50).clamp(1, 1000);
    let show_archived = q.show_archived.unwrap_or(false);
    let order_dir = if desc { "DESC" } else { "ASC" };
    let sort_col = if order_by_name {
        "sort_name"
    } else {
        "sort_time"
    };

    let mut kinds: Vec<&str> = match q.kinds.as_deref() {
        None | Some("") => vec!["script", "flow", "app"],
        Some(csv) => csv
            .split(',')
            .map(|s| s.trim())
            .filter(|s| ["script", "flow", "app"].contains(s))
            .collect(),
    };
    // Apps carry no `archived` column and are never listed as archived.
    if show_archived {
        kinds.retain(|k| *k != "app");
    }
    // Operators may only see scripts.
    if authed.is_operator {
        kinds.retain(|k| *k == "script");
    }

    let branches = branch_sqls();

    // Params after the fixed $1=w_id, $2=username, $3=email. `add_bind` returns
    // the next placeholder (`$N`) and records the value in order.
    let mut binds: Vec<String> = vec![];
    let add_bind = |binds: &mut Vec<String>, v: String| -> String {
        binds.push(v);
        format!("${}", 3 + binds.len())
    };

    let mut common: Vec<String> = vec!["o.workspace_id = $1".to_string()];
    if let Some(ps) = q.path_start.as_ref().filter(|s| !s.is_empty()) {
        let p = add_bind(&mut binds, format!("{}%", escape_like(ps)));
        common.push(format!("o.path LIKE {}", p));
    }
    if let Some(search) = q.search.as_ref().filter(|s| !s.is_empty()) {
        let p = add_bind(&mut binds, format!("%{}%", escape_like(search)));
        common.push(format!("(o.summary ILIKE {p} OR o.path ILIKE {p})"));
    }
    if let Some(label) = q.label.as_ref().filter(|s| !s.is_empty()) {
        for l in label.split(',') {
            let p = add_bind(&mut binds, l.trim().to_string());
            common.push(format!(
                "(o.labels @> ARRAY[{p}] OR folder_labels(o.workspace_id, o.path) @> ARRAY[{p}])"
            ));
        }
    }
    let common_where = common.join(" AND ");

    // Keyset predicate for pages after the first (non-starred rows only). A
    // row-value comparison keeps the composite order; the key is cast to the
    // branch column's type.
    let keyset_sql: Option<String> = match &q.cursor {
        Some(raw) => {
            let cur = decode_cursor(raw)?;
            let kp = add_bind(&mut binds, cur.k);
            let pp = add_bind(&mut binds, cur.p);
            let tp = add_bind(&mut binds, cur.t);
            let tbp = add_bind(&mut binds, cur.tb.to_string());
            let cmp = if desc { "<" } else { ">" };
            let key_cast = if order_by_name {
                format!("{}::text", kp)
            } else {
                format!("{}::timestamptz", kp)
            };
            Some(format!(
                "({sort_col}, path, kind, tiebreak) {cmp} ({key_cast}, {pp}::text, {tp}::text, {tbp}::bigint)"
            ))
        }
        None => None,
    };

    // Fine-grained scoped tokens (e.g. `scripts:read:f/foo/*`) must be confined to
    // their granted paths. RLS alone doesn't honor token scopes, so push the
    // per-domain path grant into SQL (empty grant -> the branch matches nothing).
    // Unscoped sessions -> AllowAll -> no predicate.
    let scope_where = |filter: ScopePathFilter, binds: &mut Vec<String>| -> Option<String> {
        match filter {
            ScopePathFilter::AllowAll => None,
            ScopePathFilter::Restricted { exact, prefix } => {
                let mut terms: Vec<String> = vec![];
                for e in exact {
                    binds.push(e);
                    terms.push(format!("o.path = ${}", 3 + binds.len()));
                }
                for pre in prefix {
                    binds.push(pre.clone());
                    let pe = format!("${}", 3 + binds.len());
                    binds.push(format!("{}/%", escape_like(&pre)));
                    let pl = format!("${}", 3 + binds.len());
                    terms.push(format!("(o.path = {} OR o.path LIKE {})", pe, pl));
                }
                Some(if terms.is_empty() {
                    "false".to_string()
                } else {
                    format!("({})", terms.join(" OR "))
                })
            }
        }
    };
    let script_scope = scope_where(
        build_scope_path_filter(&authed, "scripts", "read"),
        &mut binds,
    );
    let flow_scope = scope_where(
        build_scope_path_filter(&authed, "flows", "read"),
        &mut binds,
    );
    let app_scope = scope_where(build_scope_path_filter(&authed, "apps", "read"), &mut binds);

    // Per-kind archived predicate (scripts/flows have the column; apps don't and
    // are excluded from the archived view).
    let archived_pred = if show_archived {
        "o.archived = true"
    } else {
        "o.archived = false"
    };
    let mut script_extras: Vec<String> = vec![];
    if !q.include_without_main.unwrap_or(false) || authed.is_operator {
        script_extras.push("(o.auto_kind IS NULL OR o.auto_kind <> 'lib')".to_string());
    }
    script_extras.push(archived_pred.to_string());
    if show_archived {
        // The script table keeps every version as its own row and marks superseded
        // ones archived=true, so a bare `archived = true` would surface an active
        // path's old versions and repeat a genuinely archived path once per version.
        // Match the canonical script listing: only a path whose LATEST row is archived
        // belongs in the archived view. (Flows/apps are one row per path, so this only
        // applies to scripts.)
        script_extras.push(
            "o.ctid = (SELECT ctid FROM script s2 WHERE s2.path = o.path \
             AND s2.workspace_id = o.workspace_id ORDER BY s2.created_at DESC LIMIT 1)"
                .to_string(),
        );
    }
    if let Some(s) = &script_scope {
        script_extras.push(s.clone());
    }
    let mut flow_extras: Vec<String> = vec![archived_pred.to_string()];
    if let Some(s) = &flow_scope {
        flow_extras.push(s.clone());
    }
    let mut app_extras: Vec<String> = vec![];
    if let Some(s) = &app_scope {
        app_extras.push(s.clone());
    }

    // Favorite filter for a branch: Some(true) = starred only, Some(false) =
    // non-starred only, None = no filter (the archived view doesn't pin starred,
    // so its single stream includes both — see the query assembly below).
    let build_branch = |base: &str,
                        kind: &str,
                        extras: &[String],
                        fav: Option<bool>,
                        keyset: Option<&str>,
                        limit: Option<usize>|
     -> String {
        // Base-table predicates go inside the projection subquery (they read
        // o.*/favorite.*); the keyset reads the projected sort aliases, so it
        // sits in the wrapper WHERE where those aliases are visible.
        let mut w = vec![common_where.clone()];
        w.extend(extras.iter().cloned());
        match fav {
            Some(true) => w.push("favorite.path IS NOT NULL".to_string()),
            Some(false) => w.push("favorite.path IS NULL".to_string()),
            None => {}
        }
        let keyset_clause = keyset
            .map(|ks| format!(" WHERE {}", ks))
            .unwrap_or_default();
        // Per-branch LIMIT so each branch's correlated projections (draft_users,
        // folder_labels) are evaluated only for its own top rows, not the whole
        // table; the outer union re-limits to the global page.
        let limit_clause = limit.map(|n| format!(" LIMIT {}", n)).unwrap_or_default();
        format!(
            "(SELECT * FROM ({base} WHERE {where_}) {kind}_b{keyset_clause} ORDER BY {sort_col} {dir}, path {dir}, kind {dir}, tiebreak {dir}{limit_clause})",
            where_ = w.join(" AND "),
            dir = order_dir,
        )
    };

    let branch_for = |kind: &str,
                      fav: Option<bool>,
                      keyset: Option<&str>,
                      limit: Option<usize>|
     -> Option<String> {
        if !kinds.contains(&kind) {
            return None;
        }
        let (base, extras): (&str, &[String]) = match kind {
            "script" => (&branches.script, &script_extras),
            "flow" => (&branches.flow, &flow_extras),
            "app" => (&branches.app, &app_extras),
            _ => return None,
        };
        Some(build_branch(base, kind, extras, fav, keyset, limit))
    };

    let run_union = |branches_sql: Vec<String>, limit: Option<usize>| -> String {
        let unioned = branches_sql.join(" UNION ALL ");
        let limit_clause = limit.map(|n| format!(" LIMIT {}", n)).unwrap_or_default();
        format!(
            "SELECT * FROM ({unioned}) q ORDER BY {sort_col} {dir}, path {dir}, kind {dir}, tiebreak {dir}{limit_clause}",
            dir = order_dir,
        )
    };

    let mut tx = user_db.begin(&authed).await?;
    let mut items: Vec<RunnableItem> = vec![];
    let first_page = q.cursor.is_none();

    // Pin starred on the first page. Both views are now one row per path (the
    // archived view filters to each path's latest row, see archived_pred), so a
    // favorite is a single row in either — the starred-first contract holds in the
    // archived view too, and the pinned first page stays bounded.
    if first_page {
        let starred_branches: Vec<String> = ["script", "flow", "app"]
            .iter()
            .filter_map(|k| branch_for(k, Some(true), None, None))
            .collect();
        if !starred_branches.is_empty() {
            let sql = run_union(starred_branches, None);
            let mut query = sqlx::query_as::<_, RunnableItem>(&sql)
                .bind(&w_id)
                .bind(&authed.username)
                .bind(&authed.email);
            for b in &binds {
                query = query.bind(b);
            }
            items.extend(query.fetch_all(&mut *tx).await?);
        }
    }

    // Main paged stream: non-starred rows (starred were pinned on the first page above).
    let main_fav = Some(false);
    let ns_branches: Vec<String> = ["script", "flow", "app"]
        .iter()
        .filter_map(|k| branch_for(k, main_fav, keyset_sql.as_deref(), Some(per_page)))
        .collect();

    let mut next_cursor: Option<String> = None;
    if !ns_branches.is_empty() {
        let sql = run_union(ns_branches, Some(per_page));
        let mut query = sqlx::query_as::<_, RunnableItem>(&sql)
            .bind(&w_id)
            .bind(&authed.username)
            .bind(&authed.email);
        for b in &binds {
            query = query.bind(b);
        }
        let ns = query.fetch_all(&mut *tx).await?;
        if ns.len() == per_page {
            if let Some(last) = ns.last() {
                next_cursor = Some(encode_cursor(last, order_by_name));
            }
        }
        items.extend(ns);
    }

    tx.commit().await?;

    Ok(Json(ListRunnablesResponse { items, next_cursor }))
}
