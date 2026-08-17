# Git repo viewer — hub script patch

This documents the changes that need to be applied to the hub script
`clone_repo_and_upload_to_instance_storage` (currently published as
`hub/28182/clone_repo_and_upload_to_instance_storage` and referenced from
`frontend/src/lib/hubPaths.json` as `cloneRepoToS3forGitRepoViewer`).

The repo viewer in the Windmill app expects the hub script to:

1. **Upload files in parallel** with bounded concurrency (sequential per-file
   uploads of 400+ files easily blow past any reasonable client-side timeout).
2. **Log progress** so the streaming logs visible in the viewer are useful.
3. **Write a completion marker** as the very last action of a successful run,
   so the API and frontend can distinguish a fully-populated S3 directory from
   a partial / interrupted upload.

The marker file the frontend looks for is `.windmill_clone_complete` at the
root of the per-commit directory:

```
gitrepos/{workspace}/{resource_path}/{commit_hash}/.windmill_clone_complete
```

The frontend passes `markerFile=.windmill_clone_complete` to the
`checkS3FolderExists` API, which only reports the folder as existing when this
exact file is present.

## Replacement `uploadDirectoryToS3` implementation

Replace the recursive sequential `uploadDirectoryToS3` function with a
batched-concurrent implementation, and write the marker after the walk
completes:

```ts
const UPLOAD_CONCURRENCY = 16
const CLONE_MARKER_FILE = ".windmill_clone_complete"

async function uploadDirectoryToS3(
  directoryPath: string,
  s3BasePath: string,
  workspace: string,
) {
  console.log(`Uploading ${directoryPath} -> ${s3BasePath}`)

  // Walk the directory once, producing a flat list of (localPath, s3Key) pairs.
  const tasks: { localPath: string; s3Key: string }[] = []
  function walk(dir: string, s3Path: string) {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const fullPath = join(dir, entry.name)
      const s3Key = s3Path ? `${s3Path}/${entry.name}` : entry.name
      if (entry.isDirectory()) {
        walk(fullPath, s3Key)
      } else if (entry.isFile()) {
        tasks.push({ localPath: fullPath, s3Key })
      }
    }
  }
  walk(directoryPath, s3BasePath)

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
}
```

## Notes for review

- **Concurrency level**: 16 is a starting point; tune based on instance
  storage backend. Too high will overwhelm the API; too low won't help.
- **Marker is the last write**: if any upload fails, the marker is never
  written and the viewer correctly shows the state as incomplete.
- **No deletion of stale partials**: the script overwrites the same per-commit
  paths on retry, so a partial upload + retry naturally heals. Old commit
  directories from before this patch are unreachable through the UI but still
  consume storage; an instance admin can prune them manually if desired.
- **Error propagation**: keep the existing `try/catch` in `main` so an upload
  failure surfaces in the job result and is shown in the new viewer error
  banner.
