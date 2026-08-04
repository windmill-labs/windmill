# Removing `policy.on_behalf_of_email`

An app's identity is `policy.on_behalf_of`; the address beside it is a function of that
principal. `on_behalf_of_email` is no longer required — a policy carrying only a principal
executes, deriving the address — but it is still written on every save, and that is the only
thing holding it in place.

## The gate

`get_on_behalf_of` gained its derive-when-absent fallback in **1.779**. Every replica before that
*requires* the key and errors outright without it, so it would 400 every anonymous and publisher
app saved by a newer one. A rolling deploy runs both versions at once, which is why the write
stays until no replica older than 1.779 can be live — in practice, once `MIN_KEEP_ALIVE_VERSION`
(`windmill-common/src/min_version.rs`) has passed it.

There is no `MIN_VERSION_*` constant for this and it does not need one: those exist to gate
behavior at runtime or to trip the build when a constraint expires, and nothing here does either.
The key is written unconditionally, so no replica ever meets its absence until someone follows
the steps below.

## Step 1 — stop writing the key

- `stored_on_behalf_of_email` in `windmill-api/src/apps.rs`, and the `create_app` /
  `update_app_internal` call sites that store what it returns.
- The CLI and frontend workspace-deploy paths (`cli/windmill-utils-internal/src/deploy.ts`,
  `frontend/src/lib/utils_workspace_deploy.ts`). These send the address *instead of* a principal
  for a cross-workspace deploy, which is the one case where it is the only identity available —
  so this is "stop sending it once the target resolves a principal itself", not a deletion.

Policies written before this keep their key and keep being read from it; they agree with their
principal, so nothing has to strip them.

## Step 2 — drop the field

Remove `on_behalf_of_email` from `Policy` and the fallback in `get_on_behalf_of`, which then
always derives. Optionally strip the key from stored policies.

This can ship with step 1. It is written separately because step 1 alone is revertible without
touching stored data or the response schema, and because the gate above is what makes either
step safe — nothing about step 2 needs its own waiting period.

## Why the address is not derived on read

Read paths return the stored address verbatim rather than recomputing it. Deriving on read means
resolving a principal that, for a draft, is caller-controlled — which turns the read into an
oracle for addresses the caller cannot otherwise see, and leaves a principal that resolves to
nobody with no address at all. Both were live defects while the read paths did derive.
