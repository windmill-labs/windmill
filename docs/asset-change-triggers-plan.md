# Plan: Asset-Change Implicit Triggers

## Goal

Introduce reactive asset-change triggers in Windmill. Scripts declare `#trigger: asset on=change ...` in their source; the system parses annotations on every save, materializes them as rows in an `asset_trigger` table, dispatches jobs when matching asset materialization events occur, and keeps the triggers perfectly in sync with the script.

**Implicit triggers are read-only in the UI, cannot be edited via CLI, and are regenerated from code on every save.**

## Core concepts

- **Implicit trigger**: a trigger row whose source of truth is an annotation in a script. Carries `owner_script_path` + `owner_script_hash`, `is_implicit=true`. Rebuilt on script save; cascades on rename/delete. Never touched by CLI sync.
- **Asset event**: append-only log of writes to assets, keyed by `(workspace, kind, path, partition_key, job_id, at)`. Emitted by the worker when runtime asset detection sees writes.
- **Subscription set**: resolved set of reactive upstream asset paths for a given trigger, derived from inferred reads (minus `exclude`, plus `extra`, filtered by `kinds`).

## Guidance for every stage

- Use `wm-ts-nav` (`outline` / `body` / `def`) to explore before reading whole files. See `AGENTS.md`.
- Coding patterns: `rust-backend` skill for Rust, `svelte-frontend` for UI, `native-trigger` skill for trigger-service patterns.
- Validation after each stage: `docs/validation.md`. EE conventions: `docs/enterprise.md`.
- Migrations: `cargo sqlx migrate add -r <name>` from `backend/`.
- **Do not alter the existing `asset` table**; only append new tables.
- Model `asset_trigger` structure and lifecycle after `kafka_trigger` / `nats_trigger` (schemas visible in `backend/summarized_schema.txt`).

---

## Stage 1 — Asset event log

**Why:** reactive dispatch needs an event stream. The existing `asset` table conflates static decls and runtime usage; an append-only event log is cleaner and keeps existing code untouched.

**Tasks:**

1. Migration `asset_event.up.sql`:
   ```sql
   CREATE TABLE asset_event (
     id BIGSERIAL PRIMARY KEY,
     workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
     asset_kind ASSET_KIND NOT NULL,
     asset_path VARCHAR(255) NOT NULL,
     partition_key VARCHAR(255),
     job_id UUID NOT NULL,
     script_hash BIGINT,
     access_type ASSET_ACCESS_TYPE NOT NULL,
     columns JSONB,
     metadata JSONB,
     at TIMESTAMPTZ NOT NULL DEFAULT now()
   );
   CREATE INDEX ON asset_event (workspace_id, asset_kind, asset_path, at DESC);
   CREATE INDEX ON asset_event (job_id);
   ```
2. In `backend/windmill-common/src/runtime_assets.rs::insert_runtime_assets`, additionally insert `asset_event` rows for every entry with `access_type IN ('w', 'rw')`. Same batch/transaction.
3. After insert, `pg_notify('asset_event', json_payload)` with `(id, workspace_id, asset_kind, asset_path, partition_key)`.
4. Add a feature-gated worker startup hook so notifications are emitted only when dispatcher is enabled.

**Validation:**

- Integration test: run a script that writes an s3 asset → `asset_event` row appears and a notification is emitted.
- Verify no regression in the existing `asset` table writes.

---

## Stage 2 — Annotation parser

**Why:** source of truth for implicit triggers. Language-agnostic, mirrors `parse_volume_annotations`.

**Tasks:**

1. New module `backend/windmill-common/src/trigger_annotations.rs`.
2. Public API: `parse_trigger_annotations(code: &str, lang: ScriptLang) -> Result<Vec<TriggerAnnotation>, ParseErr>`.
3. Grammar (one per line, anywhere in file):
   ```
   <comment_prefix> trigger: asset [on=change] [fires=all|any] [debounce=<dur>]
                                  [only=[path,...]] [exclude=[path,...]] [extra=[path,...]]
                                  [kinds=[datatable|ducklake|s3object|resource|volume,...]]
                                  [partition_map=identity|none|all|window(-Nd,Md)]
                                  [cancel_on_new=true|false]
                                  [backlog=coalesce|replay|skip]
   ```
4. V1 accepts only `on=change` (explicitly reject other values with a "not yet supported" message — easier to extend).
5. Defaults: `fires=all`, `debounce=30s`, `cancel_on_new=false`, `backlog=coalesce`, `partition_map=identity`.
6. Error on multiple `#trigger: asset` annotations in the same script (v1 rule).
7. Reuse language comment prefixes from whatever `parse_volume_annotations` uses.
8. Tests in `backend/tests/` covering: every language prefix, all option combinations, invalid option values, duplicate-annotation rejection.

**Validation:**

- Unit tests, 100% of grammar cases.
- Parser round-trip: `format(parse(s)) == s` for canonical forms.

---

## Stage 3 — `asset_trigger` table + implicit sync on script save

