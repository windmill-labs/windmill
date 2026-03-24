# Dev Preview

Preview Windmill scripts and flows locally via `wmill dev`.

## Setup

Start the dev server (includes a localhost reverse proxy):

```bash
wmill dev
```

This starts:
- A file watcher on the local repo
- A reverse proxy on `localhost:3100` forwarding to the remote Windmill instance
- A dev WebSocket on `/ws_dev` for live file updates

## Preview a specific file

Open in browser or Claude Preview:

```
http://localhost:3100/dev?path={wm_path}&local=true
```

Where `wm_path` is the Windmill path derived from the local file:
- **Scripts**: strip the file extension — `u/admin/my_script.ts` → `u/admin/my_script`
- **Flows**: strip the `.flow/` suffix — `f/my_flow.flow/flow.yaml` → `f/my_flow`

The `path` parameter locks the preview to that file. Other file changes are ignored.

## Switch files

Navigate to a new URL with a different `path` param.

## Legacy mode

Without the `path` parameter, the dev page shows the last-edited file (original behavior).
