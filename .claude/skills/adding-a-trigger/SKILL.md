---
name: adding-a-trigger
description: Checklist for adding a new TriggerCrud-based trigger type to Windmill (Azure, GCP, Kafka, etc.). Use when wiring a new trigger kind across backend, frontend, CLI, and capture infrastructure.
---

# Skill: Adding a New Trigger Type

Use this skill when adding a trigger kind that implements `TriggerCrud` (Kafka, GCP, Azure, MQTT, SQS, NATS, Postgres, Email…). For native triggers (Nextcloud, Google Drive — things wired through `windmill-native-triggers`), use the `native-trigger` skill instead.

The goal of this doc is to enumerate every file that needs to change. Missing any one of them leads to silent regressions: sync drops the trigger, capture button does nothing, workspace forks lose it, sidebar counters undercount. Follow the checklist top-to-bottom — each section is independent enough to be validated on its own.

Throughout this doc, substitute `{kind}` for the new trigger kind (`azure`, `kafka`, …), `{Kind}` for PascalCase (`Azure`, `Kafka`), `{KIND}` for SCREAMING (`AZURE`, `KAFKA`).

## Reference implementations

- **GCP** — closest analogue to Azure. Has push + pull, OIDC auth, ARM-like resource paths, capture handler. Grep for `gcp_trigger` / `GcpTrigger`.
- **Kafka** — simpler (pull-only, streaming). Good for trivial integrations.
- **Azure** — most recently added (2026). Shared-secret push auth, Event Grid namespaces + basic topics, ARM resource discovery, Namespace-pull data-plane. Grep for `azure_trigger` / `AzureTrigger`.

## 1. Database migration

Create a migration: `cargo sqlx migrate add -r add_{kind}_trigger` from `backend/`. Never write timestamps manually.

The `up.sql` usually defines:
- An optional enum type (e.g. `AZURE_MODE`) if the trigger has sub-kinds
- The `{kind}_trigger` table with at minimum these columns (mirrored from kafka/gcp):
  - primary: `(workspace_id, path)`
  - `script_path`, `is_flow`, `enabled`, `mode`, `permissioned_as`, `edited_by`, `email`
  - `edited_at`, `error`, `server_id`, `last_server_ping`
  - `error_handler_path`, `error_handler_args jsonb`, `retry jsonb`
  - trigger-specific fields
- Indexes on foreign keys + any frequently-filtered columns
- Foreign key to `workspace`

Down migration drops the table and any enum types.

## 2. Backend crate (`windmill-trigger-{kind}`)

Create a new crate under `backend/windmill-trigger-{kind}/` with:

- `Cargo.toml`: features `enterprise`, `private` if EE, standard deps
- `src/lib.rs`: `pub use mod_ee::*;` behind `#[cfg(all(feature = "enterprise", feature = "private"))]`
- `src/mod_ee.rs`: core types + helpers
- `src/handler_ee.rs`: `TriggerCrud` impl + route handlers
- `src/listener_ee.rs`: (only if streaming/pull-based) `Listener` trait impl

Required in `mod_ee.rs`:
- `{Kind}Config` struct (persisted shape, `FromRow`)
- `{Kind}ConfigRequest` struct (what API receives — usually similar to Config but with validation fields)
- `{Kind}Trigger` unit struct (implements the traits)
- `impl TriggerJobArgs for {Kind}Trigger` — sets `TRIGGER_KIND`, `Payload`, `v1_payload_fn`

Required in `handler_ee.rs`:
- `#[async_trait] impl TriggerCrud for {Kind}Trigger` with:
  - `type Trigger = Trigger<{Kind}Config>`
  - `type TriggerConfigRequest = {Kind}ConfigRequest`
  - `const ROUTE_PREFIX: &'static str = "/{kind}_triggers";`
  - `const TABLE_NAME`, `ADDITIONAL_SELECT_FIELDS`
  - `get_deployed_object`, `validate_config`, `create_trigger`, `update_trigger`, `delete_trigger`, `test_connection`
  - `additional_routes` (optional — mount extra endpoints for things like ARM resource listing, topic discovery)

