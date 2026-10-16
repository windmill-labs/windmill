# Fork-isolated workspace object storage

A fork clones its parent's `large_file_storage` settings, so without isolation it reads and
writes the parent's objects: a script iterating in a fork overwrites production's files. Each
storage the fork isolates (a `fork_storage_namespace` row, chosen at fork creation, default
isolated) instead maps logical keys:

- **Writes** (put, multipart, delete, copy destination) go to `__wm_forks/<fork segment>/<key>`,
  the same one-component segment DuckLake's fork namespaces use (`fork_data_dir_segment`).
- **Reads** try the fork's own key, then each ancestor's mapping, nearest first, then the root's
  raw key. They stop at the first ancestor whose storage is a different one
  (`lfs_entry_storage_ref`): its keys are not in this bucket. A shared fork maps like its parent.
- **Listings** show the workspace's own objects only, as logical keys.
- **Deletes** remove the fork's copy; an ancestor's object stays readable through it.

The mapping (`windmill_common::fork_storage::ForkStorageKeys`) is applied wherever Windmill
turns a workspace-storage key into a storage operation: the `ForkStore` object-store wrapper
(`windmill_object_store::fork_store::workspace_object_store`) for every server-side helper, and
key rewriting in the S3 proxy's raw S3 forwarding (URI key, list `prefix`/`start-after`/`marker`
and response keys, `x-amz-copy-source`). A named resource (`s3_resource_path`) is not workspace
storage and is never mapped. A workspace whose keys are mapped is refused its workspace
storage's own credentials (`s3_resource_info`, the duckdb/polars settings endpoints): a client
holding them would write past the mapping.

Not mapped, on purpose: AI sessions and volumes (their keys already carry the workspace id),
SMTP attachments (job-id keys), the storage usage count (it counts the whole bucket). Known
gaps: the worker's result-cache hash HEADs the logical key unmapped, and a read of an object
the fork does not own costs one HEAD per chain level before the GET.

## Invariants

- **`__wm_forks/` is reachable only through the workspace's own chain.** A key under it maps to
  itself when it is in the workspace's own area (writes) or an area of the chain (reads), and is
  refused otherwise; listings hide it. This is also what keeps DuckLake working: its fork data
  paths arrive already prefixed, and its defer attaches read ancestors' raw keys through the
  read chain. Every workspace gets the rule, isolating or not, so pre-existing forks' lakes keep
  working.
- **No `.` or `..` key segments**, in any workspace: the proxy forwards mapped keys in a URL,
  where they collapse and would climb out of the fork's prefix.
- **The prefix derives from the workspace id**, so an isolating fork's id cannot be changed
  (`change_workspace_id` refuses): under a new id it would see none of its objects. Nor can an
  isolated storage be repointed, since the purge targets the storage recorded at creation.
- **A shared lake needs its storage shared**: its files must land at the parent's keys, where the
  parent's catalog looks for them. Fork creation refuses the other combination.
- **Children before parents**: deleting a fork that has forks of its own is refused when anything
  in its chain isolates a storage, shared forks included. A child reads through, or writes into,
  the isolating prefix via that link, and without it the child's chain would map like the root's.
- **Deletion and same-id creation are serialized** on the `fork:<id>` advisory lock, and a purge
  skips an id that exists again: the rows are then the new fork's. The proxy caches only the raw
  client, never a mapping, so fork creation and deletion invalidate the one mapping cache.
- **Deletion always purges** the fork's area of each isolated storage, post-commit, from the
  storage identity recorded at creation: nothing can reach it once the fork is gone. Rows have
  no FK and outlive the workspace when the purge fails; creating a fork with the same id retries
  them and refuses while they cannot be purged, since a same-id fork would read them.
- Isolation is a per-fork-creation choice (`shared_storages` on create-fork), never inherited.
  Forks created before the registry existed have no rows and keep sharing.

## Requiring isolation

The `RequireIsolatedForks` protection rule on a workspace makes its forks leave nothing shared
with it, for every user the ruleset does not exempt (admins always are): no `shared_storages`,
no `shared_ducklakes`, and every data table whose database it owns cloned rather than kept as a
pointer. A data table whose database another workspace owns is left alone: that is not its data.
A clone governed from another workspace still has its own database, so it is not exempt. It is
checked against the workspace being forked, in
`create_workspace_fork` and in the git-branch pre-check (so a refused fork leaves no branch
behind), and the data table part again under the parent's `lock_fork_datatables`, which data table
edits also take: a table added between the two would otherwise reach the fork as a pointer.
