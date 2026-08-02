# Removing `schedule.email`

A schedule's identity is its `permissioned_as`; the address beside it is a function of that
principal. `schedule.email` is no longer read for the identity, but it is still written, because
workers before **1.777** select it in `get_schedule_opt` — inside the same transaction as the job
completion, so a missing column rolls that completion back and the occurrence runs again. That is
the failure to keep in mind throughout: not a wrong address, a schedule that runs twice.

The column can go once no worker older than 1.777 can be live, in practice once
`MIN_KEEP_ALIVE_VERSION` (`windmill-common/src/min_version.rs`) has passed it. There is no
`MIN_VERSION_*` constant for this and it does not need one: those gate behavior at runtime, and
nothing here does — the write is unconditional, so no worker meets the column's absence until
someone follows the steps below.

Removal still takes two releases: the last readers are this codebase's own, and a rolling deploy
runs both versions at once.

## Release A — code only, column untouched

1. `workspaces_export.rs`'s schedule `SELECT`. The only reader left, and the only one that fails
   at runtime rather than at compile time, since its column list is dynamic SQL. Drop `email`
   from that list and derive the address, as `get_schedule` does.
2. The writes in `windmill-api-schedule` (`create_schedule`, `edit_schedule`), the clone in
   `workspaces.rs`, and the EE ducklake-maintenance upsert.
3. The `UPDATE schedule SET email` sweep in `change_user_email`.

## Release B — once every replica runs A

4. `ALTER TABLE schedule DROP COLUMN email`.

`ScheduleWithEmail::email` stays: it is a `required` field of the `Schedule` response schema, and
by then every path fills it by deriving, so only its source changes.

Dropping the column in release A would break the replicas still on the release before it, which
is the same rolling-deploy hazard that made the column worth keeping in the first place.
