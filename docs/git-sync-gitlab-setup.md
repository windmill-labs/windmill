# Git sync with GitLab

GitLab has no equivalent of a GitHub App, so there is nothing to install and no
consent screen. What Windmill needs instead is one credential you create in
GitLab and paste once. With it, a GitLab repository gets the same managed
features an app-backed GitHub repository has: instant pull over a webhook, merge
requests opened on deploy, and a diff preview posted onto the merge request.

## The credential

Create a **group access token** on the group that owns the project (Settings →
Access tokens), or a **group service account** and a personal access token for
it. Either one is a bot identity that outlives the person who created it, which
is what you want for a credential the instance uses unattended.

| | |
| --- | --- |
| Scope | `api` |
| Role | Developer to push deploy branches; **Maintainer** to also manage the webhook and open merge requests |
| Expiry | Required for a group access token; a group service account PAT can be non-expiring on self-managed (see below) |

The `api` scope is what makes the token rotatable, so Windmill can renew it
before it expires. A `write_repository`-only token can still push, but Windmill
cannot inspect or renew it and reports that in the workspace's git sync settings.

## Connecting a repository

In the resource form for a `git_repository` resource, use the **GitLab** button:
paste the instance URL and the token, pick a project from the list, and Windmill
stores the whole remote URL, credential included, in a **secret variable** and
points the resource at it (`"url": "$var:u/you/gitlab_group_project_url"`).

The variable indirection is what makes renewal possible: when Windmill rotates
the token it rewrites that one variable, and everything referencing it keeps
working. A URL pasted directly into the resource also syncs, but nothing can
renew it.

## Expiry and renewal

Windmill reads `expires_at` from the token itself and shows it on the repository
in the workspace's git sync settings. Within three weeks of expiry it rotates the
token through GitLab's own `POST /personal_access_tokens/self/rotate`, writes the
replacement back to the variable, and verifies it. Only the token can rotate
itself, so a token without `api` (or `self_rotate`) is a permanent warning rather
than something Windmill can fix.

Rotation is deliberately never retried. GitLab revokes the old token the instant
it issues the replacement, and presenting an already-rotated token to `/rotate`
again is treated as reuse: it revokes **the whole token family, including the
live replacement**. So a rotation that succeeded at GitLab but failed to persist
is surfaced as an error to act on, not retried.

Non-expiring tokens are possible only for a **group service account PAT** on
self-managed, with `require_personal_access_token_expiry` turned off in the
instance's application settings. A group access token is always rejected without
an `expires_at`.

## What each managed feature needs

| Feature | Needs |
| --- | --- |
| Instant pull | A project hook Windmill creates, so Maintainer; and a Windmill base URL GitLab can reach |
| Merge requests on deploy | Developer, plus the `api` scope |
| Diff preview on a merge request | The project hook, plus permission to post merge request notes |

Instant pull falls back to checking the tracked branch about every minute when
the hook cannot be created or delivered, so nothing silently stops syncing.

## Self-managed differences

**Webhooks to a private network are blocked by default.** GitLab refuses to
create a hook pointing at a private or local address until an administrator
enables *Allow requests to the local network from webhooks and integrations*
(Admin → Settings → Network → Outbound requests,
`allow_local_requests_from_web_hooks_and_services`). A Windmill instance on the
same private network as GitLab needs this; without it, hook creation fails with a
"blocked" error and the repository keeps polling.

Everything else is identical: Windmill talks to `<your-gitlab>/api/v4` and needs
no inbound access of its own beyond the hook deliveries.

## The deploy preview is a note, not a pipeline status

On GitHub the preview is a check run: its own object, advisory unless the
repository makes it required. GitLab has no equivalent. Its only comparable
primitive is a commit status, and posting one has side effects Windmill will not
impose on a project:

- GitLab files the status **as a job inside whatever pipeline already covers that
  commit**, so a failed Windmill status fails the project's own pipeline, and its
  reviewers see their test suite as failed.
- `allow_failure` is ignored on the commit-status endpoint, so the status cannot
  be made advisory.
- On a commit with no pipeline it creates an `external` pipeline instead, which
  then gates merging under *Pipelines must succeed*, including while it is still
  running.

So on GitLab the preview lives entirely in a **merge request note** that Windmill
keeps up to date: it carries the workspace, the status line, the commit, a link
to the job, and the full list of changes merging would deploy. A note cannot
block a merge or change what the project's own CI reports.

The note is upserted rather than appended, so a merge request accumulates one
Windmill comment however many times it is pushed to.
