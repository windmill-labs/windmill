use std::collections::HashSet;

use sqlx::{PgExecutor, Postgres, Transaction};

use crate::{error, scripts::ScriptHash};

pub use windmill_parser::asset_parser::{
    merge_column_lineage, parse_pipeline_annotations, ColumnLineage, ColumnRef, DataTest,
    OnSchemaChange, PartitionKind, PipelineAnnotations, RetrySpec, TriggerSpec, PARTITION_TOKEN,
};
pub use windmill_types::assets::*;

// --- Workspace DuckDB macro registry cache (worker hot path) ---
// Every DuckDB job reads the `macro_definition` registry; cascades can run
// many jobs per second. Primary invalidation is the
// `notify_macro_registry_change` event, emitted transactionally with every
// registry mutation and dispatched to this cache in main.rs — the TTL is a
// backstop for mutation paths that don't emit (manual SQL edits).

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct MacroRegistryEntry {
    pub name: String,
    pub params: String,
    pub body: String,
    pub is_table_macro: bool,
    pub provider_path: String,
}

#[derive(Clone)]
pub struct ExpiringMacroRegistry {
    pub rows: std::sync::Arc<Vec<MacroRegistryEntry>>,
    pub expires_at: std::time::Instant,
}

pub const MACRO_REGISTRY_TTL: std::time::Duration = std::time::Duration::from_secs(60);

// Tests sharing one process across several databases must disable the cache:
// it is keyed by workspace id alone, and a notify from one DB can't evict
// entries populated from another (same hazard as DEPLOYED_SCRIPT_CACHE_DISABLED).
pub static MACRO_REGISTRY_CACHE_DISABLED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

lazy_static::lazy_static! {
    pub static ref MACRO_REGISTRY_CACHE: quick_cache::sync::Cache<String, ExpiringMacroRegistry> =
        quick_cache::sync::Cache::new(1000);
}

#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq)]
#[sqlx(type_name = "SCRIPT_TRIGGER_KIND", rename_all = "lowercase")]
pub enum ScriptTriggerKind {
    Asset,
    Schedule,
    Webhook,
    Email,
    Kafka,
    Mqtt,
    Nats,
    Postgres,
    Sqs,
    Gcp,
}

pub async fn insert_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    asset: &AssetWithAltAccessType,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    // Convert columns BTreeMap to JSONB format
    let columns_json = asset
        .columns
        .as_ref()
        .map(|cols| serde_json::to_value(cols).unwrap_or(serde_json::Value::Null));

    // Invalidate the per-workspace producer-writes cache only when this insert
    // actually adds a write producer: the cache (asset_dispatch::
    // ASSET_PRODUCER_WRITES_CACHE) tracks script rows with 'w'/'rw' access, so
    // a row that was a no-op (ON CONFLICT skipped), a flow usage, or read-only
    // can't change it. Emitting in the same statement keeps the notify atomic
    // with the insert and visible to pollers only on commit. See the matching
    // delete-side guard in clear_static_asset_usage.
    sqlx::query!(
        r#"WITH ins AS (
             INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)
                VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING
             RETURNING usage_kind, usage_access_type
           )
           INSERT INTO notify_event (channel, payload)
           SELECT 'notify_asset_producer_change', $1
           FROM ins WHERE usage_kind = 'script' AND usage_access_type IN ('w', 'rw')"#,
        workspace_id,
        asset.path,
        asset.kind as AssetKind,
        (asset.access_type.or(asset.alt_access_type)) as Option<AssetUsageAccessType>,
        usage_path,
        usage_kind as AssetUsageKind,
        columns_json as Option<serde_json::Value>
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn clear_static_asset_usage<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    usage_path: &str,
    usage_kind: AssetUsageKind,
) -> error::Result<()> {
    // Invalidate the per-workspace producer-writes cache that gates the
    // asset-trigger dispatch hook (windmill-queue
    // asset_dispatch::ASSET_PRODUCER_WRITES_CACHE). That cache only tracks
    // script rows with 'w'/'rw' access, so emit the notify only when the
    // delete actually removed such a write producer: flow usage, read-only
    // usage, and deletes that matched no producer row leave the cache
    // unchanged (the common case — most deploys touch no write asset). The
    // matching add side lives in insert_static_asset_usage. Emitting in the
    // same statement keeps the notify atomic with the delete and visible to
    // pollers only on commit.
    sqlx::query!(
        r#"WITH del AS (
             DELETE FROM asset WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = $3
             RETURNING usage_access_type
           )
           INSERT INTO notify_event (channel, payload)
           SELECT 'notify_asset_producer_change', $1
           WHERE $3 = 'script'
             AND EXISTS (SELECT 1 FROM del WHERE usage_access_type IN ('w', 'rw'))"#,
        workspace_id,
        usage_path,
        usage_kind as AssetUsageKind
    )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn clear_static_asset_usage_by_script_hash<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    script_hash: ScriptHash,
) -> error::Result<()> {
    // Always script usage → invalidate the producer-writes cache for this
    // workspace, but only when the delete actually removed a write producer
    // ('w'/'rw'); see clear_static_asset_usage. Atomic with the delete.
    sqlx::query!(
        r#"WITH del AS (
             DELETE FROM asset WHERE workspace_id = $1 AND usage_kind = 'script'
               AND usage_path = (SELECT path FROM script WHERE hash = $2 AND workspace_id = $1)
             RETURNING usage_access_type
           )
           INSERT INTO notify_event (channel, payload)
           SELECT 'notify_asset_producer_change', $1
           WHERE EXISTS (SELECT 1 FROM del WHERE usage_access_type IN ('w', 'rw'))"#,
        workspace_id,
        script_hash.0
    )
    .execute(executor)
    .await?;
    Ok(())
}

