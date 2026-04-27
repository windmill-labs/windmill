# Windmill Preview Workflow

Use this skill any time the user wants to **see**, **open**, **navigate to**, **visualize**, or **preview** a flow, script, or app — and any time you've just finished writing one and want to offer visual verification.

The Windmill dev page renders the flow graph / script editor, lets the user step through steps, and live-reloads on every save. It runs locally via `wmill dev` and is reached on a localhost port.

## Choosing your branch

Inspect your available tool list:
- Anything starting with `mcp__Claude_Preview__` is present → **Branch A** (embed the preview, runs through a localhost proxy, one launch entry per target).
- No such tool → **Branch B** (direct mode, hand the user a URL to open in their own browser, do **not** touch `launch.json`).

Pick one. Never start the proxy "just in case" — Branch B has no proxy involved.

## Branch A — MCP `mcp__Claude_Preview__*` tools available

This is the Claude Desktop / Claude Code preview integration exposed via MCP. Tool names start with the `mcp__Claude_Preview__` prefix.

**Each flow / script / app gets its own named entry** in the user's `.claude/launch.json` so multiple previews coexist without colliding — each entry pins a different port + path. Never reuse a generic "windmill" entry for different targets.

### Step A1 — Reuse or add a per-target entry in `.claude/launch.json`

Convention: name the entry `windmill: <wmill_path>` (e.g. `windmill: f/test/my_flow`).

- **Entry already exists** → reuse it; note its `port` for the next step.
- **Not there** → add one. Pick a port not already taken by another entry (start at 4000 and bump). Shape:

For flows / scripts:
```json
{
  "name": "windmill: f/test/my_flow",
  "runtimeExecutable": "bash",
  "runtimeArgs": ["-c", "wmill dev --proxy-port ${PORT:-4000} --path f/test/my_flow --no-open"],
  "port": 4000,
  "autoPort": true
}
```

For apps (`*__raw_app/`), `wmill app dev` is the equivalent — runs from the app folder, no `--path`:
```json
{
  "name": "windmill: f/test/my_app",
  "runtimeExecutable": "bash",
  "runtimeArgs": ["-c", "cd f/test/my_app__raw_app && wmill app dev --no-open --port ${PORT:-4001}"],
  "port": 4001,
  "autoPort": true
}
```

If `.claude/launch.json` doesn't exist yet, create it with the standard shell `{ "version": "0.0.1", "configurations": [...] }`.

### Step A2 — Invoke the MCP preview tool

Point it at the entry you just added/found. Use `http://localhost:<port>/` as the URL — the proxy's redirect at `/` is what appends the workspace ID, the auth token, and the path. Do **NOT** construct a `/dev?...` URL yourself — you don't have the workspace ID or auth token.

The MCP tool launches the configuration on demand, so you don't need to start the `wmill dev` process manually.

## Branch B — no `mcp__Claude_Preview__*` tools available

Don't touch `launch.json` and don't start the proxy. Start the dev server directly with `--no-open` in the background and hand the URL to the user.

For flows or scripts:
```bash
wmill dev --path <wmill_path> --no-open
```

For apps:
```bash
cd <app_path>__raw_app && wmill app dev --no-open --port 4000
```

Both commands print the URL on stdout. The exact line shape differs:

- `wmill dev --no-open` prints `Go to <url>` (the full remote URL with workspace, token, path baked in).
- `wmill app dev --no-open` prints `🚀 Dev server running at <url>` (the local app server).

Capture the URL with a loose match (the first `https?://...` token after startup — remote workspaces use HTTPS, local ones HTTP) and post it to the user: *"Preview is running — open `<url>` in your browser."* Don't try to construct the URL yourself.

## Both branches — keep the run alive

These commands are long-running. Start them in the background; don't block waiting. Tell the user the preview is up.

## Non-visual alternative

If the user wants a programmatic test rather than a visual one:
- Flow: `wmill flow preview <path> -d '<args>'`
- Script: `wmill script preview <path> -d '<args>'`

Both print the job result, are safe to run yourself, and don't deploy.

## Anti-patterns to avoid

- ❌ Reusing a single generic `launch.json` entry for every preview target. Each flow/script/app gets its own named entry on its own port — that's how multiple sessions coexist without one preview clobbering another.
- ❌ Mutating an existing entry's `--path` to retarget it. Add a new entry instead.
- ❌ Constructing `http://localhost:<port>/dev?path=<X>` yourself. The proxy's `/` redirect is what appends the workspace ID and auth token; bypassing it gives a broken page. Always use `http://localhost:<port>/`.
- ❌ Editing `.claude/launch.json` in Branch B. Direct mode prints the URL — just relay it.
- ❌ Starting the proxy when no `mcp__Claude_Preview__*` tool is available. Direct mode is correct then — the proxy is overhead with no embedder to use it.
- ❌ Starting `wmill dev` in the foreground (you'll hang). Always background.
- ❌ Listing both "open in IDE pane" and "open in browser" as a menu — pick one based on context.
