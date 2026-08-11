# npm proxy: where cached registry content belongs

Design rationale for `backend/windmill-api-npm-proxy`. Read this before changing what the
proxy caches or where it keeps it.

## Context

The npm proxy serves an instance's configured private registry to two browser-side
consumers: the raw app editor's package installer, and the script editor's type
acquisition (ATA). Both are chatty by nature. Opening one script asks for a package's file
tree and then for every `.d.ts` in it, which for `@types/lodash` is several hundred
requests against one archive.

Everything the proxy caches today lives in the API process's memory:

| Cache | Budget | TTL | Holds |
|---|---|---|---|
| `PACKAGE_JSON_CACHE` | 64 MiB | 60 s | registry documents (packuments) |
| `TARBALL_CACHE` | 256 MiB | 300 s | per package version: the compressed archive, its file list, and the retained `.d.ts` and manifest |

That is roughly 320 MiB of steady state, plus a transient of `reads in flight x
RETAINED_BYTES` while a read builds its map before the cache weighs it. On a 2 GB API
container the steady state alone is 16% of the box, and the transient scales with request
concurrency rather than with any budget.

## The two things being cached are not the same shape

This is the whole design.

**Packuments** are small (10 to 100 KB in npm's abbreviated form), hot, and mutable: a new
version publishes and the document changes. They exist to save a registry round trip, so
moving them to a store that itself costs a round trip buys nothing. They belong in memory,
with a short TTL, and their budget can be small.

**Package archives** are large (10 KB to 50 MB), cold between sessions, and immutable: a
published version's tarball never changes. They are re-fetchable from the registry at any
time. This is blob-shaped data, and memory is the wrong tier for it.

## Decision

Keep packuments in memory. Move archive-derived content to disk, backed by the instance
object store when one is configured.

### Store extracted files, not the archive

Storing the tarball as the cached unit is the obvious move and it is wrong: every `/file`
request would still download and inflate the whole archive to reach one `.d.ts`. For
`aws-sdk@2.1692.0` that is 50 MB of inflation per declaration, and ATA asks for 442 of
them.

Store instead what the endpoints actually serve, which is already what the current code
chooses to retain:

```
<cache root>/npm_proxy/<registry hash>/<package>/<version>/
  manifest.json      the file list and the resolved entry point
  files/<path>       each retained .d.ts and the package.json
```

`/filetree` then reads one small file and `/file` reads one small file. No inflation on
the request path at all, and nothing large resident.

The extraction rule stays as it is: the manifest plus `.d.ts`, because that is exactly what
`ata/index.ts` reads (`treeToDTSFiles` filters to `.d.ts`, and the caller loop fetches each
dependency's `package.json`). Anything else in the archive is walked past. A request for a
path outside that set still works, by walking the archive once on demand.

### Tiering

Follow the order `windmill-worker/src/global_cache.rs` already uses for dependency caches:

1. **Local disk** first. Fast, per replica, survives nothing but a container restart.
2. **Instance object store** on a local miss, which then populates disk. Shared across
   replicas, survives restarts.
3. **Registry** on a miss in both, which populates disk and the object store.

The object store layer stores a tar of the extracted subset rather than individual objects.
That subset is small (kilobytes to a few MB, not the 50 MB archive), one PUT and one GET
per package version instead of hundreds, and it matches `build_tar_and_push` /
`pull_from_tar` which already exist.

### Keys are scoped to the registry

The cache key must include the registry identity, not just package and version. The `npmrc`
instance setting can be changed to point at a different registry that serves a different
artifact under the same name and version. Hash the resolved registry URL into the path.

Hashing it in is only half of it: a miss has to resolve the setting once and use that one
snapshot for the key, the packument and the tarball. Resolving a second time lets the
setting change in between, and since the tarball's origin is only checked by host, two
repositories on one host will write the second registry's files under the first one's key,
where immutability then keeps them.

Immutability means the archive tiers need no TTL. The packument cache still does, since
that is what discovers new versions.

## What already exists

Little of this is new machinery.

- `windmill_object_store::get_object_store() -> Option<Arc<dyn ObjectStore>>` is the
  instance store accessor, and returning `Option` is exactly the "no object store
  configured" branch.
- `windmill_common::worker::extract_tar` and `atomic_publish_dir` handle unpacking and the
  concurrent half-extracted directory problem. `atomic_publish_dir` has a thundering-herd
  test already.
- `windmill_common::worker::ROOT_CACHE_DIR` is the existing on-disk cache root convention.
- `global_cache::load_cache` / `save_cache` / `build_tar_and_push` / `pull_from_tar` are the
  disk-then-object-store pattern in worker form. They live in `windmill-worker`, so either
  the shared parts move to `windmill-common` or the proxy grows a small equivalent.

## CE does not get the object store tier

`windmill-object-store` is compiled entirely under `#[cfg(feature = "parquet")]`, and every
call site gates on `#[cfg(all(feature = "enterprise", feature = "parquet"))]`. Instance
object storage is therefore an EE capability.

This matters more than usual here: the users most likely to configure a private npm
registry are self-hosted, and a good number of them are on CE. The disk tier is not a
nicety for them, it is the whole feature. Disk has to work well on its own, and the object
store has to be a pure addition on top.

## Failure modes disk has and memory does not

- **Eviction.** A sweep trims to 2 GiB, least recently used first, rate limited to once per
  ten minutes or 256 MiB written, whichever comes first. It measures allocated blocks
  rather than file lengths: a package of many tiny declarations costs a block each and
  would otherwise look nearly free.
- **Recency is `mtime`, marked explicitly.** Cache volumes mount `relatime`, so access time
  does not move on a read and an LRU built on it is silently a FIFO. A cache hit touches
  the manifest instead.
- **ENOSPC and read-only cache directories degrade, they do not fail.** Every write is best
  effort; when one fails the archive is walked a second time into an in-memory map for that
  request. Only on that path, since building the map alongside a successful disk fill would
  put back the per-request heap this exists to remove.
- **Debris, distinguished from live writers.** A killed writer leaves a `.tmp.` directory
  that nothing will read. The sweep removes those, but only once they are an hour old, and
  never treats one as an eviction candidate: concurrent cold fills are exactly when a sweep
  is most likely to run, and deleting a scratch directory underneath its writer would
  publish a package missing whatever it had already written.
- **The live path holds a whole package or nothing.** Publishing renames a fully written
  temp directory into place, so a partial one never looks complete. Losing that rename is
  success only when the destination has a manifest, which means another writer published the
  same immutable content; a destination without one is debris, and it is renamed aside and
  replaced. Eviction is the same rename in reverse rather than a recursive delete in place.
- **Transfers stream, and everything that scales with a package stays off the runtime.**
  Pulls go to a temp file, and unpacking, measuring and publishing all run on a blocking
  thread; pushes build a tar on disk and upload it with a bound on parts in flight, not
  bytes queued. Buffering either, or walking a tree of thousands of declarations on a
  runtime worker, would put back what this exists to remove.
- **A malicious archive is not a broken disk.** An entry whose path escapes its directory
  is refused before either sink, rather than degrading the way an unwritable cache does.

- **Eviction races the reads.** A package can be swept between the lookup and the manifest
  read, so a missing manifest is treated as a miss and repopulated rather than raised. A
  missing file already falls through to the on-demand walk.

- **A recursive delete has no atomic point, so nothing deletes on the live path.**
  `remove_dir_all` unlinks in readdir order, and either half of a package it can leave
  behind is permanent, which is why publishing and eviction both move whole directories with
  `rename`:
  - *Files gone, manifest left* reads as a complete package forever. The manifest
    short-circuits the lookup, so the object store is never consulted, and every declaration
    it names falls through to the on-demand walk, re-fetching and inflating the archive per
    request.
  - *Manifest gone, files left* reads as a miss, but `rename` refuses a non-empty
    destination, so every tree published afterwards is discarded in favour of the debris and
    the path answers 500 with a valid copy sitting in the bucket.

  A sweep that cannot rename a candidate aside leaves it whole rather than falling back to
  deleting it where it stands: the cap is a bound the next sweep rechecks, and a package
  half-deleted under it is not.

Still open: **single flight**. Concurrent misses for the same package each fetch and
extract. Publishing by rename and per-pull scratch directories make that safe, just
wasteful.

## Alternative considered: just lower the budgets

The constants were sized for throughput without weighing a 2 GB container. Cutting
`TARBALL_CACHE_BYTES` to 64 MiB and `PACKAGE_JSON_CACHE_BYTES` to 16 MiB is a four line
change with no new failure modes, and the only cost is re-fetching large packages more
often.

It does not fix the transient, which scales with concurrency rather than with the budget,
and it does not survive a restart or share across replicas. Take it if the storage work is
not worth doing now; it is strictly a smaller version of the same trade.

## Status

Implemented in `store.rs`. The archive cache is gone from memory: extraction streams each
entry straight to the package directory, so a read holds one entry rather than the whole
retained set, and `PACKAGE_JSON_CACHE` at 16 MiB is all that is left in the heap.

Two things settled during implementation and are worth keeping in mind before changing it:

- **A warm package must not cost a round trip.** The registry is resolved from settings, and
  the packument is fetched only on a cache miss. Fetching it first, to reach the tarball URL,
  made a fully cached package still hit the registry once per minute.
- **The manifest is exempt from the retention budget.** `next` orders its `package.json`
  3748th, so a budget spent on the declarations ahead of it would leave the entry point
  defaulted, which is a wrong answer rather than a slow one.