fn is_write_access(access: Option<AssetUsageAccessType>) -> bool {
    matches!(
        access,
        Some(AssetUsageAccessType::W) | Some(AssetUsageAccessType::RW)
    )
}

/// Kinds whose *read* usage auto-derives a cascade trigger edge inside a
/// `// pipeline`. Scoped to the two intra-pipeline data kinds — a ducklake
/// table read (the core case) and an s3 object read (file-ingestion
/// producers). Resource / datatable / volume reads stay explicit-`// on`:
/// a config/lookup read cascading is more often surprising than wanted.
fn is_auto_trigger_kind(kind: AssetKind) -> bool {
    matches!(kind, AssetKind::Ducklake | AssetKind::S3Object)
}

/// Trigger refs auto-derived from a pipeline script's inferred reads, so the
/// FROM clause alone wires the cascade edge (no redundant `// on <asset>`).
///
/// Included: an input read read-*only* (`R`) of a supported kind
/// ([`is_auto_trigger_kind`]). The effective access type is
/// `access_type.or(alt_access_type)` — same precedence as the persisted
/// `asset.usage_access_type` and the frontend mirror's `access_type ??
/// alt_access_type`, so a manual read override on an ambiguous parse still
/// derives an edge (and the live canvas and the deployed graph agree).
/// Excluded, each for a reason:
/// - `RW` / `W` — the script also writes the asset; an edge would be a
///   self-triggering loop.
/// - `None` access — usage is ambiguous (poisoned merge) with no override;
///   can't confirm a read, so fail safe and don't cascade.
/// - already in `explicit_refs` — the author wrote `// on <asset>`, which
///   wins (it carries the per-edge debounce/opts).
/// - in `muted_refs` — a `// mute <asset>` opt-out (lookup / SCD input).
///
/// `mute_all` (from `// mute all`) short-circuits to no derivation, leaving
/// only the explicit `// on` edges. Returns canonical refs (e.g.
/// `ducklake://main.orders`), deduped, in input order.
pub fn derive_pipeline_asset_trigger_refs(
    assets: &[AssetWithAltAccessType],
    explicit_refs: &HashSet<String>,
    muted_refs: &HashSet<String>,
    mute_all: bool,
) -> Vec<String> {
    if mute_all {
        return vec![];
    }
    let mut out = vec![];
    let mut seen = HashSet::new();
    for a in assets {
        // Effective access mirrors the persisted `usage_access_type` and the
        // frontend derivation: an explicit parse wins, else the manual override.
        let access = a.access_type.or(a.alt_access_type);
        if access != Some(AssetUsageAccessType::R) || !is_auto_trigger_kind(a.kind) {
            continue;
        }
        let Some(prefix) = a.kind.canonical_prefix() else {
            continue;
        };
        let r = format!("{}{}", prefix, a.path);
        if explicit_refs.contains(&r) || muted_refs.contains(&r) {
            continue;
        }
        if seen.insert(r.clone()) {
            out.push(r);
        }
    }
    out
}