Register the crate in `backend/Cargo.toml` as a workspace member and as a dep of `windmill-api` behind the feature flag.

## 3. Wire into `windmill-api` (feature-gated everywhere)

**`backend/windmill-api/src/triggers/handler.rs`** — mount the trigger crate:
```rust
#[cfg(all(feature = "enterprise", feature = "{kind}_trigger", feature = "private"))]
{
    use crate::triggers::{kind}::{Kind}Trigger;
    router = router.nest({Kind}Trigger::ROUTE_PREFIX, complete_trigger_routes({Kind}Trigger));
}
```

**`backend/windmill-api/src/triggers/{kind}/mod.rs`** — re-export the crate:
```rust
pub use windmill_trigger_{kind}::*;
```

**`backend/windmill-api/src/lib.rs`** — if the trigger receives inbound pushes, add a webhook route:
```rust
.nest("/{kind}/w/{workspace_id}", {
    #[cfg(all(feature = "enterprise", feature = "{kind}_trigger", feature = "private"))]
    { triggers::{kind}::handler_oss::{kind}_push_route_handler() }
    #[cfg(not(...))]
    { Router::new() }
})
```

## 4. `TriggerKind` enum (`backend/windmill-types/src/triggers.rs`)

Already has slots for most triggers but verify your variant exists:
- Add `{Kind}` to the `TriggerKind` enum
- Add match arm in `to_key()`
- Add match arm in `from_str`
- Add match arm in `JobTriggerKind` (if jobs need kind tagging)

## 5. OpenAPI (`backend/windmill-api/openapi.yaml`)

This file is huge and the single most-forgotten place. Add:

- `/w/{workspace}/{kind}_triggers/create` + `/update/{path}` + `/delete/{path}` + `/get/{path}` + `/list` + `/exists/{path}` + `/setmode/{path}` + `/test` paths (mirror gcp section)
- Any `additional_routes` your handler exposes (resource discovery, etc.)
- Schemas: `{Kind}Trigger`, `{Kind}TriggerData`, `{Kind}Mode` (if enum), `{Kind}DeliveryConfig`, helper request/response types
- Add `{kind}` to `CaptureTriggerKind` enum
- Add `{kind}_used: boolean` to the `UsedTriggers` response schema

Regenerate frontend client: `npm run generate-backend-client` from `frontend/`.

## 6. `UsedTriggers` + workspace export

**`backend/windmill-api-workspaces/src/workspaces.rs`** — add `{kind}_used: bool` to the `UsedTriggers` struct and add an `EXISTS(SELECT 1 FROM {kind}_trigger …)` to the `get_used_triggers` query.

**`backend/windmill-api/src/workspaces_export.rs`** — add export block mirroring gcp's (export lists all triggers, serializes them to YAML/JSON). The block re-uses the `trigger_ignore_keys` variable so the new kind automatically participates in fork-export stripping (`mode` field is omitted when the source workspace is a fork — keeps fork→parent merges from flipping the parent's enabled state).

**Fork cloning (`clone_triggers_and_schedules` in workspaces.rs)** — add an `INSERT INTO {kind}_trigger ... SELECT ...` block that copies all rows from the parent workspace, forcing `mode = 'disabled'::TRIGGER_MODE`. Always runs at fork creation; forgetting this means users can't carry `{kind}` triggers into their forks.

## 6.5 Hardcoded trigger-kind arrays (silent-failure hotspots)

Several files keep **hardcoded arrays** of trigger kind strings. Miss one and ACL checks / user offboarding / trash drop your kind:

- **`backend/windmill-api-groups/src/granular_acls.rs`** — `KINDS: [&str; N]`. **Increment N** (the compile error is cryptic otherwise). Controls which kinds accept granular ACL operations.
- **`backend/windmill-api-users/src/users.rs`** (`extra_perms_tables`) — which tables get `extra_perms` entries cleaned when a user is deleted.
- **`backend/windmill-api/src/offboarding.rs`** — three separate arrays (enumeration, fork-copy, and delete paths). **All three** need the new kind.
- **`backend/windmill-api/src/trash.rs`** — `valid_tables` for the trash / restore API.
- **`backend/windmill-git-sync/src/lib.rs`** — add a test assertion for `DeployedObject::{Kind}Trigger.get_kind() == "{kind}_trigger"` (the `get_kind` match arm itself lives in the enum impl — already required by the Rust compiler).
- **`backend/windmill-api-auth/src/scopes.rs`** — add the `{Kind}Triggers` variant to `ScopeDomain` enum + `as_str` match + `from_str` match. Required for the OAuth/token system to recognise `{kind}_triggers:read|write` scopes.
- **`backend/windmill-api/src/token.rs`** (`build_trigger_scope_domains` → `TRIGGER_DOMAINS`) — add `("{kind}_triggers", "{Kind display name}")` so the CreateToken UI's scope selector surfaces the `read` / `write` checkboxes.

**OpenAPI enums** to extend (do NOT forget — generated client will allow it but server rejects as 400):
- `CaptureTriggerKind` enum
- Three `kind` enums under `/w/{workspace}/acls/{get,add,remove}/{kind}/{path}` (yes, same list repeated three times)

After editing any of these, run a full `cargo check` with your feature flag + `gcp_trigger` + other core flags — the `KINDS: [&str; N]` length mismatch only surfaces when the crate compiles.

## 7. Capture infrastructure (`backend/windmill-api/src/capture.rs`)

If the trigger supports push delivery, it also needs a capture endpoint so users can test it:

- `{Kind}TriggerConfig` struct (gated by feature flags)
- `TriggerConfig::{Kind}` variant
- `set_{kind}_trigger_config` function (creates the subscription/equivalent pointing at the capture URL — use your `manage_{kind}_subscription` helper with `trigger_mode=false`)
- Both real + no-op versions behind feature gates
- `TriggerKind::{Kind} => set_{kind}_trigger_config(...)` arm in `set_config`
- `{kind}_payload` async handler — validates auth (if any), processes payload, calls `insert_capture_payload`
- Route: `.route("/{kind}/{runnable_kind}/{*path}", post({kind}_payload))` inside `workspaced_unauthed_service` — and expand the surrounding `#[cfg(any(...))]` to include your feature flag

## 8. CLI (`cli/`) — easy to miss, breaks sync silently

Check all of these:

**`cli/src/types.ts`:**
- Add `"{kind}"` to `TRIGGER_TYPES` array
- Add `"{kind}_trigger"` to `getTypeStrFromPath` return union
- Add match case in `getTypeStrFromPath`'s `typeEnding ===` chain
- Add `pushTrigger("{kind}", ...)` branch in `pushObj`

**`cli/src/commands/trigger/trigger.ts`:**
- Import `{Kind}Trigger` type
- Add `{kind}: {Kind}Trigger` to the `Trigger` type map
- Add `{kind}: wmill.get{Kind}Trigger`, `update{Kind}Trigger`, `create{Kind}Trigger` to each function map
- Add `{kind}: { ... }` template to `triggerTemplates`
- Add `list{Kind}Triggers` call + spread in the `list` aggregation
- Update `--kind` option descriptions to mention the new kind

**`cli/src/commands/sync/sync.ts`:**
- Add `path.endsWith(".{kind}_trigger" + ext)` in the file-type filter
- Add `typ == "{kind}_trigger"` in `getTypeOrder`
- Add `"{kind}_trigger"` to the delete-suffix regex (~line 3092)
- Add a `case "{kind}_trigger"` in the delete switch