**Why:** this is the feature's backbone — implicit triggers tied to their owner script and kept in sync automatically. Directly addresses the CLI-skip and owner-path pinning requirements.

**Tasks:**

1. Migration creating `asset_trigger` (model after `kafka_trigger`):
   ```sql
   CREATE TABLE asset_trigger (
     workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
     path VARCHAR(255) NOT NULL,
     script_path VARCHAR(255) NOT NULL,
     is_flow BOOLEAN NOT NULL DEFAULT false,

     -- implicit-trigger pinning
     owner_script_path VARCHAR(255),
     owner_script_hash BIGINT,
     is_implicit BOOLEAN NOT NULL DEFAULT false,

     -- trigger config
     on_event VARCHAR(32) NOT NULL,               -- 'change' for v1
     subscription_set JSONB NOT NULL,             -- resolved: {paths:[{kind,path}], fires, ...}
     fires VARCHAR(16) NOT NULL DEFAULT 'all',
     debounce_s INT NOT NULL DEFAULT 30,
     partition_map JSONB,
     cancel_on_new BOOLEAN NOT NULL DEFAULT false,
     backlog VARCHAR(16) NOT NULL DEFAULT 'coalesce',

     -- boilerplate shared with other triggers
     error TEXT,
     server_id VARCHAR(64),
     last_server_ping TIMESTAMPTZ,
     error_handler_path VARCHAR(255),
     error_handler_args JSONB,
     retry JSONB,
     extra_perms JSONB NOT NULL DEFAULT '{}',
     edited_by VARCHAR(255) NOT NULL,
     edited_at TIMESTAMPTZ NOT NULL DEFAULT now(),
     email VARCHAR(255) NOT NULL,
     mode trigger_mode NOT NULL DEFAULT 'enabled',
     labels TEXT[],

     PRIMARY KEY (workspace_id, path)
   );
   CREATE INDEX ON asset_trigger (workspace_id, owner_script_path) WHERE is_implicit = true;
   ```

2. New module `backend/windmill-common/src/implicit_triggers.rs`:
   - `sync_implicit_asset_triggers_for_script(db, workspace_id, script) -> Result<()>`:
     - Parse annotations (Stage 2).
     - Run `AssetsFinder` on the script body; compute the reactive read set (filtered by `kinds`, minus `exclude`, plus `extra`, or replaced by `only`).
     - Deterministic implicit path: `"impl:asset:{script_path}"` (or `{script_path}/__asset_trigger__` — pick one; document it).
     - UPSERT one `asset_trigger` row with `is_implicit=true`, `owner_script_path=script.path`, `owner_script_hash=script.hash`.
     - DELETE implicit asset_trigger rows for this owner_script_path not in the annotation set.

3. Wire into `backend/windmill-api-scripts/src/scripts.rs` after the existing asset-detection path, in the same transaction as the script save.

4. Cascade hooks:
   - **Rename** (`update_script_path`): update both `owner_script_path` and `script_path` on matching implicit rows.
   - **Archive/delete**: delete matching implicit rows.
   - **Revert to previous version**: re-run sync using the restored hash.

5. API endpoints in `backend/windmill-api/src/` (new `asset_triggers.rs`):
   - `GET /w/:ws/asset_triggers/list` — returns all rows, with `is_implicit` badge.
   - `GET /w/:ws/asset_triggers/get/:path`.
   - `POST/PUT/DELETE` on rows where `is_implicit=true` return **409 Conflict** with body `{ error: "Implicit trigger — edit script annotations in <owner_script_path>" }`.
   - OpenAPI update in `backend/windmill-api/openapi.yaml`.

6. **CLI sync** (`cli/src/...`):
   - Pull: exclude rows with `is_implicit=true` (grep existing sync flow for other trigger types, add filter).
   - Push: never create/update/delete asset_trigger rows with `is_implicit=true` flag set remotely. Never generate local YAML for implicit triggers.
   - Document in CLI reference.

7. Flow save path: same treatment if flow root YAML has annotations (stretch; can defer to v2).

**Validation:**

- Save script with annotation → one row appears, `is_implicit=true`, `owner_script_path` set.
- Edit annotation → row updates.
- Remove annotation → row deleted.
- Rename script → `owner_script_path` and `script_path` updated.
- Archive script → row deleted.
- `wmill sync pull` on a workspace with implicit triggers → no files emitted for them.
- `wmill sync push` with a local YAML that tries to set `is_implicit=true` → rejected or silently ignored (pick one; document).
- UI/API mutation attempt on implicit row → 409.

---

## Stage 4 — Dispatcher service

**Why:** turn asset events into script runs.

**Tasks:**

1. New crate `backend/windmill-trigger-asset/` modeled on `backend/windmill-trigger-kafka/`.
2. Listener:
   - Start-of-day catch-up: read `asset_event.id > last_processed_id` for this server.
   - LISTEN on `asset_event` channel for live events.