/// Clear and reinsert the full static-asset usage set of a script in one tx,
/// invalidating the producer-writes cache at most once and only on a real
/// change. The cache (asset_dispatch::ASSET_PRODUCER_WRITES_CACHE) keys a
/// workspace by the set of (kind, path) script rows with 'w'/'rw' access, so a
/// redeploy that keeps the same write producers must leave it untouched. We
/// diff the old write set (captured from the clearing DELETE's RETURNING)
/// against the new one and emit a single notify only when they differ —
/// emitting per statement, as clear/insert_static_asset_usage do, would fire
/// twice on every write-asset redeploy (the clear removes the row, the reinsert
/// adds it back). The notify rides the deploy tx, so it stays atomic with the
/// DML and visible to pollers only on commit.
///
/// DELETE and INSERT of the same primary key cannot share a single statement
/// (both would read the pre-statement snapshot, so the reinsert's ON CONFLICT
/// would silently drop the row), which is why this clears and reinserts as
/// separate statements rather than one CTE.
///
/// Performs no authorization itself: `tx` must already be scoped to a caller
/// authorized for `workspace_id`/`usage_path` (e.g. `user_db.begin(&authed)`,
/// which applies RLS), exactly like the sibling clear/insert helpers.
pub async fn replace_static_asset_usage(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    usage_path: &str,
    assets: &[AssetWithAltAccessType],
) -> error::Result<()> {
    let cleared = sqlx::query!(
        r#"DELETE FROM asset
           WHERE workspace_id = $1 AND usage_path = $2 AND usage_kind = 'script'
           RETURNING kind AS "kind!: AssetKind", path,
                     usage_access_type AS "usage_access_type: AssetUsageAccessType""#,
        workspace_id,
        usage_path,
    )
    .fetch_all(&mut **tx)
    .await?;
    let old_writes: HashSet<(AssetKind, String)> = cleared
        .into_iter()
        .filter(|r| is_write_access(r.usage_access_type))
        .map(|r| (r.kind, r.path))
        .collect();

    // Build new_writes from rows actually inserted (RETURNING), not the
    // requested slice: ON CONFLICT DO NOTHING is first-writer-wins, so a payload
    // with duplicate (kind, path) entries at conflicting access types persists
    // only the first. Since the DELETE above emptied this usage_path, every
    // non-conflicting insert lands, so the inserted rows are exactly the new
    // persisted set.
    let mut new_writes: HashSet<(AssetKind, String)> = HashSet::new();
    for asset in assets {
        let access = asset.access_type.or(asset.alt_access_type);
        let columns_json = asset
            .columns
            .as_ref()
            .map(|cols| serde_json::to_value(cols).unwrap_or(serde_json::Value::Null));
        let inserted = sqlx::query!(
            r#"INSERT INTO asset (workspace_id, path, kind, usage_access_type, usage_path, usage_kind, columns)
               VALUES ($1, $2, $3, $4, $5, 'script', $6) ON CONFLICT DO NOTHING
               RETURNING usage_access_type AS "usage_access_type: AssetUsageAccessType""#,
            workspace_id,
            asset.path,
            asset.kind as AssetKind,
            access as Option<AssetUsageAccessType>,
            usage_path,
            columns_json as Option<serde_json::Value>,
        )
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(row) = inserted {
            if is_write_access(row.usage_access_type) {
                new_writes.insert((asset.kind, asset.path.clone()));
            }
        }
    }

    if old_writes != new_writes {
        sqlx::query!(
            r#"INSERT INTO notify_event (channel, payload)
               VALUES ('notify_asset_producer_change', $1)"#,
            workspace_id,
        )
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

// Wipe all pipeline trigger declarations held by the given runnable. Used at
// deploy time: redeploying a script wipes its prior `// on` annotations so
// removing them implicitly un-declares those edges.
pub async fn clear_script_triggers<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_path: &str,
    runnable_kind: AssetUsageKind,
) -> error::Result<()> {
    sqlx::query!(
        r#"DELETE FROM script_trigger
           WHERE workspace_id = $1 AND runnable_kind = $2 AND runnable_path = $3"#,
        workspace_id,
        runnable_kind as AssetUsageKind,
        runnable_path,
    )
    .execute(executor)
    .await?;
    Ok(())
}

// Insert a single trigger declaration. Caller is expected to wipe first.
// `join_all` is the script-level `// trigger all` flag (AND join barrier);
// `retry_count` / `retry_delay_s` are the `// retry <n> [<delay>]` policy.
// All three are script-level — the same value for every row of a given
// runnable — but stored per-row to keep the wipe-and-reinsert pattern and a
// single-query subscriber lookup.
pub async fn insert_script_trigger<'e>(
    executor: impl PgExecutor<'e>,
    workspace_id: &str,
    runnable_kind: AssetUsageKind,
    runnable_path: &str,
    trigger_kind: ScriptTriggerKind,
    trigger_ref: &str,
    join_all: bool,
    debounce_s: Option<i32>,
    retry_count: Option<i16>,
    retry_delay_s: Option<i32>,
) -> error::Result<()> {
    sqlx::query!(
        r#"INSERT INTO script_trigger
             (workspace_id, runnable_kind, runnable_path, trigger_kind, trigger_ref, join_all,
              debounce_s, retry_count, retry_delay_s)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        workspace_id,
        runnable_kind as AssetUsageKind,
        runnable_path,
        trigger_kind as ScriptTriggerKind,
        trigger_ref,
        join_all,
        debounce_s,
        retry_count,
        retry_delay_s,
    )
    .execute(executor)
    .await?;
    Ok(())
}

