# Feature usage telemetry

`feature_usage` is the product-telemetry accumulator: day-bucketed counters that roll into the
anonymous usage-stats payload. It answers "does anyone use this, and which variant do they pick"
without any identifying data leaving the instance.

It currently carries 14 registered actions across three features (`ai_session`, `ai_chat`,
`flow_editor`). Nearly all of the product is uninstrumented, so new user-facing work is the
opportunity to change that.

## When to instrument

Raise it **in the plan**, with the concrete vocabulary written out, and let the user keep or drop
it in one line. Don't stop and ask as a standalone question.

Propose it when a new user-facing affordance leaves a real question open:

- a new panel, mode, tab, toggle, or entry point — is it discovered and used at all?
- competing UX paths, or a new default — which one wins?
- an opt-in or beta gate — what is the take rate?
- a multi-step flow — where do people stop?

Stay silent for bugfixes, refactors, internal plumbing, and anything whose useful signal would
need per-item identifiers (paths, names, prompts, code) — those cannot be logged at all, see
[Privacy rules](#privacy-rules). If the answer wouldn't change a decision, instrumenting is
overkill; say nothing.

## Designing the vocabulary

| Field | Meaning | Limits |
|---|---|---|
| `feature` | Product area: `ai_chat`, `flow_editor` | ≤50 chars |
| `kind` | The action within it: `message`, `panel_placement`. `(feature, kind)` is the allowlisted pair | ≤50 chars |
| `key` | A facet of the action — mode, tab kind, tool name, `provider:model`. Aggregation groups by `(feature, kind, key)`, so this is what splits one counter into comparable buckets | ≤100 chars, identifier-shaped, optional |
| `entity_id` | An **opaque random** id (e.g. a session id) when you need per-entity distributions rather than a flat count | ≤50 chars, identifier-shaped, optional |
| `value` | Increment, default 1 | clamped to 1…1,000,000 |

Identifier-shaped means ASCII alphanumerics plus `_ - : . /` — no spaces. Anything else is
rejected.

Supplying `entity_id` is what unlocks the distribution stats: the payload reports `entity_count`,
`total_value`, `median_value`, `p90_value`, and `inactive_3d_entity_count` per
`(feature, kind, key)`. Omit it for a plain "how many times did this happen" counter. Keep the key
vocabulary closed and small — enumerate the values in a TS union next to the call site, the way
`flowEditorTelemetry.ts` does, so the whole set is reviewable in one place.

## The recipe

Four steps. Skipping step 1 or 3 fails quietly.

**1. Register the pair** in `FEATURE_USAGE_KINDS`
(`backend/windmill-api-workspaces/src/workspaces.rs`). An unregistered `(feature, kind)` is
dropped by `valid_feature_usage_event` with a bare `continue` — no error, no log, still a 204 to
the browser. Frontend-only instrumentation records **nothing** and looks like it worked.

**2. Log from the frontend:**

```ts
import { logFeatureUsage } from '$lib/utils/featureUsage'

logFeatureUsage('flow_editor', 'panel_placement', { key: 'force_detach' })
```

Fire-and-forget. Events sum locally per `(workspace, feature, kind, key, entityId)` and flush
every 30s, on `visibilitychange` → hidden, and on `pagehide`; 50 events per request, and a failed
batch is dropped rather than retried.

**3. Update the disclosure.** `InstanceSettings.svelte` lists what a non-minimal payload contains
(two places — the copy appears twice). A new counter that isn't named there means the instance
under-discloses what it sends. This has already drifted once.

**4. Verify a row lands.** The silent-drop path means "no error" proves nothing:

```sql
SELECT feature, kind, key, entity_id, day, value FROM feature_usage ORDER BY updated_at DESC LIMIT 10;
```

## Privacy rules

Only aggregated counts ever leave the instance, and only when telemetry is enabled and minimal
mode is off. Never put a path, prompt, script body, workspace name, email, or any user identifier
into `key` or `entity_id`. Entity ids must be opaque random ids, never anything that maps back to
a user or a resource. If the signal you want can only be expressed with identifying data, it
cannot be collected — drop it.

Counters aggregate over the last 30 days; rows are pruned after 60.

## Backend-only features

Ingestion is frontend-only: `log_feature_usage` is an HTTP route the browser posts to, and there
is no Rust-side helper. A feature with no UI cannot be instrumented today without adding one.
Scope the default to user-facing work, and say so rather than implying backend coverage exists.
