/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Read side of the metric catalog: what a DuckLake table declares (for the
//! script editor drawer) and what is declared under a path prefix (for agents).

use axum::{
    extract::{Path, Query},
    routing::get,
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use windmill_api_auth::{build_scope_path_filter, ApiAuthed, ScopePathFilter};
use windmill_common::{
    data_metrics::{canonical_table_path, MetricEntry},
    db::UserDB,
    error::{Error, JsonResult},
};

pub fn workspaced_service() -> Router {
    Router::new().route("/list", get(list_metrics))
}

#[derive(Deserialize)]
struct ListQuery {
    /// `<lake>/<table>`, with or without the `ducklake://` scheme.
    table: Option<String>,
    /// Path prefix to scope to, e.g. `f/analytics`.
    path_prefix: Option<String>,
    /// Results per page, clamped to `MAX_PER_PAGE`. Defaults to `MAX_PER_PAGE`.
    per_page: Option<i64>,
    /// Keyset cursor: echo the previous response's `next_cursor` fields. All four
    /// move together (they are the sort key); omit them for the first page.
    cursor_table: Option<String>,
    cursor_kind: Option<String>,
    cursor_name: Option<String>,
    cursor_script: Option<String>,
}

/// Position on the (table_path, kind, name, script_path) sort key. Only ever built
/// from a row the caller received, so it never reveals a hidden row.
#[derive(Serialize)]
struct MetricCursor {
    table_path: String,
    kind: String,
    name: String,
    script_path: String,
}

/// One page. `next_cursor` present means more rows may follow (pass it back);
/// absent means the catalog is exhausted.
#[derive(Serialize)]
struct MetricPage {
    metrics: Vec<MetricEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    next_cursor: Option<MetricCursor>,
}

/// Builds the descendant pattern for a path prefix, anchored to the `/` boundary
/// so `f/analytics` does not also match `f/analytics2`. LIKE wildcards in the
/// caller's own text are escaped, since an unescaped `%` or `_` would silently
/// widen the scope past the path they asked for.
fn descendants_of(prefix: &str) -> String {
    let escaped = prefix
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("{escaped}/%")
}

/// Largest page a caller may request; the caller pages with the keyset cursor.
const MAX_PER_PAGE: i64 = 1000;

/// Lists declared measures and dimensions, optionally narrowed to one table or to
/// a producing script path prefix.
async fn list_metrics(
    authed: ApiAuthed,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(q): Query<ListQuery>,
) -> JsonResult<MetricPage> {
    let table = q
        .table
        .as_deref()
        .filter(|t| !t.is_empty())
        .map(canonical_table_path);
    // A prefix matches the path itself and everything beneath it, but not a
    // sibling folder that merely starts with the same characters.
    let prefix = q
        .path_prefix
        .as_deref()
        .map(|p| p.trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .map(str::to_string);
    let prefix_descendants = prefix.as_deref().map(descendants_of);

    // The four cursor components are one value; a partial cursor would compare
    // against a tuple with a NULL (matching nothing) or silently restart. Reject it.
    let cursor_present = [
        &q.cursor_table,
        &q.cursor_kind,
        &q.cursor_name,
        &q.cursor_script,
    ]
    .iter()
    .filter(|c| c.is_some())
    .count();
    if cursor_present != 0 && cursor_present != 4 {
        return Err(Error::BadRequest(
            "cursor_table, cursor_kind, cursor_name and cursor_script must be supplied together"
                .to_string(),
        ));
    }

    // RLS reflects the user's own permissions but says nothing about a token's path
    // scopes, so a scoped token must be filtered separately. This route has its own
    // `data_metrics` scope domain (not an alias of `scripts`, which would let a
    // metrics token reach every /scripts route). The grant is anchored on the
    // producing script's path, as with `list_macros`. It is pushed INTO the query,
    // not applied after the fetch: with scope filtering in SQL every returned row is
    // authorized, so the keyset cursor (the last returned row) can never name a row
    // the caller cannot see, and a post-fetch filter would instead let the page size
    // reveal the count of out-of-scope declarations under a table/prefix.
    let (scope_all, scope_exact, scope_prefix) =
        match build_scope_path_filter(&authed, "data_metrics", "read") {
            ScopePathFilter::AllowAll => (true, Vec::new(), Vec::new()),
            ScopePathFilter::Restricted { exact, prefix } => (false, exact, prefix),
        };

    let per_page = q.per_page.unwrap_or(MAX_PER_PAGE).clamp(1, MAX_PER_PAGE);

    let mut tx = user_db.begin(&authed).await?;
    // Keyset paging: continue strictly after the previous page's last row on the
    // (table_path, kind, name, script_path) sort key, so total work over the whole
    // catalog is linear (offset paging re-reads every prior page). One extra row is
    // fetched to learn whether more remain without a second query.
    //
    // The EXISTS runs under `script`'s RLS on this authed connection, so a caller
    // only sees declarations from producers they can read. `data_metric` itself has
    // no RLS, so removing this predicate would expose every workspace metric.
    // Archived/deleted versions are excluded, else a renamed producer keeps serving
    // its old path's declarations. The token scope filter ($10 allow-all, else the
    // path is a $11 exact grant or sits at/under a $12 `prefix/*` grant on the `/`
    // boundary) mirrors `ScopePathFilter::allows`. The ORDER BY is a total order
    // (script_path breaks ties on table/kind/name), so keyset windows can't skip or
    // duplicate a row.
    let mut rows = sqlx::query_as!(
        MetricEntry,
        "SELECT script_path, table_path, kind, name, expr, filter \
         FROM data_metric dm \
         WHERE dm.workspace_id = $1 \
           AND ($2::text IS NULL OR dm.table_path = $2) \
           AND ($3::text IS NULL OR dm.script_path = $3 OR dm.script_path LIKE $4) \
           AND ($6::text IS NULL OR \
                (dm.table_path, dm.kind, dm.name, dm.script_path) > ($6, $7, $8, $9)) \
           AND ( $10 \
                 OR dm.script_path = ANY($11) \
                 OR EXISTS ( SELECT 1 FROM unnest($12::text[]) AS pfx \
                             WHERE dm.script_path = pfx \
                                OR left(dm.script_path, length(pfx) + 1) = pfx || '/' ) ) \
           AND EXISTS ( \
             SELECT 1 FROM script s \
             WHERE s.workspace_id = dm.workspace_id AND s.path = dm.script_path \
               AND s.archived = false AND s.deleted = false \
           ) \
         ORDER BY dm.table_path, dm.kind, dm.name, dm.script_path \
         LIMIT $5",
        &w_id,
        table.as_deref(),
        prefix.as_deref(),
        prefix_descendants.as_deref(),
        per_page + 1,
        q.cursor_table.as_deref(),
        q.cursor_kind.as_deref(),
        q.cursor_name.as_deref(),
        q.cursor_script.as_deref(),
        scope_all,
        &scope_exact[..],
        &scope_prefix[..],
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;

    // The probe row proves more remain; drop it and hand back its predecessor as the
    // cursor. Without it the page fit entirely, so there is no next page.
    let next_cursor = if rows.len() as i64 > per_page {
        rows.truncate(per_page as usize);
        rows.last().map(|last| MetricCursor {
            table_path: last.table_path.clone(),
            kind: last.kind.clone(),
            name: last.name.clone(),
            script_path: last.script_path.clone(),
        })
    } else {
        None
    };
    Ok(Json(MetricPage { metrics: rows, next_cursor }))
}
