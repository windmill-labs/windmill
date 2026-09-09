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
//! `(sort_key, path, kind, tiebreak)` cursor, where `tiebreak` (a script's hash,
//! 0 for flow/app) is a stable final key that keeps the order total even when rows
//! tie on (sort_key, path, kind) — so deep pages don't re-scan. Visibility is
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
use std::collections::HashMap;
use windmill_common::{
    db::UserDB,
    error::{Error, JsonResult},
};
use windmill_types::scripts::ScriptHash;
use windmill_types::user_drafts::DraftUserRef;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_runnables))
        .route("/counts", get(count_runnables_by_owner))
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
    /// Case-insensitive fuzzy match on `summary (path)`, mirroring how the homepage
    /// ranks: split into terms on anything but ASCII letters, digits and apostrophes,
    /// then every term must appear whole and in order, with anything in between. Only
    /// the first `MAX_SEARCH_TERMS` apply. Omitted or empty filters nothing; a query
    /// that holds no ASCII-alphanumeric character at all — `" "`, `"_"`, `"привет"` —
    /// yields no terms and matches nothing, as it does on the homepage.
    search: Option<String>,
    per_page: Option<usize>,
    /// Opaque keyset cursor from a previous page's `next_cursor`.
    cursor: Option<String>,
    /// Also list the caller's drafts at paths with no deployed row. Off by
    /// default so picker callers stay deployed-only.
    include_draft_only: Option<bool>,
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
    // Final tiebreaker making the sort total: a script's hash, 0 for flow/app. A stable
    // last key so rows that tie on (sort_key, path, kind) still have a strict order and
    // none is skipped when a tie crosses a page boundary.
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

/// Upper bound on the terms one search query contributes to the pattern, so a
/// pasted paragraph cannot grow it without limit.
const MAX_SEARCH_TERMS: usize = 8;

/// Split a query into terms on runs of anything that is not an ASCII letter, digit
/// or apostrophe — the rule the homepage's fuzzy matcher uses, so `f/foo_bar` looks
/// for `foo` then `bar` rather than for the punctuation between them.
fn search_terms(search: &str) -> Vec<&str> {
    search
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '\''))
        .filter(|t| !t.is_empty())
        .take(MAX_SEARCH_TERMS)
        .collect()
}

/// The string a search matches against: `summary (path)`, or the bare path when
/// there is no summary — what the homepage concatenates before ranking, so both
/// halves are searchable as one and a query may span them.
fn searchable_name(path_expr: &str) -> String {
    format!("COALESCE(NULLIF(o.summary, '') || ' (' || {path_expr} || ')', {path_expr})")
}

/// Escape LIKE/ILIKE wildcards so a caller value (search term, path/scope
/// prefix) matches literally. Relies on the default `\` escape character.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// The `domain:action` scope grant of a fine-grained token (e.g.
/// `scripts:read:f/foo/*`) as a predicate on `{alias}.path`, with its values
/// appended to `binds` (placeholders numbered from `base + 1`). `None` means the
/// token is unscoped for that domain and needs no predicate; an empty grant
/// yields `false` so the branch matches nothing. RLS doesn't honor token scopes,
/// so this has to be pushed into SQL.
fn scope_path_predicate(
    authed: &ApiAuthed,
    domain: &str,
    alias: &str,
    base: usize,
    binds: &mut Vec<String>,
) -> Option<String> {
    match build_scope_path_filter(authed, domain, "read") {
        ScopePathFilter::AllowAll => None,
        ScopePathFilter::Restricted { exact, prefix } => {
            let mut terms: Vec<String> = vec![];
            for e in exact {
                binds.push(e);
                terms.push(format!("{}.path = ${}", alias, base + binds.len()));
            }
            for pre in prefix {
                binds.push(pre.clone());
                let pe = format!("${}", base + binds.len());
                binds.push(format!("{}/%", escape_like(&pre)));
                let pl = format!("${}", base + binds.len());
                terms.push(format!(
                    "({}.path = {} OR {}.path LIKE {})",
                    alias, pe, alias, pl
                ));
            }
            Some(if terms.is_empty() {
                "false".to_string()
            } else {
                format!("({})", terms.join(" OR "))
            })
        }
    }
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
              WHERE d.workspace_id = o.workspace_id AND d.path = o.path AND {typ_pred} \
                AND (d.email IS NULL OR u.username IS NOT NULL OR p.email IS NOT NULL)) as draft_users"
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

