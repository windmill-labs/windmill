---
name: preview
description: MUST use when opening the Windmill dev page / visual preview of a flow, script, or app. Triggers on words like preview, open, navigate to, visualize, see the flow/app/script, and after writing a flow/script/app for visual verification.
---

# Windmill Preview Workflow

Use this skill any time the user wants to **see**, **open**, **navigate to**, **visualize**, or **preview** a flow, script, or app — and any time you've just finished writing one and want to offer visual verification.

The Windmill dev page renders the flow graph / script editor, lets the user step through steps, and live-reloads on every save. It runs locally via `wmill dev` and is reached on a localhost port.

## Two independent decisions

### 1. Mode: proxy or direct?

`wmill dev` runs in two modes; pick by asking what kind of URL whatever will display the preview needs.

- **Proxy** (`--proxy-port <port>`) — exposes the dev page on `http://localhost:<port>/`. Use it when the embedder you'll hand the URL to **only accepts localhost URLs** (most in-IDE / in-chat preview embedders do, because they sandbox cross-origin loads).
- **Direct** (default) — the user's browser loads the dev page from the remote workspace's HTTPS URL; the local `wmill dev` only runs the WebSocket back-channel for live reload. Use it when the URL will be opened in a regular browser tab.

Default to **direct** unless you have a specific embedder that needs localhost.

### 2. Who starts the server?

- **You start it** in the background. Spawn `wmill dev …` (or `wmill app dev …`) yourself, capture the URL it prints, do whatever's next (open a tab, hand the URL to an embedder).
- **The runtime starts it from `.claude/launch.json`.** Some runtimes (currently the Claude Desktop / Claude Code MCP preview integration — tools prefixed with `mcp__Claude_Preview__`) can read a `launch.json` configuration and launch the dev server on demand when you invoke their preview tool. **Only take this path if you actually have such a tool** — otherwise nothing reads the file and `wmill dev` never starts.

The two decisions compose. The common cases:

| Embedder | Needs localhost? | launch.json runtime? | What to do |
|---|---|---|---|
| Regular browser tab | No | n/a | Direct mode, you start it, give URL to user |
| IDE / chat preview pane that takes any URL | No | No | Direct mode, you start it, point the embedder at the printed URL |
| IDE / chat preview pane that only accepts localhost | Yes | No | Proxy mode, you start it, point the embedder at `http://localhost:<port>/` |
| Claude Desktop / Code MCP preview | Yes | Yes | Proxy mode, write a `launch.json` entry, invoke the MCP tool |

Never start the proxy "just in case" — it adds the localhost hop for no benefit when no embedder needs it.

## Starting the server yourself

Use this when no `launch.json`-aware runtime is available, regardless of mode.

For flows / scripts:
```bash
# Direct mode — gives you the remote dev-page URL
wmill dev --path <wmill_path> --no-open

# Proxy mode — gives you a localhost URL that 302s to the remote dev page
wmill dev --proxy-port 4000 --path <wmill_path> --no-open
```

For apps:
```bash
cd <app_path>__raw_app && wmill app dev --no-open --port 4000
```

Each command prints the URL on stdout. Line shapes differ:

- `wmill dev --no-open` (direct) prints `Go to <url>` with the full remote URL (workspace, token, path baked in).
- `wmill dev --proxy-port` prints `Dev proxy listening on http://localhost:<port>` — the URL to hand to an embedder is `http://localhost:<port>/`.
- `wmill app dev --no-open` prints `🚀 Dev server running at <url>` — the local app server.

Capture the URL with a loose match (the first `https?://…` token after startup) and either hand it to your embedder or relay it to the user: *"Preview is running — open `<url>` in your browser."* Don't construct the URL yourself; you don't have the workspace ID or auth token.

These commands are long-running — start them in the background, don't block waiting.

## Letting `launch.json` start the server (Claude Desktop / Code MCP only)

Take this path when **and only when** an `mcp__Claude_Preview__*` MCP tool is exposed in your tool list. Skip it otherwise — without an MCP tool reading the file, `wmill dev` never starts.

**Each flow / script / app gets its own named entry** in the user's `.claude/launch.json` so multiple previews coexist without colliding — each entry pins a different port + path. Never reuse a generic "windmill" entry for different targets.

### Step 1 — Reuse or add a per-target entry in `.claude/launch.json`

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

### Step 2 — Invoke the MCP preview tool

Point it at the entry you just added/found. Use `http://localhost:<port>/` as the URL — the proxy's redirect at `/` is what appends the workspace ID, the auth token, and the path. Do **NOT** construct a `/dev?...` URL yourself.

The MCP tool launches the configuration on demand, so you don't need to start the `wmill dev` process manually.

## Non-visual alternative

If the user wants a programmatic test rather than a visual one:
- Flow: `wmill flow preview <path> -d '<args>'`
- Script: `wmill script preview <path> -d '<args>'`

Both print the job result, are safe to run yourself, and don't deploy.

## Anti-patterns to avoid

- ❌ Writing a `.claude/launch.json` entry when no `mcp__Claude_Preview__*` tool is in your tool list. Nothing will read the file; the server never starts. Spawn `wmill dev` yourself instead.
- ❌ Starting the proxy when no embedder needs a localhost URL. Direct mode is the right choice — the proxy is overhead with no purpose.
- ❌ Reusing a single generic `launch.json` entry for every preview target. Each flow/script/app gets its own named entry on its own port — that's how multiple sessions coexist without one preview clobbering another.
- ❌ Mutating an existing entry's `--path` to retarget it. Add a new entry instead.
- ❌ Constructing `http://localhost:<port>/dev?path=<X>` yourself. The proxy's `/` redirect is what appends the workspace ID and auth token; bypassing it gives a broken page. Always use `http://localhost:<port>/`.
- ❌ Starting `wmill dev` in the foreground (you'll hang). Always background.
- ❌ Listing both "open in IDE pane" and "open in browser" as a menu — pick one based on context.
