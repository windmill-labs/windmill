# Git repo viewer — hub script

The hub script `clone_repo_and_upload_to_instance_storage`
([hub page](https://hub.windmill.dev/scripts/windmill/13968)) is published from
`windmill-integrations`
(`hub/windmill/scripts/action/13968_clone_repo_and_upload_to_instance_storage/script.ts`)
and pinned in `frontend/src/lib/hubPaths.json` as
`cloneRepoToS3forGitRepoViewer`. Hub paths are exact version pins, so editing
the script means publishing a new version and repointing that entry.

A repository connected through the GitHub App carries no credential in its
URL, so the script cannot clone it with git. For those it downloads a tarball
of the commit from `GET /w/{workspace}/github_app/repo_archive/{resource_path}`
and extracts it; the installation token stays on the server. That route is
admin-only, because the repository is named by the resource's own `url` and
whoever can write the resource controls it. Repositories that aren't
app-backed are cloned with git as before, using the credential in the URL.

The rest of this file records the upload behaviour the viewer depends on.

The repo viewer in the Windmill app expects the hub script to:

1. **Upload files in parallel** with bounded concurrency (sequential per-file
   uploads of 400+ files easily blow past any reasonable client-side timeout).
2. **Log progress** so the streaming logs visible in the viewer are useful.
3. **Write a completion marker** as the very last action of a successful run,
   so the API and frontend can distinguish a fully-populated S3 directory from
   a partial / interrupted upload.
4. **Follow symlinks that stay inside the checkout.** Both the git clone and
   the archive extraction keep a repository's symlinks as links, and
   `Dirent.isFile()` / `isDirectory()` are both false for a link, so a walk
   that only checks those drops every linked file and directory from the
   viewer. See [Symlinks](#symlinks).

The marker file the frontend looks for is `.windmill_clone_complete` at the
root of the per-commit directory:

```
gitrepos/{workspace}/{resource_path}/{commit_hash}/.windmill_clone_complete
```

The frontend passes `markerFile=.windmill_clone_complete` to the
`checkS3FolderExists` API, which only reports the folder as existing when this
exact file is present.

## `uploadDirectoryToS3`

Uploads run through a bounded-concurrency pool, and the marker is written
after the walk completes:

```ts
const UPLOAD_CONCURRENCY = 16
const CLONE_MARKER_FILE = ".windmill_clone_complete"
const MAX_SYMLINKED_ENTRIES = 20_000
const MAX_SYMLINKED_BYTES = 512 * 1024 * 1024

async function uploadDirectoryToS3(
  directoryPath: string,
  s3BasePath: string,
  workspace: string,
): Promise<number> {
  console.log(`Uploading ${directoryPath} -> ${s3BasePath}`)

  // Walk the directory once, producing a flat list of (localPath, s3Key) pairs.
  const tasks: { localPath: string; s3Key: string }[] = []
  const root = fs.realpathSync(directoryPath)
  // Real paths of the directories being descended through.
  const ancestors = new Set<string>()
  // What entries reached through a link have cost so far; see Symlinks below.
  let symlinkedEntries = 0
  let symlinkedBytes = 0
  let symlinkBudgetSpent = false
  function chargeSymlinkBudget(relPath: string, entries: number, bytes: number): boolean {
    if (symlinkBudgetSpent) return false
    symlinkedEntries += entries
    symlinkedBytes += bytes
    if (symlinkedEntries <= MAX_SYMLINKED_ENTRIES && symlinkedBytes <= MAX_SYMLINKED_BYTES) {
      return true
    }
    symlinkBudgetSpent = true
    console.log(
      `Skipping ${relPath} and every symlinked entry after it: symlinks reach more than ` +
      `${MAX_SYMLINKED_ENTRIES} entries or ${MAX_SYMLINKED_BYTES / 2 ** 20} MiB`
    )
    return false
  }
  function walk(dir: string, relDir: string, viaLink: boolean) {
    ancestors.add(dir)
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const relPath = relDir ? `${relDir}/${entry.name}` : entry.name
      const linked = viaLink || entry.isSymbolicLink()
      if (linked && !chargeSymlinkBudget(relPath, 1, 0)) continue
      let localPath = join(dir, entry.name)
      if (entry.isSymbolicLink()) {
        const link = fs.readlinkSync(localPath)
        try {
          localPath = fs.realpathSync(localPath)
        } catch (e: any) {
          console.log(`Skipping symlink ${relPath} -> ${link}: cannot resolve target (${e.code})`)
          continue
        }
        if (localPath !== root && !localPath.startsWith(root + sep)) {
          console.log(`Skipping symlink ${relPath} -> ${link}: target is outside the repository`)
          continue
        }
      }
      const stat = fs.statSync(localPath)
      if (stat.isDirectory() && ancestors.has(localPath)) {
        console.log(`Skipping ${relPath}: links back to a directory it is inside`)
        continue
      }
      if (linked && stat.isFile() && !chargeSymlinkBudget(relPath, 0, stat.size)) continue
      if (stat.isDirectory()) {
        walk(localPath, relPath, linked)
      } else if (stat.isFile()) {
        tasks.push({ localPath, s3Key: `${s3BasePath}/${relPath}` })
      }
    }
    ancestors.delete(dir)
  }
  walk(root, "", false)

  console.log(`Discovered ${tasks.length} files to upload`)

  // Bounded-concurrency upload pool.
  let nextIndex = 0
  let uploaded = 0
  let lastReport = 0
  async function worker() {
    while (true) {
      const idx = nextIndex++
      if (idx >= tasks.length) return
      const { localPath, s3Key } = tasks[idx]
      const fileContent = fs.readFileSync(localPath)
      const blob = new Blob([fileContent], {
        type: "application/octet-stream",
      })
      await wmillclient.HelpersService.gitRepoViewerFileUpload({
        workspace,
        fileKey: s3Key,
        requestBody: blob,
      })
      uploaded++
      // Throttled progress log so 400+ files don't drown the log.
      if (uploaded - lastReport >= 25 || uploaded === tasks.length) {
        lastReport = uploaded
        console.log(`Uploaded ${uploaded} / ${tasks.length} files`)
      }
    }
  }

  const workers = Array.from(
    { length: Math.min(UPLOAD_CONCURRENCY, tasks.length) },
    () => worker(),
  )
  await Promise.all(workers)

  // Write the completion marker LAST. Until this exists, the viewer treats the
  // directory as not-yet-cloned.
  const markerKey = `${s3BasePath}/${CLONE_MARKER_FILE}`
  const markerBody = JSON.stringify({
    completed_at: new Date().toISOString(),
    file_count: tasks.length,
  })
  await wmillclient.HelpersService.gitRepoViewerFileUpload({
    workspace,
    fileKey: markerKey,
    requestBody: new Blob([markerBody], { type: "application/json" }),
  })
  console.log(`Wrote completion marker: ${markerKey}`)

  return tasks.length
}
```

## Symlinks

A link is resolved with `realpathSync` and followed only when its target lies
inside the checkout's real path. A file target is uploaded under the link's own
path; a directory target is walked as if it sat there, so
`inventories/prod/group_vars -> ../../shared/group_vars` shows up in the viewer
with its files. Everything else is skipped and logged:

- **A target outside the checkout.** The repository chooses the target, and the
  checkout sits in the job's working directory next to the ssh key
  `get_git_ssh_cmd` writes (`../ssh_id_priv_0`) and the job's `args.json`. A
  link to one of those, or to `/proc/self/environ` with the caller's
  `WM_TOKEN`, would put it in storage for every reader of the resource. This
  is why the walk does not follow links the way `aws s3 sync` does.
- **A target that cannot be resolved**: a dangling link, or a link loop
  (`ELOOP`).
- **A directory that is already being walked higher up** (`loop -> .`,
  `up -> ..`). The guard holds the real paths of the current descent only, as
  `find -L` does, not every directory seen so far: a directory reachable
  through two links is uploaded under both paths, as the checkout presents it.
- **Anything reached through a link once the budget is spent.** Because a
  directory can be reached along many paths, two links to the next directory
  at each level double the tree, and a repository a few dozen links deep would
  expand past what the job can hold in memory. Every entry reached through a
  link counts against a budget of 20,000 entries and 512 MiB. It is charged
  before the link is resolved, so links that end up skipped count too, and
  neither their work nor their log lines can multiply. Past the budget, the rest
  are skipped with one log line. The checkout's own files are always uploaded.

## Notes for review

- **Concurrency level**: 16 is a starting point; tune based on instance
  storage backend. Too high will overwhelm the API; too low won't help.
- **Marker is the last write**: if any upload fails, the marker is never
  written and the viewer correctly shows the state as incomplete.
- **No deletion of stale partials**: the script overwrites the same per-commit
  paths on retry, so a partial upload + retry naturally heals. Old commit
  directories from before this patch are unreachable through the UI but still
  consume storage; an instance admin can prune them manually if desired.
- **A new pin doesn't refresh commits already uploaded**: the viewer keys
  storage on the commit hash (`gitrepos/{workspace}/{resource_path}/{commit_hash}/`)
  and only checks that the marker exists. So a commit uploaded by an earlier
  script version keeps that version's tree (hub/28905's had no symlinks) until
  the repository's head moves to a new commit, or an admin deletes that
  commit's directory.
- **Error propagation**: keep the existing `try/catch` in `main` so an upload
  failure surfaces in the job result and is shown in the new viewer error
  banner.
