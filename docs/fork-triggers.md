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

## Default behavior — `fork_triggers = false`

Forks created without opting in start trigger-empty. The fork has no rows in
any `*_trigger` table or in `schedule`. This is the safest default: nothing
in the fork can compete with the parent or alter the parent's state on merge.

## Opt-in cloning — `fork_triggers = true`

When the user ticks "Clone triggers and schedules" (UI) or passes
`--fork-triggers` (CLI), fork creation also runs `clone_triggers_and_schedules`
which copies every row from each `*_trigger` table and from `schedule` into
the fork. Two invariants apply:

- **Always disabled.** The clone forces `mode='disabled'::TRIGGER_MODE` on
  triggers and `enabled=false` on schedules, regardless of the parent's
  state. The user re-enables manually in the fork.
- **Listener identifiers copied verbatim.** Stored values for
  `group_id`, `replication_slot_name`, `subscription_name`, `client_id`,
  `consumer_name`, etc. are copied 1:1. Until the runtime-suffix work
  ships (see below), this means enabling a cloned listener in the fork
  would compete with the parent.

`native_trigger` (Nextcloud, Google Drive, GitHub) is intentionally **not
cloned**. Those triggers manage external webhook state we don't want
duplicated.

## Merge-direction filter (always on)

Whenever the source workspace ID matches `wm-fork-*`, the tarball export at
`/api/w/{workspace}/workspaces/tarball` strips fork-local fields:

- `mode` from every `*_trigger` row
- `enabled` from every `schedule` row

The trigger update handler complements this: when an incoming `update_trigger`
request omits both `mode` and `enabled`, the existing DB value is preserved
instead of falling back to the BaseTriggerData default of `Enabled`. This
means the fork→parent merge cannot flip the parent's operational state, even
if the fork has an explicit (locally-disabled) state for that path.

The schedule `EditSchedule` payload already lacks an `enabled` field, so its
update path is naturally safe.

## Conflict warning on enable

The `set_*_trigger_mode` and `set_schedule_enabled` endpoints check whether
the parent workspace has the same path enabled. If so, they reject the
request with an error string of the shape:

```
fork-conflict:<kind>:<parent_workspace_id>
```

The frontend's `withForkConflictRetry` helper detects this prefix, asks the
user to confirm via a dialog, and re-issues the call with `force: true` if
the user agrees. The CLI sees the raw error.

This warning is the *durable* solution for trigger kinds where the conflict
cannot be eliminated:

- **SQS** — the queue *is* the event source; two consumers will compete for
  messages no matter what.
- **GCP-Existing subscription** — same as SQS.
- **Schedule** — both crons fire on the same wall-clock; deduplication has
  to happen in the script logic.

For the kinds that *can* be auto-namespaced (see below), the warning is the
short-term placeholder until that work lands.

## Future work — runtime listener suffix (Phase 3)

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

The suffix is invisible in the stored row and never reaches the parent on
merge — the merge-direction filter strips identifier columns too.

`Phase 3` also adds cleanup-on-fork-delete hooks for the upstream resources
(Azure / GCP / Postgres publication) so deleted forks don't leak external
state.
