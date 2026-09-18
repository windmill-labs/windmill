# Operator write rights

Most of `workspace_settings.operator_settings` is visibility: flags that hide pages from operators
so the UI stays uncluttered. They are not enforced, and were never meant to be.

`manage_schedules` and `manage_triggers` are different. They are **enforced** on the write paths,
because hiding the schedules page never stopped an operator creating a schedule through the API,
the CLI or MCP. An admin who wants operators to see what is scheduled without letting them change
it could not express that with a visibility flag alone.

Read them with `windmill_common::workspaces::operator_manage_rights` (60s cache) and gate a write
with `check_operator_can_manage`, naming a `ManageKind`. Call it **before** opening an RLS
transaction: it takes a connection from the root pool, and a second pooled connection held
alongside a transaction self-deadlocks on a one-connection pool.

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
