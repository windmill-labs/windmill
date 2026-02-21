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
  extra_mounts:
    - host_path: ~/.codex
      guest_path: /tmp/.codex
      writable: true
EOF

# 3. (Optional) Build sandbox image — only needed for agent-yolo profile
docker build -f Dockerfile.sandbox -t windmill-sandbox .

# 4. Install frontend deps
cd dev-dashboard/frontend && bun install && cd ..

# 5. Start the dashboard
cd backend  && bun run dev &   # API on :5111
cd frontend && bun run dev &   # UI  on :5112

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

**Backend** — Bun/TypeScript HTTP + WebSocket server (`backend/src/server.ts`). Wraps the `workmux` CLI to create/remove worktrees, manages tmux terminal sessions streamed to the browser via WebSocket, and runs `socat` port forwarding for Docker sandbox containers.

**Frontend** — Svelte 5 SPA with Tailwind CSS and xterm.js (`frontend/src/`). Provides a two-panel UI: worktree list sidebar + embedded terminal. Polls the backend every 5 seconds for status updates.

### Worktree Profiles

When creating a worktree, you pick a profile that determines what runs inside it:

| Profile | What it does |
|---------|-------------|
| `full` | Agent + Cargo backend + Vite frontend (uses pane layout from `.workmux.yaml`) |
| `agent-only` | Agent in one pane + a shell in a second pane. No backend/frontend auto-started. |
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
  extra_mounts:
    - host_path: ~/.codex
      guest_path: /tmp/.codex
      writable: true
```

**Fields:**

- **`nerdfont`** — Set to `true` if your terminal uses a Nerd Font (adds icons to `workmux list` output). Default `false`.
- **`sandbox.image`** — Docker image used for `agent-yolo` sandboxed worktrees. Must be pre-built with `workmux sandbox build` or pulled with `workmux sandbox pull`.
- **`sandbox.extra_mounts`** — Additional bind mounts into sandbox containers. The example above mounts Codex credentials so the Codex agent can authenticate from inside the container.

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

Start both backend and frontend (from the `dev-dashboard/` directory):

```bash
# Terminal 1: backend (auto-reloads on save)
cd backend && bun run dev

# Terminal 2: frontend (Vite dev server)
cd frontend && bun run dev
```

Open http://localhost:5112 in your browser.

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `DASHBOARD_PORT` | `5111` | Backend API port |

The frontend dev server is hardcoded to port `5112` and proxies `/api/*` and `/ws/*` to the backend.

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