/// Parse a debounce duration into whole seconds. Accepts a bare integer
/// (seconds) or an `<n>` with an `s`/`m`/`h`/`d` suffix (e.g. `30s`,
/// `5m`, `2h`, `1d`). Returns `None` for empty / malformed / non-positive
/// input — the caller treats `None` as "no debounce" (fan-out), so a typo
/// fails safe rather than silently debouncing.
pub fn parse_duration_secs(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let (num, mult): (&str, i64) = match s.as_bytes().last() {
        Some(b's') => (&s[..s.len() - 1], 1),
        Some(b'm') => (&s[..s.len() - 1], 60),
        Some(b'h') => (&s[..s.len() - 1], 3600),
        Some(b'd') => (&s[..s.len() - 1], 86400),
        Some(c) if c.is_ascii_digit() => (s, 1),
        _ => return None,
    };
    let n: i64 = num.trim().parse().ok()?;
    let secs = n.checked_mul(mult)?;
    if secs <= 0 || secs > i32::MAX as i64 {
        return None;
    }
    Some(secs as i32)
}

#[cfg(test)]
mod debounce_duration_tests {
    use super::parse_duration_secs;

    #[test]
    fn parses_units_and_bare_seconds() {
        assert_eq!(parse_duration_secs("60"), Some(60));
        assert_eq!(parse_duration_secs("30s"), Some(30));
        assert_eq!(parse_duration_secs("5m"), Some(300));
        assert_eq!(parse_duration_secs("2h"), Some(7200));
        assert_eq!(parse_duration_secs(" 1d "), Some(86400));
        // Explicit plus sign comes free with i64 parsing; the TS mirror
        // (parseDurationSecs) matches it — keep the two in lockstep.
        assert_eq!(parse_duration_secs("+5m"), Some(300));
        assert_eq!(parse_duration_secs("+45"), Some(45));
    }

    #[test]
    fn rejects_garbage_and_nonpositive() {
        assert_eq!(parse_duration_secs(""), None);
        assert_eq!(parse_duration_secs("abc"), None);
        assert_eq!(parse_duration_secs("0"), None);
        assert_eq!(parse_duration_secs("-5"), None);
        assert_eq!(parse_duration_secs("10x"), None);
        assert_eq!(parse_duration_secs("s"), None);
    }
}

#[cfg(test)]
mod derive_trigger_tests {
    use super::{derive_pipeline_asset_trigger_refs, AssetKind, AssetUsageAccessType};
    use std::collections::HashSet;
    use windmill_types::assets::AssetWithAltAccessType;

    fn asset(
        kind: AssetKind,
        path: &str,
        at: Option<AssetUsageAccessType>,
    ) -> AssetWithAltAccessType {
        AssetWithAltAccessType {
            path: path.to_string(),
            kind,
            access_type: at,
            alt_access_type: None,
            columns: None,
        }
    }

    fn derive(assets: &[AssetWithAltAccessType]) -> Vec<String> {
        derive_pipeline_asset_trigger_refs(assets, &HashSet::new(), &HashSet::new(), false)
    }

    #[test]
    fn read_only_ducklake_and_s3_derive_an_edge() {
        use AssetUsageAccessType::R;
        let a = [
            asset(AssetKind::Ducklake, "main.orders", Some(R)),
            asset(AssetKind::S3Object, "raw/events", Some(R)),
        ];
        assert_eq!(
            derive(&a),
            vec![
                "ducklake://main.orders".to_string(),
                "s3://raw/events".to_string()
            ]
        );
    }

