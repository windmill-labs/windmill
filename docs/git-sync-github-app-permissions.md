# Windmill GitHub App: why the new permissions

Windmill is moving the git sync work it used to ask you to run as a GitHub Action
into the app itself, so two-way sync works out of the box. That needs a few more
permissions. They are all scoped to the repositories you install the app on, and
they do not grant any new access to your code beyond what the app already has.

| Permission (Read and write) | What it enables |
| --- | --- |
| Repository webhooks | Create a webhook so pushes deploy to your Windmill workspace instantly, instead of polling on a timer. |
| Pull requests | Open the promotion / fork pull requests for you, replacing the `gh pr create` GitHub Action. |
| Checks | Post a "Windmill diff" check on a pull request showing what it would change in the workspace. |

Windmill creates a per-repository webhook on each connected repo and sets its
events (`push`, plus `pull_request` for checks) itself, so you do not need to
change the app-level "Subscribe to events" list. The events are only available
because the permissions above are granted.

Approving is safe and reversible. Existing sync keeps working unchanged while the
update is pending, and Windmill falls back to polling for any repository where a
permission has not been granted yet.

## Self-managed apps (GitHub Enterprise Server)

The features work identically through a self-managed app: every API call uses the
app's own endpoint (`https://<ghes-host>/api/v3`). The difference is that there is
no update to approve, since you own the app: grant the three permissions above in
your app's settings (Settings → Developer settings → GitHub Apps → Permissions &
events) and accept the permission update on the installation. Webhooks are still
created per repository by Windmill, so the app-level "Subscribe to events" list
needs no changes, and your Windmill base URL only needs to be reachable from the
GHES host, not from the public internet.
