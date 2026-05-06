# Triggers and schedules in workspace forks

A workspace fork is a developer-controlled copy of a parent workspace, used to
test changes before merging back via git sync. Triggers and schedules in a
fork need special handling for two reasons:

1. **Listener take-over**: most trigger kinds (Kafka, Postgres, MQTT, NATS,
   SQS, GCP, Azure) attach to a stateful upstream resource. Two listeners
   sharing the same identifier compete for events.
2. **Merge round-trip**: every change made in a fork can flow back to the
   parent through git sync. If the fork sets a trigger to `disabled`, that
   value would otherwise overwrite the parent's `enabled` state on merge.

## Cloning model — always cloned, always disabled

Fork creation always runs `clone_triggers_and_schedules`. Every row in each
`*_trigger` table and in `schedule` is copied from the parent into the fork
with two invariants:

- **Always disabled.** The clone forces `mode='disabled'::TRIGGER_MODE` on
  triggers and `enabled=false` on schedules, regardless of the parent's
  state. Disabled rows have **no side effects** — no listener attaches to
  the upstream, no cron fires — so this clone is safe by construction.
  The user re-enables manually in the fork.
- **Listener identifiers copied verbatim.** Stored values for `group_id`,
  `replication_slot_name`, `subscription_name`, `client_id`,
  `consumer_name`, etc. are copied 1:1. Until the runtime-suffix work
  ships (see below), enabling a cloned listener in the fork would compete
  with the parent — the conflict warning below catches that case.

`native_trigger` (Nextcloud, Google Drive, GitHub) is intentionally **not
cloned**. Those triggers manage external webhook state we don't want
duplicated.

**Non-workspaced HTTP triggers are also skipped.** A row with
`workspaced_route=false` (and where neither `CLOUD_HOSTED` nor the
`HTTP_ROUTE_WORKSPACED_ROUTE` instance setting is on) has a runtime URL
without any workspace prefix. A clone would collide with the parent's row at
the matchit router level, where duplicate inserts are silently dropped — one
trigger would invisibly hijack the other. There is no namespacing escape
hatch for these (the whole point of `workspaced_route=false` is to skip the
prefix), so the clone filter excludes them. The fork user can re-create one
manually if they need it. When `CLOUD_HOSTED` or `HTTP_ROUTE_WORKSPACED_ROUTE`
is on, every route is workspace-prefixed at runtime regardless of the column,
and the clone copies all rows.

**Non-workspaced email triggers are skipped on the same grounds.** A row
with `workspaced_local_part=false` exposes a bare `local_part@domain`
address shared instance-wide; a clone would share the address with the
parent and incoming mail would be delivered arbitrarily. The clone filter
copies email triggers only when `workspaced_local_part IS TRUE` (or
`CLOUD_HOSTED`, since cloud scopes email lookup by `workspace_id` natively).

## Merge-direction filter (always on)