    #[test]
    fn writes_and_rw_are_skipped_to_avoid_self_edges() {
        use AssetUsageAccessType::*;
        // W (pure producer) and RW (reads *and* writes the same table — a
        // self-cascade if edged) both derive nothing.
        let a = [
            asset(AssetKind::Ducklake, "main.out", Some(W)),
            asset(AssetKind::Ducklake, "main.self", Some(RW)),
        ];
        assert!(derive(&a).is_empty());
    }

    #[test]
    fn ambiguous_access_and_unsupported_kinds_are_skipped() {
        use AssetUsageAccessType::R;
        let a = [
            asset(AssetKind::Ducklake, "main.ambiguous", None), // poisoned merge
            asset(AssetKind::Resource, "f/db", Some(R)),        // out of scope
            asset(AssetKind::DataTable, "main.dt", Some(R)),    // out of scope
        ];
        assert!(derive(&a).is_empty());
    }

    #[test]
    fn manual_read_override_on_ambiguous_parse_derives_an_edge() {
        use AssetUsageAccessType::{R, W};
        // Parser can't confirm access (`access_type: None`) but the user manually
        // overrode it. Effective access = `access_type.or(alt_access_type)`, the
        // same value persisted to `asset.usage_access_type` and used by the
        // frontend canvas — so a read override derives an edge (parity, no
        // silently-vanishing edge on deploy) and a write override does not.
        let read_override = AssetWithAltAccessType {
            path: "main.override_r".to_string(),
            kind: AssetKind::Ducklake,
            access_type: None,
            alt_access_type: Some(R),
            columns: None,
        };
        let write_override = AssetWithAltAccessType {
            path: "main.override_w".to_string(),
            kind: AssetKind::Ducklake,
            access_type: None,
            alt_access_type: Some(W),
            columns: None,
        };
        assert_eq!(
            derive(&[read_override, write_override]),
            vec!["ducklake://main.override_r".to_string()]
        );
    }

    #[test]
    fn explicit_and_muted_refs_are_excluded() {
        use AssetUsageAccessType::R;
        let a = [
            asset(AssetKind::Ducklake, "main.explicit", Some(R)),
            asset(AssetKind::Ducklake, "main.muted", Some(R)),
            asset(AssetKind::Ducklake, "main.keep", Some(R)),
        ];
        let explicit: HashSet<String> = ["ducklake://main.explicit".to_string()].into();
        let muted: HashSet<String> = ["ducklake://main.muted".to_string()].into();
        assert_eq!(
            derive_pipeline_asset_trigger_refs(&a, &explicit, &muted, false),
            vec!["ducklake://main.keep".to_string()]
        );
    }

    #[test]
    fn mute_all_derives_nothing() {
        use AssetUsageAccessType::R;
        let a = [asset(AssetKind::Ducklake, "main.orders", Some(R))];
        assert!(
            derive_pipeline_asset_trigger_refs(&a, &HashSet::new(), &HashSet::new(), true)
                .is_empty()
        );
    }

    #[test]
    fn duplicate_reads_dedup() {
        use AssetUsageAccessType::R;
        let a = [
            asset(AssetKind::Ducklake, "main.orders", Some(R)),
            asset(AssetKind::Ducklake, "main.orders", Some(R)),
        ];
        assert_eq!(derive(&a), vec!["ducklake://main.orders".to_string()]);
    }
}

#[cfg(test)]
mod trigger_ref_roundtrip_tests {
    use super::{parse_asset_trigger_ref, trigger_spec_to_row, AssetKind, ScriptTriggerKind};
    use windmill_parser::asset_parser::{parse_asset_syntax, AssetKind as PAssetKind, TriggerSpec};

    // `trigger_spec_to_row` rebuilds a stored ref as `s3://<path>`, and
    // `parse_asset_trigger_ref` parses it back. The two must be inverse for
    // every S3 URI form, or a consumer's `// on` trigger lands on a different
    // graph node than the producer's inferred write. `parse_asset_syntax`
    // keeps the URI suffix verbatim (a default-storage path starts with `/`),
    // so the naive `prefix + path` rebuild round-trips for every form.
    fn roundtrip(uri: &str) -> String {
        let (pkind, path) = parse_asset_syntax(uri, false).expect("parse uri");
        assert_eq!(pkind, PAssetKind::S3Object);
        let spec = TriggerSpec::Asset { asset_kind: pkind, path: path.to_string(), debounce: None };
        let (kind, stored) = trigger_spec_to_row(&spec).expect("to row");
        assert_eq!(kind, ScriptTriggerKind::Asset);
        let (rkind, rpath) = parse_asset_trigger_ref(&stored).expect("parse ref");
        assert_eq!(rkind, AssetKind::S3Object);
        // The producer path and the round-tripped consumer path must match.
        assert_eq!(
            rpath, path,
            "round-trip diverged for {uri} (stored {stored})"
        );
        rpath
    }