/// The draft-only branch for a kind: the caller's drafts at paths carrying no
/// deployed row, projected into the same column set as `branch_sqls` so they
/// sort, search and paginate as ordinary rows. Same `$1`/`$2`/`$3` contract.
fn draft_branch_sql(kind: &str) -> String {
    let (typ_pred, deployed) = match kind {
        "script" => ("d.typ = 'script'", "script"),
        "flow" => ("d.typ = 'flow'", "flow"),
        _ => ("d.typ IN ('app', 'raw_app')", "app"),
    };
    // Scripts bind the Path widget to `script.path`, so the typed path round-trips
    // through the draft JSON's own `path`; flows and apps write a separate
    // `draft_path` only when it differs from the deployed one. See scripts.rs.
    let typed_path = if kind == "script" {
        "path"
    } else {
        "draft_path"
    };
    // `auto_kind` is only what the editor stamped into the draft — a `// pipeline`
    // annotation is not re-derived from the content here, unlike the per-kind
    // endpoints. That errs toward listing a pipeline member as its own row rather
    // than folding it into a pipeline entry and hiding it.
    let kind_cols = match kind {
        "script" => {
            "d.value->>'language' as language, d.value->>'kind' as script_kind, \
                     d.value->>'auto_kind' as auto_kind, false as raw_app"
        }
        _ => {
            "NULL::text as language, NULL::text as script_kind, NULL::text as auto_kind, \
              (d.typ = 'raw_app') as raw_app"
        }
    };
    format!(
        "SELECT '{kind}' as kind, o.path, o.summary, o.workspace_id, '{{}}'::jsonb as extra_perms, \
                false as starred, false as archived, \
                true as is_draft, true as draft_only, o.draft_path, \
                json_build_array(json_build_object('username', $2::text)) as draft_users, \
                NULL::text[] as labels, NULL::text[] as inherited_labels, \
                NULL::bool as ws_error_handler_muted, o.created_at as edited_at, \
                NULL::bigint as hash, o.language, o.script_kind, o.auto_kind, \
                NULL::bool as use_codebase, NULL::bool as has_deploy_errors, \
                o.raw_app, NULL::text as execution_mode, NULL::bigint as id, NULL::bigint as version, \
                o.created_at as sort_time, lower(COALESCE(NULLIF(o.summary, ''), o.draft_path, o.path)) as sort_name, 0::bigint as tiebreak \
         FROM ( \
             SELECT DISTINCT ON (d.path) d.workspace_id, d.path, d.created_at, \
                    COALESCE(d.value->>'summary', '') as summary, \
                    NULLIF(NULLIF(d.value->>'{typed_path}', ''), d.path) as draft_path, \
                    {kind_cols} \
             FROM draft d \
             WHERE d.workspace_id = $1 AND {typ_pred} AND (d.email = $3 OR d.email IS NULL) \
               AND NOT EXISTS (SELECT 1 FROM {deployed} x \
                               WHERE x.workspace_id = d.workspace_id AND x.path = d.path) \
             -- Owned draft over a legacy NULL-email one, then newest: the app branch spans
             -- two draft kinds (`app` and `raw_app`) that can both exist at a path, and the
             -- pick decides raw_app, summary and draft_path. Same tiebreak as apps.rs.
             ORDER BY d.path, (d.email IS NULL), d.created_at DESC \
         ) o"
    )
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

    let branches = branch_sqls();

    // Params after the fixed $1=w_id, $2=username, $3=email. `add_bind` returns
    // the next placeholder (`$N`) and records the value in order.
    let mut binds: Vec<String> = vec![];
    let add_bind = |binds: &mut Vec<String>, v: String| -> String {
        binds.push(v);
        format!("${}", 3 + binds.len())
    };

    let mut common: Vec<String> = vec!["o.workspace_id = $1".to_string()];
    // Same predicates for the draft branches, except the search: a draft is named by
    // the path typed in the editor, its stored path being a generated `draft_<uuid>`
    // nobody searches for. Labels never apply (the draft branches are dropped under a
    // label filter), so they are not repeated here.
    let mut draft_common: Vec<String> = vec!["o.workspace_id = $1".to_string()];
    if let Some(ps) = q.path_start.as_ref().filter(|s| !s.is_empty()) {
        let p = add_bind(&mut binds, format!("{}%", escape_like(ps)));
        common.push(format!("o.path LIKE {}", p));
        // An owner filter follows where the draft says it will live, not the
        // `u/<caller>/draft_<uuid>` it is parked at.
        draft_common.push(format!("COALESCE(o.draft_path, o.path) LIKE {}", p));
    }
    if let Some(search) = q.search.as_ref().filter(|s| !s.is_empty()) {
        let terms = search_terms(search);
        if terms.is_empty() {
            // A search of nothing but separators is still a search: it must match nothing,
            // where no predicate at all would answer it with the whole workspace.
            common.push("false".to_string());
            draft_common.push("false".to_string());
        } else {
            // `%` between the terms is what makes them a fuzzy match rather than a literal
            // one: each must appear whole, in this order, with anything in between. The
            // haystack is the summary-and-path string the homepage matches on, so a query
            // may span the two and the endpoint withholds nothing the homepage would rank.
            let p = add_bind(
                &mut binds,
                format!(
                    "%{}%",
                    terms
                        .iter()
                        .map(|t| escape_like(t))
                        .collect::<Vec<_>>()
                        .join("%")
                ),
            );
            common.push(format!("{} ILIKE {p}", searchable_name("o.path")));
            draft_common.push(format!(
                "{} ILIKE {p}",
                searchable_name("COALESCE(o.draft_path, o.path)")
            ));
        }
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
    let draft_common_where = draft_common.join(" AND ");

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

    // Only push scope binds for kinds whose branch is actually included: a scoped token
    // with e.g. `kinds=script` omits the flow/app branches, so binding their scope values
    // (which no SQL references) would make the parameter count mismatch and 500.
    let script_scope = if kinds.contains(&"script") {
        scope_path_predicate(&authed, "scripts", "o", 3, &mut binds)
    } else {
        None
    };
    let flow_scope = if kinds.contains(&"flow") {
        scope_path_predicate(&authed, "flows", "o", 3, &mut binds)
    } else {
        None
    };
    let app_scope = if kinds.contains(&"app") {
        scope_path_predicate(&authed, "apps", "o", 3, &mut binds)
    } else {
        None
    };

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

    // Draft-only rows are the caller's own work in progress: never archived, so they
    // have no place in the archived view, and carrying no labels of their own they are
    // out of scope of a label filter (as in the per-kind endpoints). Operators don't
    // see other people's drafts and have none of their own to see.
    let include_drafts = q.include_draft_only.unwrap_or(false)
        && !authed.is_operator
        && !show_archived
        && q.label.as_ref().filter(|s| !s.is_empty()).is_none();
    let draft_extras_for = |kind: &str| -> Vec<String> {
        let mut extras: Vec<String> = vec![];
        let scope = match kind {
            "script" => &script_scope,
            "flow" => &flow_scope,
            _ => &app_scope,
        };
        // The lib filter reads the same projected `auto_kind`, so a draft-only library
        // script hides with the deployed ones.
        if kind == "script" && (!q.include_without_main.unwrap_or(false) || authed.is_operator) {
            extras.push("(o.auto_kind IS NULL OR o.auto_kind <> 'lib')".to_string());
        }
        if let Some(s) = scope {
            extras.push(s.clone());
        }
        extras
    };

    // Favorite filter for a branch: Some(true) = starred only, Some(false) =
    // non-starred only, None = no filter. Both views pin starred on the first page
    // (each is one row per path), so the paged stream always passes Some(false).
    let build_branch = |base: &str,
                        kind: &str,
                        common: &str,
                        extras: &[String],
                        fav: Option<bool>,
                        keyset: Option<&str>,
                        limit: Option<usize>|
     -> String {
        // Base-table predicates go inside the projection subquery (they read
        // o.*/favorite.*); the keyset reads the projected sort aliases, so it
        // sits in the wrapper WHERE where those aliases are visible.
        let mut w = vec![common.to_string()];
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
        Some(build_branch(
            base,
            kind,
            &common_where,
            extras,
            fav,
            keyset,
            limit,
        ))
    };

    let draft_branch_for = |kind: &str,
                            fav: Option<bool>,
                            keyset: Option<&str>,
                            limit: Option<usize>|
     -> Option<String> {
        if !include_drafts || !kinds.contains(&kind) {
            return None;
        }
        // `fav` is ignored: with no favorite join there is nothing to filter on, and the
        // starred pass skips draft branches entirely.
        let _ = fav;
        Some(build_branch(
            &draft_branch_sql(kind),
            &format!("draft_{kind}"),
            &draft_common_where,
            &draft_extras_for(kind),
            None,
            keyset,
            limit,
        ))
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
        // No draft branch here: a draft-only path has no favorite row (the UI won't let
        // you star one), so it would scan the caller's whole draft slice per kind to
        // return nothing. The main stream below takes them unfiltered instead.
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
    let ns_branches: Vec<String> =
        ["script", "flow", "app"]
            .iter()
            .filter_map(|k| branch_for(k, main_fav, keyset_sql.as_deref(), Some(per_page)))
            .chain(["script", "flow", "app"].iter().filter_map(|k| {
                draft_branch_for(k, main_fav, keyset_sql.as_deref(), Some(per_page))
            }))
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

#[derive(Deserialize)]
struct CountRunnablesQuery {
    /// Comma-separated subset of `script,flow,app`; omitted means all.
    kinds: Option<String>,
    /// Include library scripts (no runnable main). Ignored for flows/apps.
    include_without_main: Option<bool>,
    /// Also count the caller's drafts at paths with no deployed row, matching
    /// the same flag on `/list`.
    include_draft_only: Option<bool>,
}

#[derive(Serialize)]
struct RunnableCountsResponse {
    /// Owner prefix (`f/<folder>` or `u/<user>`) -> number of visible runnables,
    /// counting what the tree lists as a row (see the pipeline note below).
    /// Owners with none are omitted so the tree can hide them.
    counts: HashMap<String, i64>,
}

#[derive(sqlx::FromRow)]
struct OwnerCount {
    owner: String,
    count: i64,
}

/// A byte-ordered range covering exactly the paths under `o.owner`: any
/// `owner/<rest>` is >= `owner || '/'` and, since '/' (0x2F) is immediately
/// followed by '0' (0x30), < `owner || '0'`. `~>=~` / `~<~` are the
/// text_pattern_ops operators, which is what lets `idx_<kind>_owner_prefix`
/// answer the count with an index-only scan — the default opclass sorts by the
/// database collation and cannot serve a byte-prefix range.
fn owner_prefix_range(alias: &str) -> String {
    format!("{alias}.path ~>=~ (o.owner || '/') AND {alias}.path ~<~ (o.owner || '0')")
}

/// Per-owner runnable counts for the homepage tree, so every folder / user node
/// can show its size and the empty ones can be dropped without loading them.
///
/// Runs off the non-RLS pool and re-derives visibility from `path` alone: an
/// owner is readable whole or not at all (admin, folder in the caller's read
/// set, or the caller's own user space). That is what makes the count an
/// index-only prefix scan — RLS instead applies `split_part`/`current_setting`
/// predicates per row, which no index serves. The one case `path` cannot express
/// is an item shared individually out of an otherwise unreadable owner; those
/// owners are recovered by a second `extra_perms` pass over the GIN indexes.
///
/// The share pass's GIN indexes are on `extra_perms` alone, so on a multi-tenant
/// instance its bitmap matches grantee keys (`g/all` exists in every workspace)
/// across workspaces and `workspace_id` is only a recheck. Scoping the index
/// would need `btree_gin`, a contrib extension self-hosted installs can't be
/// assumed to have; the RLS policies already scan these indexes the same way.
async fn count_runnables_by_owner(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(q): Query<CountRunnablesQuery>,
) -> JsonResult<RunnableCountsResponse> {
    let kinds: Vec<&str> = match q.kinds.as_deref() {
        None | Some("") => vec!["script", "flow", "app"],
        // Deduplicated: every kind becomes its own count subquery, so a repeated
        // entry would both double that kind's count and multiply the scans.
        Some(csv) => {
            let mut ks: Vec<&str> = vec![];
            for k in csv.split(',').map(|s| s.trim()) {
                if ["script", "flow", "app"].contains(&k) && !ks.contains(&k) {
                    ks.push(k);
                }
            }
            ks
        }
    };
    if kinds.is_empty() {
        return Ok(Json(RunnableCountsResponse { counts: HashMap::new() }));
    }
    let with_libs = q.include_without_main.unwrap_or(false) && !authed.is_operator;

    // Per-kind predicates shared by both passes. `alias` is the branch's table
    // alias; `binds` collects the scope values, numbered from `base + 1`.
    let kind_filters =
        |kind: &str, alias: &str, base: usize, binds: &mut Vec<String>| -> Vec<String> {
            let mut w = vec![format!("{alias}.workspace_id = $1")];
            match kind {
                "script" => {
                    w.push(format!("{alias}.archived = false"));
                    // `pipeline` members are always excluded: the tree folds them into
                    // their folder's single "Pipeline" entry and never lists them as
                    // rows, so counting them would promise items that never appear.
                    // `lib` follows the caller's include_without_main, as in the listing.
                    let mut hidden = vec!["'pipeline'"];
                    if !with_libs {
                        hidden.push("'lib'");
                    }
                    w.push(format!(
                        "({alias}.auto_kind IS NULL OR {alias}.auto_kind NOT IN ({}))",
                        hidden.join(", ")
                    ));
                    if let Some(s) = scope_path_predicate(&authed, "scripts", alias, base, binds) {
                        w.push(s);
                    }
                }
                "flow" => {
                    w.push(format!("{alias}.archived = false"));
                    if let Some(s) = scope_path_predicate(&authed, "flows", alias, base, binds) {
                        w.push(s);
                    }
                }
                _ => {
                    if let Some(s) = scope_path_predicate(&authed, "apps", alias, base, binds) {
                        w.push(s);
                    }
                }
            }
            w
        };
    let table_of = |kind: &str| match kind {
        "script" => "script",
        "flow" => "flow",
        _ => "app",
    };

    // `SELECT owner, count(*) ... GROUP BY owner` over a set of per-kind path
    // selects — the shape both the admin sweep and the share pass end in.
    let grouped_by_owner = |branches: Vec<String>| -> String {
        format!(
            "SELECT split_part(p.path, '/', 1) || '/' || split_part(p.path, '/', 2) AS owner, \
                    count(*)::bigint AS count \
             FROM ({}) p GROUP BY 1",
            branches.join(" UNION ALL ")
        )
    };

    let mut counts: HashMap<String, i64> = HashMap::new();

    if authed.is_admin {
        // Admins read the whole workspace, so one grouped scan per kind is the
        // cheapest shape. Enumerating owners and prefix-scanning each instead
        // would cost one scan per folder AND per member, growing with headcount
        // rather than with content.
        // $1 = workspace.
        let mut binds: Vec<String> = vec![];
        let branches: Vec<String> = kinds
            .iter()
            .map(|kind| {
                let alias = "t";
                let w = kind_filters(kind, alias, 1, &mut binds);
                format!(
                    "SELECT {alias}.path FROM {} {alias} WHERE {}",
                    table_of(kind),
                    w.join(" AND ")
                )
            })
            .collect();
        let sql = grouped_by_owner(branches);
        let mut query = sqlx::query_as::<_, OwnerCount>(&sql).bind(&w_id);
        for b in &binds {
            query = query.bind(b);
        }
        for r in query.fetch_all(&db).await? {
            counts.insert(r.owner, r.count);
        }
        add_draft_counts(&authed, &db, &w_id, &kinds, with_libs, &q, &mut counts).await?;
        counts.retain(|_, c| *c > 0);
        return Ok(Json(RunnableCountsResponse { counts }));
    }

    // Pass 1 — prefix counts for the owners the caller reads wholesale: their
    // folders and their own user space, a set bounded by the grants they hold.
    // $1 = workspace, $2 = readable folder names, $3 = username.
    let mut binds: Vec<String> = vec![];
    let terms: Vec<String> = kinds
        .iter()
        .map(|kind| {
            let alias = "t";
            let mut w = kind_filters(kind, alias, 3, &mut binds);
            w.push(owner_prefix_range(alias));
            format!(
                "(SELECT count(*) FROM {} {alias} WHERE {})",
                table_of(kind),
                w.join(" AND ")
            )
        })
        .collect();
    let sql = format!(
        "WITH owners(owner) AS ( \
           SELECT 'f/' || name FROM folder WHERE workspace_id = $1 AND name = ANY($2) \
           UNION SELECT 'u/' || $3 \
         ) \
         SELECT o.owner AS owner, ({})::bigint AS count FROM owners o",
        terms.join(" + ")
    );
    let readable_folders: Vec<String> = authed.folders.iter().map(|f| f.0.clone()).collect();
    let mut query = sqlx::query_as::<_, OwnerCount>(&sql)
        .bind(&w_id)
        .bind(&readable_folders)
        .bind(&authed.username);
    for b in &binds {
        query = query.bind(b);
    }
    for r in query.fetch_all(&db).await? {
        counts.insert(r.owner, r.count);
    }

    // Pass 2 — owners the caller only reaches through an individual share.
    // $1 = workspace, $2 = the caller's grantee keys.
    let mut grantees = vec![format!("u/{}", authed.username)];
    grantees.extend(authed.groups.iter().map(|g| format!("g/{}", g)));
    let mut binds: Vec<String> = vec![];
    let branches: Vec<String> = kinds
        .iter()
        .map(|kind| {
            let alias = "t";
            let mut w = kind_filters(kind, alias, 2, &mut binds);
            w.push(format!("{alias}.extra_perms ?| $2"));
            format!(
                "SELECT {alias}.path FROM {} {alias} WHERE {}",
                table_of(kind),
                w.join(" AND ")
            )
        })
        .collect();
    let sql = grouped_by_owner(branches);
    let mut query = sqlx::query_as::<_, OwnerCount>(&sql)
        .bind(&w_id)
        .bind(&grantees);
    for b in &binds {
        query = query.bind(b);
    }
    for r in query.fetch_all(&db).await? {
        // Owners already in `counts` were counted whole in pass 1, shares
        // included — only the ones missing there are added here.
        counts.entry(r.owner).or_insert(r.count);
    }

    add_draft_counts(&authed, &db, &w_id, &kinds, with_libs, &q, &mut counts).await?;
    counts.retain(|_, c| *c > 0);
    Ok(Json(RunnableCountsResponse { counts }))
}

/// Adds the caller's draft-only rows to `counts`, in the same shape `/list`
/// returns them so a badge never disagrees with the rows behind it.
///
/// Not restricted to the readable owners the deployed passes walk: a draft is
/// the caller's own and `/list` reads it off the non-RLS `draft` table, so
/// scoping it to folder grants would hide a count for a row that still lists.
/// Deployed and draft rows can't overlap (the anti-join is what makes a draft
/// "draft-only"), so the two counts add.
async fn add_draft_counts(
    authed: &ApiAuthed,
    db: &DB,
    w_id: &str,
    kinds: &[&str],
    with_libs: bool,
    q: &CountRunnablesQuery,
    counts: &mut HashMap<String, i64>,
) -> Result<(), Error> {
    if !q.include_draft_only.unwrap_or(false) || authed.is_operator {
        return Ok(());
    }
    // $1 = workspace, $2 = the caller's email.
    let mut binds: Vec<String> = vec![];
    let branches: Vec<String> = kinds
        .iter()
        .map(|kind| {
            let (typ_pred, deployed, domain) = match *kind {
                "script" => ("d.typ = 'script'", "script", "scripts"),
                "flow" => ("d.typ = 'flow'", "flow", "flows"),
                _ => ("d.typ IN ('app', 'raw_app')", "app", "apps"),
            };
            // The owner a draft counts under is where it says it will live, not the
            // `u/<caller>/draft_<uuid>` it is parked at — same path `/list` groups and
            // filters on. Scripts round-trip the typed path through the draft JSON's
            // own `path`; flows and apps write `draft_path`. See scripts.rs.
            let typed_path = if *kind == "script" { "path" } else { "draft_path" };
            let effective_path =
                format!("COALESCE(NULLIF(d.value->>'{typed_path}', ''), d.path) as path");
            let mut w = vec![
                "d.workspace_id = $1".to_string(),
                typ_pred.to_string(),
                "(d.email = $2 OR d.email IS NULL)".to_string(),
                format!(
                    "NOT EXISTS (SELECT 1 FROM {deployed} x \
                     WHERE x.workspace_id = d.workspace_id AND x.path = d.path)"
                ),
            ];
            if *kind == "script" {
                // Same rule as the deployed count, over the `auto_kind` the editor
                // stamped into the draft: a pipeline member is folded into its
                // pipeline entry rather than listed, and `lib` follows the caller.
                let mut hidden = vec!["'pipeline'"];
                if !with_libs {
                    hidden.push("'lib'");
                }
                w.push(format!(
                    "(d.value->>'auto_kind' IS NULL OR d.value->>'auto_kind' NOT IN ({}))",
                    hidden.join(", ")
                ));
            }
            if let Some(s) = scope_path_predicate(authed, domain, "d", 2, &mut binds) {
                w.push(s);
            }
            // DISTINCT ON collapses a path holding both the caller's draft and a
            // legacy NULL-email one, which `/list` shows as a single row. Wrapped
            // because its ORDER BY would otherwise bind to the whole UNION.
            format!(
                "SELECT path FROM (SELECT DISTINCT ON (d.path) {effective_path} FROM draft d WHERE {} ORDER BY d.path, (d.email IS NULL), d.created_at DESC) s",
                w.join(" AND ")
            )
        })
        .collect();
    let sql = format!(
        "SELECT split_part(p.path, '/', 1) || '/' || split_part(p.path, '/', 2) AS owner, \
                count(*)::bigint AS count \
         FROM ({}) p GROUP BY 1",
        branches.join(" UNION ALL ")
    );
    let mut query = sqlx::query_as::<_, OwnerCount>(&sql)
        .bind(w_id)
        .bind(&authed.email);
    for b in &binds {
        query = query.bind(b);
    }
    for r in query.fetch_all(db).await? {
        *counts.entry(r.owner).or_insert(0) += r.count;
    }
    Ok(())
}