3. Matching:
   - For each event, SELECT asset_trigger rows where `subscription_set @> [{kind, path}]` (JSONB containment on a GIN-indexed column) and `mode='enabled'`.
   - Optionally narrow by `workspace_id`.
4. Policy application per row:
   - `debounce_s`: track a per-trigger "pending dispatch" in a small state table (`asset_trigger_pending`); consolidate events within the window.
   - `fires=all`: SELECT MAX(at) FROM asset_event for each path in subscription_set; fire only if every path has `max(at) > last_fired_at`.
   - `fires=any`: fire on first qualifying event.
   - `cancel_on_new=true`: before enqueue, cancel any in-flight run for this trigger (reuse `cancel_persistent_script` pattern).
   - `partition_map=identity`: pass event's `partition_key` as `wm_partition`.
5. Enqueue via existing `push` with `JobPayload::ScriptByPath` (or hash), populating reserved args:
   ```json
   { "wm_asset_event": {...}, "wm_partition": "...", "wm_backfill": false }
   ```
6. Server-ownership: `server_id` + `last_server_ping` pattern — only one server dispatches per trigger; rebalancing identical to Kafka trigger service.
7. Register service in `backend/windmill-worker/src/agent_workers.rs` and main wiring.
8. **EE gate**: dispatcher likely EE; sync + table are OSS. Mirror existing EE/OSS split pattern (see `docs/enterprise.md`).

**Validation:**

- Write an asset manually via SQL → subscribed script runs.
- Burst of 100 writes in 10s with `debounce=30s` → one run.
- Two-upstream `fires=all` → one run only when both upstreams have newer events.
- `cancel_on_new=true`: long-running job, second event arrives → first job cancelled.

---

## Stage 5 — Script-signature validation at save

**Why:** prevent un-fillable required args on asset-triggered scripts.

**Tasks:**

1. In `windmill-api-scripts/src/scripts.rs` save path, after annotation parse:
   - Reserved names: `wm_asset_event`, `wm_partition`, `wm_backfill`.
   - For each required (no-default) arg in the parsed signature outside this set: fail save with:
     > `Script has required parameter '<arg>' but is asset-triggered. Give it a default, make it optional, or remove the asset trigger annotation.`
2. Runtime side (`windmill-worker/src/worker.rs`, wherever args are materialized): when invoked by an asset trigger (detect via job's trigger-metadata payload), fill reserved args from `wm_asset_event`.
3. Unit + integration tests.

**Validation:**

- Save rejects required un-reserved arg.
- Save accepts with defaults or reserved names.
- Runtime: reserved args populated correctly.

---

## Stage 6 — UI surface

**Why:** make implicit triggers visible (but not editable) in the existing trigger UX.

**Tasks:**

1. Triggers page: new "Asset" tab listing `asset_trigger` rows with "Implicit" badge linking to `owner_script_path`.
2. Disable edit/delete actions on `is_implicit=true` rows; show read-only detail view.
3. Script editor right-panel: parsed triggers section showing resolved subscription set + options. Reuse existing asset-panel patterns (`FlowAssetsHandler.svelte`).
4. Warning surface: if signature validation would fail (stage 5), show it in the editor before save.
5. Use `svelte-frontend` skill conventions throughout.

**Validation:**

- UI shows implicit trigger, linked to source script.
- Edit button absent / disabled.
- Resolved subscription set matches expectations.

---

## Stages 7–9 (follow-up, not in scope for this delivery)

Outlined so the implementing agent knows what's coming and avoids painting in corners:

- **Stage 7 — Asset detail page + materialize verb**: `/assets/:kind/:path` page, `asset_producer` table, `POST /assets/materialize`.
- **Stage 8 — Partitioning**: `#produces` annotation with partition spec, `wm_partition` plumbed fully through worker, `partition_map=window|all`, backfill UI.
- **Stage 9 — Canvas editor**: workspace/folder-filtered node-and-edge view over `scripts × assets × asset_triggers` with freshness/status overlays and in-place edit.

Stages 7–8 should be revisited before freezing stage 3/4 APIs — specifically `subscription_set` JSONB shape needs partition fields already envisioned so stage 8 is an additive migration.

---

## Hand-off notes

- **Sequencing**: stages 1→2→3 are linearly dependent. Stage 4 depends on 1+3. Stage 5 depends on 2+3. Stage 6 depends on 3 (and ideally 5). An agent can take stages in order, each as its own PR.
- **Forward-compatibility**: leave `partition_key` and `partition_map` fields in place from stage 1, even though they're unused in v1. Saves a later migration.
- **Existing patterns to follow**:
  - Trigger service: `backend/windmill-trigger-kafka/` and `backend/windmill-trigger-nats/`.
  - Annotation parsing: `parse_volume_annotations`, `parse_ci_test_annotation`.
  - Asset inference: `AssetsFinder`, `AssetCollector`, `ParseAssetsResult`.
  - Runtime asset write path: `insert_runtime_assets`, `register_runtime_asset`.
  - CLI sync skip logic: grep for existing exclusion patterns in `cli/src/`.