**`cli/src/guidance/skills.ts`** — **DO NOT EDIT DIRECTLY**. It's auto-generated by `system_prompts/generate.py`. Instead:
- Edit `system_prompts/utils.py` → append `('{Kind}Trigger', '{kind}_trigger')` to the `SCHEMA_MAPPINGS['triggers']` list (this is the master list — the one in `generate.py` is duplicated and `utils.py` wins)
- Then run `python3 system_prompts/generate.py` — it regenerates `cli/src/guidance/skills.ts` with the schema extracted from `backend/windmill-api/openapi.yaml`
- Commit the regenerated file

## 9. Frontend — editor + drawer

Under `frontend/src/lib/components/triggers/{kind}/`:

- `{Kind}TriggerPanel.svelte` — the tile shown in the triggers listing
- `{Kind}TriggerEditor.svelte` — outer drawer wrapper
- `{Kind}TriggerEditorInner.svelte` — state + business logic; must expose:
  - `openEdit(path, isFlow, defaultValues?)` method
  - `isEditor` prop, `onConfigChange` + `onCaptureConfigChange` callbacks
  - `get{Kind}Config()` + `get{Kind}CaptureConfig()` helpers
  - `captureConfig = $derived.by(untrack(() => isEditor) ? get{Kind}CaptureConfig : () => ({}))`
  - `$effect(() => { const args = [captureConfig, isValid] as const; untrack(() => onCaptureConfigChange?.(...args)) })`
- `{Kind}TriggerEditorConfigSection.svelte` — form fields; use design-system components (`TextInput`, `Select`, `Toggle`, `ToggleButtonGroup`), never raw `<input>`
- `{Kind}Capture.svelte` — capture panel; wraps `CaptureSection` with `captureType="{kind}"`
- `utils.ts` — `requestBody` builders and any trigger-type-specific helpers

## 10. Frontend — global integration

Easy to miss:

- **`frontend/src/lib/components/triggers.ts`** — add `'{kind}'` to the `TriggerKind` union
- **`frontend/src/lib/components/triggers/CaptureWrapper.svelte`**:
  - Import `{Kind}Capture`
  - Add to `isStreamingCapture()` array (streaming = pull-style; push-style is typically `false`)
  - Add `{:else if captureType === '{kind}'}` branch with the `<{Kind}Capture>` render
- **`frontend/src/lib/components/sidebar/SidebarContent.svelte`** — import the icon, add the nav entry
- **`frontend/src/lib/components/sidebar/OperatorMenu.svelte`** — add the operator-mode entry
- **`frontend/src/routes/(root)/(logged)/+layout.svelte`** — destructure `{kind}_used` from `/get_used_triggers` response, push `'{kind}'` into `usedKinds`
- **`frontend/src/lib/components/search/GlobalSearchModal.svelte`** — import icon, add "Go to {Kind} ..." entry
- **`frontend/src/lib/components/offboarding-utils.ts`** — add mappings `{kind}_trigger: '{kind}_triggers'` and `{kind}_trigger: '{kind} trigger'`
- **`frontend/src/lib/components/icons/{Kind}Icon.svelte`** — single-path SVG, `fill={color ?? 'currentColor'}`, `size` prop default 16 (match existing icons — don't hardcode colors, don't use `width`/`height` props)
- **`frontend/src/routes/(root)/(logged)/{kind}_triggers/+page.svelte`** — listing page (mirror `gcp_triggers/+page.svelte` for push+pull, `kafka_triggers` for pure streaming)
- **`frontend/src/lib/components/CompareWorkspaces.svelte`** — workspace fork / compare tool. Needs: service import, editor import, `{kind}Editor` `$state`, `case '{kind}'` in `openTriggerDetails()`, entry in `triggerServices` object (list/delete/normalize), and `<{Kind}TriggerEditor bind:this={{kind}Editor} />` in the template

## 10.5 AI system prompts (`system_prompts/`)

