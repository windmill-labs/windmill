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

## Operational state is owned by the parent

The rule that makes both the normal-git PR merge and the in-app merge behave:

> **A trigger's `mode` (and a schedule's `enabled`) belongs to the parent
> workspace. Git-sync *reads* the parent's value into a fork's synced file and
> *never writes* a fork's value back. No git-sync / merge / create / update
> write sets a fork's operational state; the `setmode` / `setenabled` endpoint
> is the intended explicit mutator.**

(Runtime error handling can still auto-disable an errored trigger or schedule —
that's orthogonal to this rule, which governs git-sync/merge/create/update
writes.)

This is enforced in two halves, keyed off `parent_workspace_id IS NOT NULL`
(the column, not the `wm-fork-*` naming convention — it stays consistent with
the conflict-warning gates and survives any future ID rename):

**Read half — parent-value substitution on export.** When the source workspace
is a fork, the tarball export at `/api/w/{workspace}/workspaces/tarball`
rewrites each trigger's `mode` (and each schedule's `enabled`) to the
*parent's* value for the same path, looked up at export time. A fork-only path
(absent from the parent) keeps the fork's own value — there's no parent state
to defer to, so it lands with whatever the fork creator set.

The earlier design *stripped* these fields instead. That broke a normal-git PR
merge: the parent branch (and the merge base) carries the line, the fork branch
dropped it, so the 3-way merge either silently deleted `mode`/`enabled` from
the parent — corrupting the source of truth — or conflicted outright when the
parent had also edited it. Substituting the parent's value makes the fork's
file byte-identical to the parent on that field, so the merge has nothing to
resolve.

**Write half — an *update* into a fork preserves its existing state.** The
read-half writes the *parent's* (often enabled) value into a fork's synced
file. If that file is then written back into the fork — `wmill sync push`,
compare-workspaces "Update current", or any bidirectional sync — a naive update
would flip the fork's trigger to the parent's state and re-create the listener
conflict. So `update_trigger` preserves the fork's existing `mode` for a fork
target (`workspace_is_fork` in `windmill-trigger/src/handler.rs`); schedule
`enabled` is naturally preserved because `EditSchedule` has no `enabled` field.

**Create is *not* forced.** A create into a fork keeps the caller's chosen
state, so creating a trigger/schedule in the fork UI works normally and a
fork-only row lands enabled if the user wants it. A fork-only row has no parent
counterpart, so it can't share an upstream identifier and can't conflict. (Rows
*cloned* from the parent at fork creation still start disabled — that's forced
in the SQL clone, `clone_triggers_and_schedules`, not in the create handler.)
The fork owner toggles state via `setmode`/`setenabled` (which carry the
conflict warning below) — those endpoints are unchanged, so the UI's enable/
disable controls behave the same in a fork as anywhere else.

For a non-fork target the incoming value is applied as given — so a fork→parent
merge of an existing trigger writes the parent's own (substituted) value (a
no-op), and a fork-only trigger lands with the fork creator's chosen state.

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

Operational state (`mode` for triggers, `enabled` for schedules) follows the
same "owned by the parent" rule as the git round-trip (see *Operational state
is owned by the parent* above) — the two paths share the backend `create`/
`update` handlers, so they can't diverge.

- **Update**: the merge deploy strips `mode`/`enabled`
  (`stripOperationalStateOnUpdate` in the shared `windmill-utils-internal`
  package, `cli/windmill-utils-internal/src/deploy.ts`), so the target's
  existing value is preserved — equivalent to substituting the target's value.
  For a fork target the backend preserves it regardless (`workspace_is_fork`);
  for a parent target the `is_mode_unspecified()` safeguard does. Schedules also
  rely on `EditSchedule` lacking the `enabled` field.
- **Create**: the source's `mode`/`enabled` is passed through (into a fork or a
  parent alike) — there's no row to preserve, so the trigger/schedule lands with
  the state the source chose (omitting the flag defaults to `enabled`:
  `BaseTriggerData::mode()` → `Enabled`, schedule insert → `true`). Create is not
  force-disabled for forks: a fork-only row has no parent counterpart and can't
  conflict, and rows cloned from the parent already start disabled via the SQL
  clone at fork creation.

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
