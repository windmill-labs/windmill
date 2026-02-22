# Dev Dashboard

Web-based dashboard for managing Windmill development worktrees. Lets you create, monitor, and interact with multiple isolated development environments, each running its own AI coding agent (Claude or Codex), backend, and frontend.

## Quick start

```bash
# 1. Install dependencies
cargo install workmux          # worktree orchestrator
sudo apt install tmux socat    # (or brew install tmux socat)
curl -fsSL https://bun.sh/install | bash  # bun >1.3.5 required

# 2. Create the workmux global config
mkdir -p ~/.config/workmux
cat > ~/.config/workmux/config.yaml << 'EOF'
nerdfont: false

sandbox:
  image: windmill-sandbox

  # Forward R2/AWS credentials into sandbox containers (for screenshot uploads).
  # The actual values come from dev-dashboard/.env, sourced by dev.sh/run.sh.
  env_passthrough:
    - AWS_ACCESS_KEY_ID
    - AWS_SECRET_ACCESS_KEY
    - R2_ENDPOINT
    - R2_BUCKET
    - R2_PUBLIC_URL

  extra_mounts:
    # Codex agent credentials
    - host_path: ~/.codex
      guest_path: /tmp/.codex
      writable: true
    # EE repo access (optional — only needed for enterprise features)
    - host_path: ~/windmill-ee-private
      writable: true
    - host_path: ~/windmill-ee-private__worktrees
      writable: true
EOF

# 3. (Optional) Build sandbox image — only needed for agent-yolo profile
docker build -f Dockerfile.sandbox -t windmill-sandbox .

# 4. Install frontend deps
cd dev-dashboard/frontend && bun install && cd ..

# 5. Start the dashboard
./dev.sh                       # dev mode (hot reload), UI on :5112
# or
./run.sh                       # production mode (build + serve), UI on :4173

# 6. Open http://localhost:5112
```

## Architecture

```
Browser (localhost:5112)
    │
    ├── REST API (/api/*)  ──┐
    └── WebSocket (/ws/*)  ──┤
                             │
                    Vite dev proxy
                             │
                    Backend (localhost:5111)
                             │
              ┌──────────────┼──────────────┐
              │              │              │
          workmux CLI    tmux sessions   socat
          (worktree       (terminal      (port forwarding
           lifecycle)      access)        for sandboxes)
```

**Backend** — Bun/TypeScript HTTP + WebSocket server (`backend/src/server.ts`). Exposes two interfaces:

- **REST API** (`/api/*`) — CRUD for worktrees. Wraps the `workmux` CLI to create/remove/merge worktrees and runs `socat` port forwarding for Docker sandbox containers. The `GET /api/worktrees` endpoint enriches each worktree with its directory, assigned ports (from `.env.local`), and whether the backend/frontend services are actually responding.
- **WebSocket** (`/ws/*`) — Live terminal connection. This is what makes the in-browser terminal work. See [Terminal streaming](#terminal-streaming) below.

**Frontend** — Svelte 5 SPA with Tailwind CSS and xterm.js (`frontend/src/`). Provides a two-panel UI: worktree list sidebar + embedded terminal. Polls the REST API every 5 seconds for status updates. The terminal is rendered by [xterm.js](https://xtermjs.org/), which handles all terminal emulation (escape sequences, colors, cursor, scrollback) in a `<canvas>`/DOM element.

### Terminal streaming

The WebSocket provides a bidirectional bridge between xterm.js in the browser and a tmux session on the server. The data flow:

```
Browser (xterm.js)  ←— WebSocket —→  Backend  ←— stdin/stdout pipes —→  script (PTY)  ←— tmux attach —→  tmux grouped session
```

When a worktree is selected, the frontend opens a WebSocket to `/ws/<worktree>` and sends an initial `resize` message with the terminal dimensions. The backend then:

1. Spawns `script -q -c "... tmux attach-session ..." /dev/null` — `script` allocates a real PTY (pseudo-terminal), which is necessary for tmux to produce proper terminal escape sequences, colors, and cursor movement.
2. The tmux command creates a **grouped session** (`tmux new-session -t <main-session>`), which is a separate "view" into the same tmux windows. This allows the dashboard and a real terminal to view the same worktree simultaneously without fighting over window/pane focus.
3. An async reader loop reads the PTY's stdout and forwards the data over the WebSocket as `{ type: "output" }` messages, which xterm.js renders.
4. Keystrokes arrive as `{ type: "input" }` messages and are written to the PTY's stdin pipe.
5. Resize events trigger `tmux resize-window` to keep dimensions in sync.

Output is also buffered in a scrollback array (up to 5000 chunks) so that reconnecting clients receive recent history immediately.

### Worktree Profiles

When creating a worktree, you pick a profile that determines what runs inside it:

| Profile | What it does |
|---------|-------------|
| `full` | Agent + Cargo backend + Vite frontend (uses pane layout from `.workmux.yaml`) |
| `agent-yolo` | Agent runs inside a Docker sandbox container with `--dangerously-skip-permissions`. Socat forwards the container's ports to the host so they're reachable from your browser. |

## Prerequisites

### Required tools

| Tool | Min version | Purpose |
|------|-------------|---------|
| [**bun**](https://bun.sh) | >1.3.5 | Runtime for both backend and frontend dev server |
| [**workmux**](https://github.com/raine/workmux) | latest | Worktree + tmux orchestration (`cargo install workmux` or see its repo) |
| **tmux** | 3.x | Terminal multiplexer — workmux manages sessions/windows through it |
| **socat** | 1.7+ | TCP port forwarding for sandbox containers (only needed for `agent-yolo` profile) |
| **git** | 2.x | Worktree management |
| **docker** | 28+ | Only needed for `agent-yolo` sandbox profile |

### Workmux global config

Workmux reads a global config from `~/.config/workmux/config.yaml`. Create it if it doesn't exist:

```yaml
nerdfont: false

sandbox:
  image: windmill-sandbox
  env_passthrough:
    - AWS_ACCESS_KEY_ID
    - AWS_SECRET_ACCESS_KEY
    - R2_ENDPOINT
    - R2_BUCKET
    - R2_PUBLIC_URL
  extra_mounts:
    - host_path: ~/.codex
      guest_path: /tmp/.codex
      writable: true
    - host_path: ~/windmill-ee-private
      writable: true
    - host_path: ~/windmill-ee-private__worktrees
      writable: true
```

**Fields:**

- **`nerdfont`** — Set to `true` if your terminal uses a Nerd Font (adds icons to `workmux list` output). Default `false`.
- **`sandbox.image`** — Docker image used for `agent-yolo` sandboxed worktrees. Must be pre-built with `workmux sandbox build` or pulled with `workmux sandbox pull`.
- **`sandbox.env_passthrough`** — Host env vars to forward into sandbox containers (global config only). Used here for R2 screenshot upload credentials.
- **`sandbox.extra_mounts`** — Additional bind mounts into sandbox containers. Mounts Codex credentials and the EE repo for enterprise features.

To build the sandbox image (from the Windmill repo root):

```bash
docker build -f Dockerfile.sandbox -t windmill-sandbox .
```

### Workmux project config

The repo-level `.workmux.yaml` at the Windmill root configures how worktrees are created. Key settings:

- **`post_create`** — Runs `./scripts/worktree-env` after creating a worktree, which generates a `.env.local` file with unique `BACKEND_PORT` and `FRONTEND_PORT` assignments so multiple worktrees don't collide.
- **`panes`** — Defines the tmux pane layout for `full` profile: agent pane (focused), backend pane (`cargo watch`), and frontend pane (`npm run dev`).
- **`files.copy`** — Copies `backend/.env` and `scripts/` into each new worktree.

## Running

From the `dev-dashboard/` directory:

```bash
./dev.sh
```

This starts both backend and frontend, with logs prefixed `[BE]` / `[FE]`. `Ctrl+C` stops both.

You can also start them separately:

```bash
# Terminal 1: backend (auto-reloads on save)
cd backend && bun run dev

# Terminal 2: frontend (Vite dev server)
cd frontend && bun run dev
```

Open http://localhost:5112 in your browser.

### Cursor IDE integration

The top bar has a **Cursor** button that opens the selected worktree's directory in Cursor IDE via the `cursor://` protocol. Click the gear icon next to it to configure SSH remote host.

By default, clicking the button reuses an existing Cursor window. To always open in a **new window**, add this to your Cursor `settings.json` (`Cmd+Shift+P` → "Preferences: Open Settings (JSON)"):

```json
"window.openFoldersInNewWindow": "on"
```

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+Up/Down` | Navigate between worktrees |
| `Cmd+K` | Create new worktree |
| `Cmd+D` | Remove selected worktree |

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DASHBOARD_PORT` | `5111` | Backend API port |

The frontend dev server is hardcoded to port `5112` and proxies `/api/*` and `/ws/*` to the backend.

### Screenshot uploads (optional)

Sandbox agents can take screenshots of the frontend UI with Playwright and upload them to a Cloudflare R2 bucket for use in PR descriptions. To enable this, create a `dev-dashboard/.env` file (already gitignored):

```bash
# Cloudflare R2 credentials — get from:
# Dashboard → R2 → Manage R2 API Tokens → Create API Token (Object Read & Write, scoped to your bucket)
AWS_ACCESS_KEY_ID=<your-r2-access-key>
AWS_SECRET_ACCESS_KEY=<your-r2-secret-key>

# Account ID is on the R2 overview page (right sidebar)
R2_ENDPOINT=https://<ACCOUNT_ID>.r2.cloudflarestorage.com
R2_BUCKET=windmill-screenshots

# Enable public access on the bucket (Settings → Public access → r2.dev subdomain)
R2_PUBLIC_URL=https://pub-<hash>.r2.dev
```

When these are set, `dev.sh`/`run.sh` source the file and the env vars are inlined onto the `workmux sandbox agent` command. The workmux global config's `env_passthrough` (see [above](#workmux-global-config)) forwards them into the container. The agent's system prompt automatically includes screenshot instructions when R2 is configured.

## API

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/worktrees` | List all worktrees with status, ports, and service health |
| `POST` | `/api/worktrees` | Create a worktree (`{ branch, profile?, agent?, prompt? }`) |
| `DELETE` | `/api/worktrees/:name` | Remove a worktree |
| `POST` | `/api/worktrees/:name/open` | Open/focus a worktree's tmux window |
| `POST` | `/api/worktrees/:name/close` | Close a worktree's tmux window (keeps the worktree) |
| `POST` | `/api/worktrees/:name/send` | Send a prompt to the worktree's agent (`{ prompt }`) |
| `GET` | `/api/worktrees/:name/status` | Get agent status for a worktree |
| `WS` | `/ws/:worktree` | Terminal WebSocket (xterm.js ↔ tmux) |
