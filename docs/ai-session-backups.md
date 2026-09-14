# AI session backups

AI sessions live in the browser: the session list (`windmill-sessions`), chat transcripts and
image blobs (`copilot-chat-history`) and artifacts (`copilot-artifacts`), all per-user IndexedDB
stores. This is the design of their backup in the workspace's object storage, and the
constraints future work on either side must keep.

Backend: `backend/windmill-api/src/ai_sessions.rs` (`/w/{w}/ai/sessions/{list,pull,push}`).
Frontend: `frontend/src/lib/components/sessions/sessionMirror*.ts`.

## Why it is lazy

A session changes at the local write rate: a transcript write every 2 s while streaming, a
session-record write per new message on screen. An object in S3 is replaced whole and every PUT
is billed, so the backup deliberately does not follow that rate. Local writes only mark a session
dirty (`sessionMirrorSignal.ts`, import-free so the stores never depend on the backup). A flush
runs 15 s after the marks go quiet, at most 2 min after the first unflushed mark, when the tab is
hidden, and 10 s after load for marks a crash left behind. Marks are persisted in localStorage
(shared by the user's tabs) for that reason, one key per mark: a shared blob would let two tabs
marking different sessions at once rewrite each other's mark away. A dirty mark is a counter
bumped on every write and cleared only if it did not move during the flush. Losing the last
seconds of a device that never comes back is accepted; a tab that closes normally keeps its marks.

A flush plans and sends one session at a time, filling requests of about 8 MB as it goes, so a
first backfill of a large history never holds more than one request's worth of records and
images in memory.

## What a push carries

The pure planner (`sessionMirrorPlan.ts`) compares each piece against the marker of what was
last pushed, kept per session in the `windmill-sessions-mirror` store:

| Piece | Object | Sent when |
|---|---|---|
| session record | `sessions/{sid}/head.json` | its signature changed |
| chat | `sessions/{sid}/chats/{cid}.json` | its `lastModified` moved |
| artifacts | `sessions/{sid}/artifacts.json` | their fingerprint changed |
| image | `images/{sid}/{cid}/{iid}` | never pushed before (write-once) |

All under `windmill_ai_sessions/{w_id}/{sha256(email)}/` in the workspace's primary storage.
Images sit outside `sessions/` so one listing of that prefix enumerates a user's sessions.

The head signature leaves out `name` (a per-browser counter the sessions page routes by),
the unsent-draft fields, `workspace_root_id` (recomputed on import), and the two fields reading
a session bumps (`lastSeenCount`, `lastActivityAt`). Reading a session must never cost a push;
keep that property when adding fields to `Session`.

Unsent drafts (no `workspace_id`) and attached files (Blobs, directory handles) are not backed up.

## Encryption and access

Every object is encrypted with a key derived from the workspace key and the user
(`build_crypt_with_key_suffix` with the email hash), because workspace storage credentials are
shared far more widely than a user's transcripts: `public_resource` storages and legacy-mode
READ/WRITE hand any member the bucket. The key is per user rather than per workspace so that a
member who copies another user's ciphertext under their own prefix gets nothing from `pull`; an
object that does not decrypt for its reader is treated as absent. The server builds every key from ids it validated
(`[A-Za-z0-9_-]{1,64}`) and the caller's own email; the client never names a key, and the
workspace storage permission rules are not consulted (the same stance as volumes). Only an
unscoped user token may reach the routes: a job token can carry an `on_behalf_of` identity and
every scoped token (guest, embed, app policy, MCP) was minted for something narrower.

`push` carries `owner`, the email the browser prepared the batch for, and the server refuses a
mismatch with 409: an in-place account switch must not file one user's sessions under another's
prefix. The client captures its user at flush start and checks every store handle's name
against it for the same reason.

The feature is on wherever the workspace has primary storage, and off with
`ai_config.sessions_storage_disabled` (the `copilot_disabled` pattern: no migration, carried by
settings export and the CLI). A build without `parquet` has no routes (404), a workspace without
storage answers `enabled: false`; either turns the backup off for the page.

## Conflicts and deletion

Last write wins across devices. The head carries no manifest; `pull` lists the session's prefix
instead, so a stale device that renames or archives a session rewrites only the head and cannot
hide chats a newer device wrote. Two devices continuing the same chat still collide.

Restore brings back only sessions the browser does not have (`importSessions` is write-if-absent,
and skips ids the user deleted in this page) and never overwrites or deletes a local one from
remote state. Only a user-initiated `deleteSession` removes the backup; the workspace-lifecycle
removals (`reconcileSessionsLifecycle`, `deleteSessionsForWorkspace`) leave it, so a session
dropped by a wrong reconcile comes back on the next restore. Objects of deleted workspaces stay
in the bucket. A session moved to another workspace is pushed whole into the new one, and the
copy in the old one gets a removal mark of its own, retried independently until it lands.

A restore writes a session's artifacts and chats before its record, and records nothing for a
session whose pieces could not be written: recording it would let the next flush push the
half-empty local state over the backup.

## Limits

Push bodies are packed to about 8 MB, at most 100 entries and 200 removals each (the server's
caps, with 32 MB on the body); an entry that outgrows the target is split into chat-only parts
with the head riding on the last, and a chat above 24 MB is left out with a console warning. A
request the server refuses (a 4xx other than 404/403/409) stops the backup for the page but keeps
the marks and the sync state, so the next load tries again; a session the server reports it could
not store stays marked and is retried with backoff. A workspace that answers `enabled: false`
keeps its removal marks (only its dirty marks and sync state are dropped): a session deleted
while backups are off must not come back from the bucket once they are on again. Pull bodies
are capped at 64 KB. Pull answers up to 20 ids within a 32 MB
budget: a session's size is known from the listings before anything of it is read, one that
would not fit is deferred (unless it is the first of the answer), and images beyond the budget
are left out (they hydrate to placeholders). A
restore takes the newest 50 sessions per workspace: every visible session gets a runtime, and
each runtime's history load reads the whole chat store. On CE the push checks the storage quota
and bumps usage by bytes written (an over-count on overwrites; the periodic recount settles it).