Whenever the source workspace has `parent_workspace_id IS NOT NULL` (i.e.
it's a fork), the tarball export at `/api/w/{workspace}/workspaces/tarball`
strips fork-local fields:

- `mode` from every `*_trigger` row
- `enabled` from every `schedule` row

The fork-detection key is the column, not the `wm-fork-*` naming convention,
so it stays consistent with the conflict-warning gates in `set_trigger_mode`
and `set_schedule_enabled` and survives any future ID rename.

The trigger update handler complements this: when an incoming `update_trigger`
request omits both `mode` and `enabled`, the existing DB value is preserved
instead of falling back to the BaseTriggerData default of `Enabled`. This
means the fork→parent merge cannot flip the parent's operational state, even
if the fork has an explicit (locally-disabled) state for that path.

The schedule `EditSchedule` payload already lacks an `enabled` field, so its
update path is naturally safe.

## Conflict warning on enable

The `set_*_trigger_mode` endpoint fires the warning whenever a fork transitions
to a mode that *attaches a listener* — `Enabled` or `Suspended`. Suspended is
not "off": the listener still attaches and consumes events; only the auto-run
of queued jobs is paused. Two suspended forks would still split Kafka events
or share a Postgres slot with the parent. `Disabled` is the only mode that
fully detaches.

The check fires whenever the parent workspace has a row at the same trigger
path — **regardless of the parent's current `mode`/`enabled`**. If so, the
endpoint rejects the request with an error string of the shape:

```
fork-conflict:<kind>:<parent_workspace_id>
```

The frontend's `withForkConflictRetry` helper detects this prefix, asks the
user to confirm via a dialog (a `ConfirmationModal` mounted at the logged
layout root, driven by the `forkConflictModal` store), and re-issues the
call with `force: true` if the user agrees. The CLI sees the raw error.

The check fires whenever the parent has the row because the fork's row was
*cloned* from the parent — the upstream identifier (Kafka group, PG slot,
SQS queue URL, GCP/Azure subscription, …) is shared by construction. That
sharing is a risk independent of the parent's current state:

- Both enabled → the listeners compete (split events) or fire twice.
- Parent disabled → the fork can destructively claim shared state (PG WAL
  advance, Azure secret_hash reuse, MQTT client_id race) before the parent
  re-enables.

The check is opt-out per kind via `TriggerCrud::FORK_CONFLICT_ON_ENABLE`
(default `true`). It is **skipped** for kinds whose upstream identifier is
already workspace-scoped at runtime — fork and parent there can never share
a real upstream:

- **HTTP** — routes are `/r/<workspace_id>/...`; cloned rows always have
  `workspaced_route=true` (non-workspaced are filtered out at clone time).
- **Email** — addresses are workspace-prefixed; cloned rows always have
  `workspaced_local_part=true`.

The check **fires** for every other kind. The frontend modal copy splits the
conflict into three families so the user can act on the right risk:

- **Split events** (Kafka, NATS, MQTT, SQS, GCP, Azure) — events split
  between the two listeners; each side receives a fraction of its traffic.
- **Duplicate firing** (Websocket, Schedule) — every event fires the script
  twice (once in fork, once in parent).
- **Slot takeover** (Postgres) — the replication slot is exclusive *and*
  destructive: enabling either errors with "slot already active" (parent
  enabled) or hijacks the WAL position (parent disabled).

This warning is the *durable* solution for trigger kinds where the conflict
cannot be eliminated by namespacing alone:

- **SQS** — the queue *is* the event source; two consumers will compete for
  messages no matter what.
- **GCP-Existing subscription** — same as SQS.
- **Schedule** — same wall-clock firing.

For the kinds that *can* be auto-namespaced (see below), the warning is the
short-term placeholder until that work lands.

## Merge UI behavior

Triggers and schedules flow through the same `compare_workspaces` API as
scripts, flows, and the rest. `tally_deployed_object_changes` records every
trigger/schedule mutation in `workspace_diff`, and `compare_two_*` strips
runtime fields (`mode`, `enabled`, `server_id`, `last_server_ping`,
`edited_at`/`edited_by`, `error`, `extra_perms`, `permissioned_as`,
`workspace_id`, `email`) plus server-managed fields the merge feature
treats as workspace-local (`subscription_id` for GCP, `push_auth_config`
for Azure — both regenerated by their deploy handlers per workspace) before
comparing. A fresh fork (where only `mode`/`enabled` differ from the parent)
reports no diffs, and a successfully-merged GCP/Azure push trigger doesn't
re-appear in the diff list because of the regenerated subscription id /
auth secret.

Both the merge UI (`CompareWorkspaces.svelte`) and `wmill workspace merge`
consume those rows directly. All ten deployable trigger kinds — HTTP,
Websocket, Kafka, NATS, Postgres, MQTT, SQS, GCP, Azure, Email — plus
schedules are handled. Non-workspaced HTTP routes and email triggers are
intentionally not cloned at fork creation (they would collide instance-wide),
but `tally_deployed_object_changes` still records mutations against them; the
deploy will fail at the workspace-collision check if the user tries to
deploy a non-workspaced row to a fork.

Operational state (`mode` for triggers, `enabled` for schedules) is handled
asymmetrically between update and create:

- **Update**: the merge deploy strips `mode`/`enabled` so the target's existing
  state is preserved. Triggers rely on the backend's `is_mode_unspecified()`
  safeguard in `update_trigger`; schedules rely on `EditSchedule` lacking the
  `enabled` field. Both deploy paths perform the strip
  (`stripOperationalState` in `utils_deployable.ts`, `preparePayload` in
  `merge.ts`). This matches the YAML round-trip's `fork_trigger_ignore_keys`
  / `fork_schedule_ignore_keys`.
- **Create**: the source's `mode`/`enabled` is passed through. There's no
  target row to preserve, so a fork-only trigger or schedule lands with the
  state the fork creator chose. When the source omits the flag entirely (e.g.
  legacy clients), the backend defaults to `enabled` for both kinds —
  `BaseTriggerData::mode()` returns `Enabled` and the schedule insert defaults
  to `true` to match.

## Future work — runtime listener suffix

A follow-up PR will append a fork-specific suffix to the upstream identifier
at runtime for the kinds that support it:

| Kind | Identifier | Notes |
|---|---|---|
| Kafka | `group_id` | Two consumer groups never share messages. |
| MQTT | `client_id` | Brokers reject duplicate client_ids; suffix avoids that. |
| NATS | durable consumer name | Fork consumes independently. |
| Postgres | `replication_slot_name` + `publication_name` | Fork auto-creates its own publication on enable, drops on disable / fork delete. |
| Azure Event Grid | `subscription_name` | `manage_azure_subscription` creates the suffixed sub in Azure. |
| GCP Pub/Sub (CreateNew) | `subscription_id` | `manage_google_subscription` creates the suffixed sub. |

The suffix is applied at runtime by the listener — the *stored* identifier
column never carries the suffix, so nothing extra needs to be filtered on
export. The follow-up also adds cleanup-on-fork-delete hooks for the
upstream resources (Azure / GCP / Postgres publication) so deleted forks
don't leak external state.
