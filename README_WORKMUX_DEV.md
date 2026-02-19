# Windmill Development with workmux

This guide covers the workmux-based development setup for Windmill. Each worktree gets its own tmux window with a Claude Code agent, a backend server (with auto-reload), and a frontend dev server — all on isolated ports.

## Prerequisites

- tmux
- Rust toolchain (rustup)
- Node.js + npm
- PostgreSQL running locally (see `backend/.env`)

## Installation

### 1. Install workmux

```bash
cargo install workmux
```

### 2. Install the Claude Code plugin

```bash
workmux claude install
```

This lets workmux manage Claude Code agents in worktree panes.

### 3. Install cargo-watch

Used for auto-recompiling the backend on file changes:

```bash
cargo install cargo-watch
```

## Port Slot System

Each worktree is assigned a **slot** that determines its ports:

| Slot | Backend | Frontend |
|------|---------|----------|
| 0    | 8000    | 3000     |
| 1    | 8010    | 3010     |
| 2    | 8020    | 3020     |
| 3    | 8030    | 3030     |
| ...  | ...     | ...      |

- **Slot 0** is reserved for the main worktree (default `cargo run` / `npm run dev`).
- Without `WM_SLOT`, the script auto-assigns the first available slot (starting from 1) and prints it.
- With `WM_SLOT=N`, it uses that slot and errors if the ports are taken.

## SSH Port Forwarding

If you develop over SSH, add this to `~/.ssh/config` on your **local machine** to pre-configure tunnels for each slot:

```
Host windmill-dev
  HostName <remote-ip>
  User <username>
  # Slot 0 (main worktree)
  LocalForward 8000 localhost:8000
  LocalForward 3000 localhost:3000
  # Slot 1
  LocalForward 8010 localhost:8010
  LocalForward 3010 localhost:3010
  # Slot 2
  LocalForward 8020 localhost:8020
  LocalForward 3020 localhost:3020
  # Slot 3
  LocalForward 8030 localhost:8030
  LocalForward 3030 localhost:3030
```

Then connect once and all tunnels are active:

```bash
ssh windmill-dev
```

Access the frontend at `http://localhost:<frontend-port>` in your local browser.

## Quickstart

```bash
# Create a new worktree (auto-assigns slot, prints ports)
workmux add my-feature

# Or with an explicit slot
WM_SLOT=2 workmux add my-feature
```

This will:

1. Create a git worktree + branch `my-feature`
2. Run `scripts/worktree-env` to assign ports and write `.env.local`
3. Open a tmux window with three panes:
   - **Claude Code agent** (focused)
   - **Backend**: `cargo watch -x run` on the assigned port (auto-reloads on save)
   - **Frontend**: `npm run dev` proxying to the backend

Check which ports were assigned:

```bash
cat <worktree-path>/.env.local
```

### Sending work to the agent

```bash
# Send a prompt to the agent in a worktree
workmux send my-feature "fix the login bug in auth.rs"

# Check agent status
workmux status
```

### Cleaning up

```bash
# Close the tmux window but keep the worktree
workmux close my-feature

# Remove everything (worktree, branch, tmux window)
workmux rm my-feature

# Merge into main, then clean up
workmux merge my-feature
```

## Configuration

The setup is defined in `.workmux.yaml` at the repo root. Key sections:

- **`post_create`**: Runs `scripts/worktree-env` to generate `.env.local` with port assignments
- **`panes`**: Defines the tmux layout (agent, backend, frontend)
- **`files.copy`**: Copies `backend/.env` and `scripts/` into each worktree
- **`files.symlink`**: Symlinks `node_modules` and `.svelte-kit` to avoid reinstalling per worktree

## Login

Default credentials: `admin@windmill.dev` / `changeme`
