# Windmill

Open-source platform for internal tools, workflows, API integrations, background jobs and UIs. This file pins the vocabulary that is specific to Windmill's domain, so that code, docs and reviews name the same thing the same way.

## Language

### Flows

**Step**:
One node of a flow — the unit a user selects in the graph and configures in the right-hand panel. Typed as `FlowModule` in code.
_Avoid_: module (ambiguous with the architectural sense), node, action

**Step setting**:
A per-step runtime option stored on the step itself: retries, error handling, timeout, concurrency limit, priority, cache, debounce, early stop, skip, suspend, sleep, lifetime. Distinct from the step's inputs and its code. The panel that edits them is the **run settings** tab; a single setting is still a step setting.
_Avoid_: advanced setting, step config, flow option

**Configured**:
Said of a step setting whose config object is present on the step. Deliberately not the same as "would change the runtime's behaviour" — a setting can be configured and still be a no-op (`sleep` of `0`). Every surface that answers "is this setting on?" answers it this way.
_Avoid_: enabled, active, effective

**Trigger step**:
The first step of a polling flow. It runs on a schedule and returns the items found since its last run; an empty return means there is nothing to process and the flow stops early, marked skipped rather than failed.
_Avoid_: poll script, trigger node, schedule step

**Default predicate**:
The `stop_after_if` expression seeded onto a trigger step at creation, encoding what "nothing new" looks like. One value, owned in one place — historically it was re-invented per creation path and the copies diverged.
