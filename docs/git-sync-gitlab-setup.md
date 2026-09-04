# Git sync with GitLab

GitLab has no equivalent of a GitHub App, so there is nothing to install and no
consent screen. What Windmill needs instead is one credential you create in
GitLab and paste once. With it, a GitLab repository gets the same managed
features an app-backed GitHub repository has: instant pull over a webhook, merge
requests opened on deploy, and a diff preview posted onto the merge request.

## The credential

Create a **project access token** on the project you are syncing (Settings →
Access tokens). It is a bot identity that outlives the person who created it,
which is what you want for a credential the instance uses unattended, and it
reaches exactly the one project.

| | |
| --- | --- |
| Scope | `api` |
| Role | Developer to push deploy branches; **Maintainer** to also manage the webhook and open merge requests |
| Expiry | Required. A group service account PAT can be non-expiring on self-managed (see below); an access token cannot |

**Use a separate token per repository.** A group access token works too and
reaches every project in the group, which is convenient for a lot of
repositories — but Windmill stores the credential per repository, and renewal
rewrites the repository it renewed for. Any other repository holding that same
token keeps the revoked one and stops syncing until you paste a new token there.
Each stranded repository says so on its card, so it is visible rather than
silent, but a token per repository avoids it entirely.

`api` is a superset: it authorizes Git over HTTPS as well, so no separate
`write_repository` is needed to clone and push, and it is also what makes the
token renewable. A `write_repository`-only token can still push, but Windmill
cannot inspect or renew it and reports that in the workspace's git sync settings.

### The identity Windmill acts as

GitLab issues an access token to a bot user it creates for it — `project_<id>_bot_…`
for a project token, `group_<id>_bot_…` for a group one — and the bot's display
name is **the name you gave the token**. That name is the byline on everything
Windmill does: the author of deploy commits, of the merge requests it opens, and
of the preview notes it writes. Name it for what it is, `windmill-sync` or
similar, rather than something only you will recognise.

Each token you create adds another bot member to the project or group. Renewal
does not — it keeps the same bot — so a repository accumulates one bot, not one
per year.

Renewal goes through GitLab's own self-rotation endpoint. Both kinds of access
token are held as their bot user's personal access token, so the token rotates
itself and Windmill never needs a credential with rights over the project or
group.

## Connecting a repository

In the resource form for a `git_repository` resource, use the **GitLab** button:
paste the instance URL and the token, pick a project from the list, and Windmill
keeps the token for you. The resource itself gets the plain remote URL
(`"url": "https://gitlab.com/group/project.git"`), with no credential in it.

The token is stored encrypted on the workspace, keyed by the resource's path, and
recorded against the repository it was issued for. Nothing reads it back out over
the API: the server attaches it when it talks to GitLab, and a sync job receives
it only against its own job token. Because it is bound to one repository,
repointing the resource's `url` at somewhere else does not carry the token along;
a repository that genuinely moved needs its token entered again.

Give the resource its final path before picking a project. The token is filed
under that path, so renaming afterwards leaves it behind.

Forks of the workspace read this one copy rather than getting their own, so
renewal reaches all of them at once and no fork holds a credential a fork admin
could read.

A URL with the token written into it keeps working, whether it sits in the
resource or in a secret variable the resource points at (`"url": "$var:..."`),
and renewal rewrites whichever of the two holds it. What cannot be renewed is a
variable held in an external secret backend, which Windmill can read but does not
own the write to; that is reported on the repository.

## Expiry and renewal

Windmill reads `expires_at` from the token itself and shows it on the repository
in the workspace's git sync settings. Within three weeks of expiry it rotates the
token through GitLab's own `POST /personal_access_tokens/self/rotate`, writes the
replacement back where the credential is stored, and verifies it. Only the token
can rotate itself, so a token without `api` (or `self_rotate`) is a permanent
warning rather than something Windmill can fix.

Only the workspace that stores a credential rotates it. A fork reading its
parent's shows the same expiry but is not itself rotatable, so one rotation
serves the whole family instead of each fork racing to renew its own copy.

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

**A relative-URL install is not supported.** GitLab can be served under a path
prefix (`https://example.com/gitlab`), and that prefix cannot be told apart from
a group of the same name: `example.com/a/b/c.git` is either group `a/b` project
`c`, or prefix `a` with group `b` project `c`. Windmill reads it as the nested
group, so on a relative-URL install it derives the wrong API base and the managed
features stay unavailable. Such a repository still syncs through its token URL,
which needs no API base.

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
