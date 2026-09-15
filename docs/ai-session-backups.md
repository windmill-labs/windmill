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
bumped on every write; a push retires it by recording the counter it covered on the session's
sync row rather than deleting the mark, since two localStorage calls cannot compare-and-delete
and a bump landing between them would be lost; retired marks are not reclaimed (one small key per
session ever backed up), and the marks of unsent drafts and of workspaces that are off stay too,
each costing one lookup per flush. Only a session gone from the store has its mark deleted. Losing the last
seconds of a device that never comes back is accepted; a tab that closes normally keeps its marks.
A signal names the user whose store the write landed in (read off the store's scoped name), so
a write that completes after the logged-in user changed marks that user's session, for their
next load, rather than the current user's.

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
| index marker | `index/{sid}` | last, by the part that completes a push of the session (empty) |

All under `windmill_ai_sessions/{w_id}/{sha256(email)}/` in the workspace's primary storage.
The listing reads only `index/`: one object per session whatever the session holds, so a
session with many chats cannot crowd newer ones out of a bounded scan, and its
`last_modified` is the session's `updated_at`. Written last, and only by an entry no unsent
part follows (a session split over several entries says `partial` on all but the last), it
lists a session only once a whole push landed; the parts of a session after a failed one are
not written either, on the server within one push and on the client across pushes, so the
head on the last part never lists a session missing a chat, and a new session whose last part
never lands is not listed at all.

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
object that does not decrypt for its reader is treated as absent. Rotating the workspace key
(`set_encryption_key`) re-keys every backup object off the request, the way it re-encrypts the
workspace's secrets, unless `skip_reencrypt` was asked for; the user segment of an object's key
is the cipher suffix, so the walk needs no email (`windmill-api-workspaces/src/ai_session_rekey.rs`).
The rotation records the key it replaced (`ai_session_backup_rekey`) in its own transaction,
on every build (the walk needs `parquet`, the record does not, so a rotation on a build
without the feature loses nothing);
until the walk has found every object under the current key and dropped that record, the read
path decrypts with the recorded keys too and every use of the backups starts the walk again, so
a server restart mid-walk leaves nothing unreadable and nothing under the old key for good. The
walk writes each object conditionally on the version it read (`PutMode::Update`): a push or a
delete landing in between used the current key already, and rewriting over it would bring back
what it replaced. The filesystem store has no conditional writes and keeps that window. A push
that started under the key a rotation replaced may write pieces after the walk listed the
bucket; it reads the key again once its writes are done and fails whole (503) if it moved, so
the browser sends those pieces again under the new key rather than settle them. The
server builds every key from ids it validated
(`[A-Za-z0-9_-]{1,64}`) and the caller's own email; the client never names a key, and the
workspace storage permission rules are not consulted (the same stance as volumes). Only an
unscoped user token may reach the routes: a job token can carry an `on_behalf_of` identity and
every scoped token (guest, embed, app policy, MCP) was minted for something narrower.

The backup is keyed by the email like the browser's own stores are (`userScopedDb` scopes
IndexedDB by it): a user whose email changes starts from an empty history on both sides, and
the objects under the old hash stay in the bucket unread. Carrying them over would need a
server-side re-key (decrypt with the old suffix, encrypt with the new, move every object) in the
email-change flow, which this design leaves out.

An image is accepted only as a base64 data URL of at most 4 MB and stored verbatim, so it
serializes back into a pull answer at its stored size; anything JSON would escape could grow
several times and defeat the pull budget.

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
in the bucket. A session moved to another workspace is pushed whole into the new one, and once
that push has landed the copy in the old one gets a removal mark of its own, retried
independently until it lands, even when the old workspace's backups are off at the time (they
may hold the copy still). Filing the removal only after the new copy is acknowledged keeps the
session backed up somewhere at every point.

A restore writes a session's artifacts and chats before its record, and records nothing for a
session whose pieces could not be written: recording it would let the next flush push the
half-empty local state over the backup.

Every answer names the storage it came from (`storage_id`, a hash of what locates the objects:
endpoint, region and bucket, not the credentials, which rotate). A sync row records it, and a
row naming another storage goes stale and its session is marked again: a workspace pointed at
a new bucket holds nothing, and the server looks nowhere else, so the next flush carries the
session whole. That includes the rows a flush has just written, when a later answer of the
same flush names another storage or the session was pushed in part on top of a row from the
old one; a session whose own parts were answered from different storages is not settled at
all. The listing a restore starts with runs the same check, so a storage switch is noticed at
the first push after it or on the next page load, whichever comes first.

## Limits

Push bodies are packed to about 8 MB (UTF-8 bytes as sent), at most 100 entries, 200 removals and
4000 pieces each (the server's caps, with 32 MB on the body, and 100 chats, 500 images or 1000
deletes per entry, since every piece is an object-store call); an entry that
outgrows the target is split into chat-only parts with the head riding on the last, and deletes
past the per-entry cap are carried over to the next push, which the session stays marked for. A chat above
16 MB or a session's artifacts above 8 MB are left out with a console warning; a chat that grew
past the cap after it was backed up has its copy deleted, so a restore never presents the old
transcript as the current one. A 413 fails only the sessions of that request. A
request the server refuses (any other 4xx but 404/403/409) stops the backup for the page but keeps
the marks and the sync state, so the next load tries again; a session the server reports it could
not store stays marked and is retried with backoff. A move files the old workspace's removal
mark before recording the new copy's row, so a mark that could not be written leaves the move
to be planned again. A workspace that answers `enabled: false`
marks its sync rows stale (the next push after storage returns carries every session whole,
since a new storage may be a new bucket) and leaves its dirty marks where they are (a move into
it must still remember the old copy); it keeps the removal marks of sessions that had been backed
up, so one deleted while backups are off does not come back once they are on, and drops the
removals of sessions never backed up from this browser, so a storage-less instance does not
collect one mark per deleted session forever. A user delete whose removal mark cannot be written
(localStorage full) is carried by the session's sync row instead (`removed`), which the flush
and the restore read like a mark; a session without a row yet (its first push may be in
flight) gets a row saying only that, and every row write keeps a removal filed meanwhile, so
the push's own row cannot erase it. Pull bodies are
capped at 64 KB. Pull answers up to 20 ids within a 32 MB
budget: a session's size is known from the listings before anything of it is read, one that
would not fit is deferred unless it is the first of the answer, in which case its chats and
artifacts are read in listing order only while they fit, and images beyond the budget are left
out (they hydrate to placeholders). The listings themselves stop at the budget and at 5000
objects per session, so a session grown without bound by valid pushes cannot grow the answer's
memory through its metadata either; removing a prefix and the re-key walk stream their
listings too. `list` scans at most 50 000 index markers, keeps the newest 500 as it goes and answers with
them (`truncated` says when there were more); the restore takes 50 of them. Nothing is read past the budget, whatever a session holds. A
restore takes the newest 50 sessions per workspace: every visible session gets a runtime, and
each runtime's history load reads the whole chat store. On CE the push checks the storage quota
and bumps usage by bytes written (an over-count on overwrites; the periodic recount settles it).