- **`system_prompts/utils.py`** — append `('{Kind}Trigger', '{kind}_trigger')` to `SCHEMA_MAPPINGS['triggers']` (master list used by code generation + CLI skills)
- **`system_prompts/generate.py`** — also has a duplicated `schema_types` list (~line 903) for the AI `triggers` skill content. Add `('{Kind}Trigger', '{kind}_trigger')` there too
- **`system_prompts/generate.py`** `schema_names` (~line 1192) — add `'{Kind}Trigger'` (add `'New{Kind}Trigger'` only if the OpenAPI declares one; GCP and Azure don't)
- Run `python3 system_prompts/generate.py` — this rewrites `cli/src/guidance/skills.ts` and all `auto-generated/` docs. Commit the regenerated files

## 11. Validation

Run all of these before declaring done:

```bash
# Backend
cd backend
cargo check --features enterprise,{kind}_trigger,private   # minimal
cargo check --features enterprise,azure_trigger,private,gcp_trigger,http_trigger,mqtt_trigger,postgres_trigger,sqs_trigger,kafka,nats,smtp,websocket  # full

# SQLx offline data (never run `cargo sqlx prepare` directly — use the wrapper)
./update_sqlx.sh

# Frontend
cd frontend
npm run generate-backend-client
npm run check:fast
```

Smoke test in the UI: create a trigger, save, check it appears in sidebar + search, delete, re-create via CLI `wmill sync`.

## 12. Common pitfalls

- **Forgetting feature gates in `workspaced_unauthed_service()`** — the surrounding `#[cfg(any(...))]` expression must include your feature flag, not just the inner `#[cfg]` on the route
- **`.route(path, ...).route(path, ...)` with same path and different methods** — older axum replaced; use `.route(path, post(h1).options(h2))` to chain methods on the same `MethodRouter`
- **`on:event` directives** — legacy Svelte 4, no-op in runes mode. Use callback props (`onSelected`, `onConfigChange`)
- **`$bindable(default_value)` on optional props** — banned by project CLAUDE.md. Use `$bindable()` + `$derived(prop ?? default)` instead
- **CORS layer intercepting OPTIONS** — tower-http CorsLayer short-circuits OPTIONS before reaching your handler. For server-to-server webhook endpoints, drop the CORS layer entirely (CORS is browser-only)
- **DeliveryAttributeMappings / custom headers for auth** — prefer HMAC or sha256-hashed shared secrets over opaque JWTs when the provider doesn't support signed tokens natively. Store only the hash; regenerate secret on every save
- **ARM / API resource-listing cascades** — if the trigger's resource type is deep (Azure: subscription → RG → namespace → topic), offer dropdowns in the UI populated from the provider's APIs using the user's credential resource
- **Clearing stale selections on dependency change** — when a dropdown's underlying data reloads (e.g., user changes SP or edition), clear selections that no longer match the new list
- **Workspace-scoped tag compatibility** — if the trigger has tags, verify forked workspaces handle them (see commit `0773b5bc85` for a historical fix)

## 13. EE file split

If the trigger is enterprise-only, the code lives in `windmill-ee-private__worktrees/.../windmill-trigger-{kind}/src/*_ee.rs` and is symlinked into the OSS tree. The `windmill-ee-private__worktrees/` directory holds the real files; changes propagate via symlinks. See `docs/enterprise.md` for the workflow.

## 14. Final checklist before PR

- [ ] Migration up/down tested (revert + re-apply)
- [ ] `./update_sqlx.sh` committed the updated `.sqlx/` offline data
- [ ] `cargo check` passes with your feature flag + with all trigger features
- [ ] `npm run check:fast` passes
- [ ] Trigger visible in sidebar with correct icon weight (not oversized/colored — use `currentColor`)
- [ ] Create, edit, delete flow all work in the UI
- [ ] Capture button works (if push-capable)
- [ ] Trigger appears in `/get_used_triggers` → sidebar pulse
- [ ] `wmill sync pull` + `wmill sync push` both round-trip the trigger
- [ ] `wmill trigger list` includes it
- [ ] OpenAPI schemas are complete (no `null` in generated types)
