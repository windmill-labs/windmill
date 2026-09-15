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

All under `windmill_ai_sessions/{w_id}/g{generation}/{sha256(email)}/` in the workspace's
primary storage (the generation is what a key rotation moves, see below).
The listing reads only `index/`: one object per session whatever the session holds, so a
session with many chats cannot crowd newer ones out of a bounded scan, and its
`last_modified` is the session's `updated_at`. Written last, and only by an entry no unsent
part follows (a session split over several entries says `partial` on all but the last), it
lists a session only once a whole push landed; the parts of a session after a failed one are
not written either, on the server within one push and on the client across pushes, so the
marker on the last part never lists a session missing a chat, and a new session whose last part
never lands is not listed at all. A push of the session whole (no sync row, or a stale one)
says `whole` on every part and opens with the head on the first; its last part writes the
marker only once the head is there. An incremental part rides on a listed session, and the
server refuses it with `needs_whole`, writing nothing, when none is listed (a removal deletes
the marker first, and a whole push from another device lists nothing until its last part),
rather than write a marker over a session missing what earlier parts or earlier pushes
carried. A push and a removal of one session are serialized on the server by a Postgres
advisory lock keyed on the session's prefix, so the two never interleave object by object.

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
(`set_encryption_key`) does not re-key the backups the way it re-encrypts the workspace's
secrets. The objects live under a prefix named by a generation
(`workspace_settings.ai_sessions_backup_generation`) that the rotation bumps in the
transaction committing the new key; once committed, the routes read and write under the new
generation's prefix, the storage identity the answers carry (`storage_id`, below) changes
with it, so every browser marks its sync rows stale and pushes its sessions whole again
there, and every older generation, which nothing writes to any more, is deleted off the
request at leisure (`windmill-api-workspaces/src/ai_session_backups.rs`). Sessions no browser
holds any more are lost. A generation is never reused, so no deletion, however late, can
touch live objects; a rotation that fails before its commit bumps nothing and deletes
nothing; two rotations racing serialize on the key row; the same key set again bumps
nothing. A rotation is rare, and the alternative,
rewriting every object in place while pushes, restarts, storage switches and further
rotations race the rewrite, is where the complexity would be; with this, nothing but the
current key ever reads an object. The
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
remote state. Only a user-initiated `deleteSession` removes the backup; the next push from
another device that still has the session is refused with `needs_whole` (nothing of it is
written), its row goes stale without a backoff, and that device's next flush sends the session
whole; the workspace-lifecycle
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

Every answer names the storage it came from (`storage_id`, a hash of what locates the objects,
endpoint, region and bucket, not the credentials, which rotate) and the backup generation a
key rotation bumps (`backup_generation`). A sync row records both, and a row naming another
storage or generation goes stale and its session is marked again: a workspace pointed at a
new bucket, or whose key was rotated, holds nothing, and the server looks nowhere else, so
the next flush carries the session whole. A switch leaves the old copy where it was, so the
row rewritten under the new storage records the old one (`alsoIn`, one entry per storage
the workspace was on), and a removal is done only once every storage holding a copy answered
it, whatever the generation (a rotation deleted the older generation's copy anyway): each
answer narrows the row to the storages still holding one, and the mark waits for them to
answer, so a switch back never brings a deleted session back. That includes the rows a flush has just written, when a later answer of the
same flush names another storage or the session was pushed in part on top of a row from the
old one; a session whose own parts were answered from different storages is not settled at
all. The listing a restore starts with runs the same check, so a storage switch is noticed at
the first push after it or on the next page load, whichever comes first.

## Limits

Push bodies are packed to about 8 MB (UTF-8 bytes as sent), at most 100 entries, 200 removals and
4000 pieces each (the server's caps, with 32 MB on the body, and 100 chats, 500 images or 1000
deletes per entry, since every piece is an object-store call); an entry that
outgrows the target is split into chat-only parts (the artifacts and deletes on the last, the
head on the last too for an incremental push and on the first part, whatever it carries, for a
push of the session whole), and deletes
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
would not fit is deferred unless it is the first of the answer, in which case it comes in
pages: the answer carries what fits in key order (at least one object, so every page makes
progress) and names where the next picks up (`next`, a cursor the browser sends back as
`resume` with that session alone). The browser writes each page's pieces as it arrives, over whatever
an earlier restore cut short had staged (the session is absent locally, so its pieces have no
local edits to keep, and the backup may have moved on), and the record, which is what makes
the session visible, only with the last page. A restore in progress keeps a staging row for
the session (the ids of every chat, image, artifact and version it wrote), which outlives it
if it is cut short; the next restore deletes the staged pieces the backup no longer has, by id
and never by clock, before the record lands (once the record is there no restore looks at the
session again, and a flush would push them back), and a prune that could not run leaves the
session, its pieces and its staging row for the restore after. A restore holds the user's tab
lock while it runs, so two tabs cannot each write the same absent session's pieces over the
other's, and runs only where Web Locks exist (a secure context: https, or localhost); on a plain
http origin the browser still backs up, and its sessions come back on a secure one. A
restore never writes an older record over a newer
one; between pages it holds nothing but the sync
row being assembled, whose chats also admit the images of a later page. Every page carries a
fingerprint of the session's listing (`listing`), taken before anything of the page is
listed or read, so an object landing after it is in the next page's; a session whose
fingerprint moved between two of its pages (a chat added by another device could sort before
the cursor and be missed) starts over, up to three times, then waits for the next restore. An object that grew
since the listing (a push replaced it) ends its page just before it and the answer names that
spot, so the next page sizes it anew rather than the session being imported without it. A pull sees every key of a session's listing but keeps the 5000 smallest
past its cursor (a page is defined by key order, and the store promises none), so a session
grown without bound by valid pushes cannot grow the answer's memory through its metadata
either; removing a prefix and a rotation's deletion stream their listings. `list` scans at most 50 000 index markers, keeps the newest 500 as it goes and answers with
them (`truncated` says when there were more); the restore takes 50 of them. Every read checks the object's size before buffering it: one larger than any push writes
(32 MB) is planted, whatever its listing said, and skipped, since whoever holds the bucket's
credentials can put anything at a predictable key; one larger than its listing said grew
since (a push replaced it) and ends its page, for the next pull to size anew. A dirty mark that cannot be written
(localStorage full) records its bump on the session's sync row instead (`extraV`, counted
with the mark's counter and kept by every row write, so a push in flight cannot retire it);
a session without a row is live without a mark, and marked again by every load's backfill.
A mark or removal for another user (a write that landed after a switch) reaches that user's
rows through a connection of its own, since the shared handle follows the current user. Nothing is read past the budget, whatever a session holds. A
restore takes the newest 50 sessions per workspace: every visible session gets a runtime, and
each runtime's history load reads the whole chat store. On CE the push checks the storage quota
and bumps usage by bytes written (an over-count on overwrites; the periodic recount settles it).
