# Operator write rights

Most of `workspace_settings.operator_settings` is visibility: flags that hide pages from operators
so the UI stays uncluttered. They are not enforced, and were never meant to be.

`manage_schedules` and `manage_triggers` are different. They are **enforced** on the write paths,
because hiding the schedules page never stopped an operator creating a schedule through the API,
the CLI or MCP. An admin who wants operators to see what is scheduled without letting them change
it could not express that with a visibility flag alone.

## Where the gate lives

On the router, not in the handlers. `gate_operator_writes` is layered in `windmill-api/src/lib.rs`
over the schedules router, the trigger routers, the native-trigger routers and capture, and refuses
anything that is not a GET/HEAD/OPTIONS.

That is not a style choice. A trigger kind can register routes of its own beside the shared CRUD
ones — bulk HTTP creation, the Postgres publication and replication-slot setup — and those are
hand-written, one per feature. The first version of this checked each handler, and every one of
those extra routes was missed, along with the whole native-trigger family, which does not use the
shared handlers at all. On the router the author of the next route writes nothing and is covered
anyway.

**A layer only covers the routers it is on.** It closes routes added *inside* a gated router; it
says nothing about a new feature that performs trigger writes from a router of its own. Capture is
exactly that — it configures a trigger without creating one, and saving a Postgres capture config
creates a replication slot and a publication on the target database — and it needed its own layer
rather than inheriting one. Before adding a feature that writes trigger or schedule state, ask
which router it lands on.

Two consequences to keep in mind when adding a route under one of these:

- A read served over POST gets refused. Two exist today, a trigger's connection test and the HTTP
  route-path availability check; both are steps inside a create form a withdrawn operator cannot
  open. Put new reads on GET.
- Anything mounted under a gated router inherits the gate. The native-trigger mount also carries
  the workspace's integration setup, which is a settings concern, so the layer goes on the trigger
  routes alone rather than the whole mount.

`check_operator_can_manage` is still the function underneath, for a write that cannot be reached
through one of these routers.

The cache is per process, so withdrawing a right has to reach every replica: an `AFTER UPDATE OF
operator_settings` trigger writes a `notify_operator_settings_change` row and `process_notify_event`
drops the entry. Keep both ends if you touch either, or a workspace that withdrew a right keeps
authorizing writes on every other replica until its own entry expires.

## Granted unless withdrawn

These name capabilities operators already hold, so absence has to mean "never configured", not a
value. That is easy to get wrong in two places, and both were wrong in the obvious first draft:

- The read coalesces to **true** (`operator_manage_rights`), including for a workspace with no
  `workspace_settings` row, which is what `OperatorManageRights::default` is for. Coalescing to
  false instead revokes the right on upgrade for every workspace that ever saved operator settings,
  since those rows carry explicit keys and none of them is this one.
- The update endpoint **merges** into the stored jsonb and takes these two as `Option<bool>`, so an
  omitted key keeps its stored value. It has to: `operator_settings` is git-synced as a whole
  object (`cli/src/core/settings.ts` posts the file's contents verbatim), so a settings file
  written before these keys existed reaches the endpoint on every pull. A serde or SQL default of
  either polarity turns that pull into a silent withdrawal or a silent restoration.

The visibility flags are plain `bool` and always serialize, so the merge is a no-op for them and
their behaviour is unchanged.

Do not model a right that an admin *grants* on these. Granted-by-default and granted-on-request
are opposite polarities, and a gate written for one is wrong for the other.
