# Removing `policy.on_behalf_of_email`

An app's identity is `policy.on_behalf_of`; the address beside it is a function of that
principal. `on_behalf_of_email` is no longer required — a policy carrying only a principal
executes, deriving the address — but it is still written on every save, because a replica below
`MIN_VERSION_DERIVES_APP_POLICY_EMAIL` (`windmill-common/src/min_version.rs`) errors outright
when the key is absent, and a rolling deploy runs both versions at once.

When that constraint stops compiling, no supported replica requires the key.

## Release A — code only, key still written

1. `stored_on_behalf_of_email` in `windmill-api/src/apps.rs`, and the `update_app_internal` /
   `create_app` call sites that store what it returns.
2. The CLI and frontend workspace-deploy paths that send an address for a cross-workspace
   deploy (`cli/windmill-utils-internal/src/deploy.ts`,
   `frontend/src/lib/utils_workspace_deploy.ts`). They send the address *instead of* a
   principal, which is the one case where it is the only identity available — so this step is
   really "stop sending it once the target can resolve a principal itself", not a deletion.

## Release B — once every replica runs A

3. Drop the key from `Policy` and stop reading it in `get_on_behalf_of`, which then always
   derives. A migration can strip it from stored policies, or leave it as an ignored field.

Removing it in release A would break the replicas still on the release before it: their
`get_on_behalf_of` requires the key and 400s every anonymous and publisher app without it.

## Why the address is not derived on read

Read paths return the stored address verbatim rather than recomputing it. Deriving on read means
resolving a principal that, for a draft, is caller-controlled — which turns the read into an
oracle for addresses the caller cannot otherwise see, and leaves a principal that resolves to
nobody with no address at all. Both were live defects while the read paths did derive. The
address a policy carries is written from its principal on every save, so the stored value is
already the derived one for anything this release wrote.