    #[test]
    fn s3_trigger_ref_roundtrips_for_every_uri_form() {
        assert_eq!(roundtrip("s3:///exports/x"), "/exports/x"); // SDK default storage
        assert_eq!(roundtrip("s3://exports/x"), "exports/x"); // named storage `exports`
        assert_eq!(roundtrip("s3://mybucket/exports/x"), "mybucket/exports/x"); // explicit
        assert_eq!(roundtrip("s3:///y=2024/f.parquet"), "/y=2024/f.parquet"); // Hive
    }
}

// Inverse of trigger_spec_to_row for the Asset variant: parses a stored
// trigger_ref (e.g. `s3://foo`, `$res:bar`) back into the (kind, path) pair
// used as a graph node id. Returns None for refs that don't match any known
// asset prefix — callers should skip those edges.
pub fn parse_asset_trigger_ref(s: &str) -> Option<(AssetKind, String)> {
    let (kind, path) = windmill_parser::asset_parser::parse_asset_syntax(s, false)?;
    Some((asset_kind_from_parser(kind), path.to_string()))
}

// Convert a parser TriggerSpec into the `(kind, ref)` pair stored in
// script_trigger. Asset refs get their canonical prefix back so the
// trigger_ref matches what downstream lookups expect.
//
// Returns `None` for native trigger kinds (Kafka, Mqtt, Postgres, …) —
// those annotations are marker-only and don't produce a `script_trigger`
// row. The actual binding lives on the trigger row's own `script_path`
// column; the graph endpoint looks it up directly per kind.
pub fn trigger_spec_to_row(spec: &TriggerSpec) -> Option<(ScriptTriggerKind, String)> {
    match spec {
        TriggerSpec::Asset { asset_kind, path, .. } => {
            // Single source of truth for the canonical prefix lives on the
            // common AssetKind; map the parser kind across first. The parser
            // enum has no Variable variant, so canonical_prefix is always Some.
            let prefix = asset_kind_from_parser(*asset_kind).canonical_prefix()?;
            Some((ScriptTriggerKind::Asset, format!("{}{}", prefix, path)))
        }
        // Schedule joins the native-trigger family — no script_trigger row
        // is inserted for the annotation. The binding lives on the schedule
        // row's own `script_path` field, same as kafka/mqtt/etc.
        TriggerSpec::Schedule
        | TriggerSpec::Webhook
        | TriggerSpec::Email
        | TriggerSpec::Kafka
        | TriggerSpec::Mqtt
        | TriggerSpec::Amqp
        | TriggerSpec::Nats
        | TriggerSpec::Postgres
        | TriggerSpec::Sqs
        | TriggerSpec::Gcp
        // data_upload is a UI-first marker — no event source, no trigger row.
        // The script's S3Object input + auto-generated S3 picker drive it.
        | TriggerSpec::DataUpload => None,
    }
}

pub fn asset_kind_from_parser(parser_kind: windmill_parser::asset_parser::AssetKind) -> AssetKind {
    match parser_kind {
        windmill_parser::asset_parser::AssetKind::S3Object => AssetKind::S3Object,
        windmill_parser::asset_parser::AssetKind::Resource => AssetKind::Resource,
        windmill_parser::asset_parser::AssetKind::Ducklake => AssetKind::Ducklake,
        windmill_parser::asset_parser::AssetKind::DataTable => AssetKind::DataTable,
        windmill_parser::asset_parser::AssetKind::Volume => AssetKind::Volume,
    }
}

pub fn asset_access_type_from_parser(
    parser_kind: windmill_parser::asset_parser::AssetUsageAccessType,
) -> AssetUsageAccessType {
    match parser_kind {
        windmill_parser::asset_parser::AssetUsageAccessType::R => AssetUsageAccessType::R,
        windmill_parser::asset_parser::AssetUsageAccessType::W => AssetUsageAccessType::W,
        windmill_parser::asset_parser::AssetUsageAccessType::RW => AssetUsageAccessType::RW,
    }
}
