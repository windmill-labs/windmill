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

### 4. Install llm CLI (required for auto branch naming)

workmux uses the `llm` CLI to automatically generate branch names from prompts. Install it with:

```bash
uv tool install llm
llm install llm-anthropic
```

Then set your Anthropic API key:

```bash
llm keys set anthropic
# paste your API key when prompted
```

### 5. Recommended: shell alias and autocomplete

Set up a `wm` alias for convenience:

```bash
# Add to your ~/.zshrc
alias wm="workmux"
```

Setting up zsh autocomplete is also recommended — see the [workmux docs](https://github.com/rubenfiszel/workmux) for instructions.

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

# Create a worktree and immediately send a prompt to the agent
workmux add -A -p "fix the login bug in auth.rs"
```

The `add` command creates the worktree but does **not** open it. To open the tmux window and start working:

```bash
workmux open my-feature
```

This will open a tmux window with three panes:

- **Claude Code agent** (focused)
- **Backend**: `cargo watch -x run` on the assigned port (auto-reloads on save)
- **Frontend**: `npm run dev` proxying to the backend

When using `-A` with `add`, the worktree is created and opened automatically, and the prompt is sent to the agent right away.

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

### Merging and cleaning up

We never merge worktrees directly — always create a PR on GitHub and let it be merged there. Once the PR is merged, clean up the worktree:

```bash
# Close the tmux window but keep the worktree
workmux close my-feature

# After your PR is merged, remove the worktree, branch, and tmux window
workmux rm my-feature
```

> **Note**: Do not use `workmux merge`. Always go through a PR to get your changes into main. You can ask the Claude Code agent in the worktree to create the PR for you.

## Configuration

The setup is defined in `.workmux.yaml` at the repo root. Key sections:

- **`post_create`**: Runs `scripts/worktree-env` to generate `.env.local` with port assignments
- **`panes`**: Defines the tmux layout (agent, backend, frontend)
- **`files.copy`**: Copies `backend/.env` and `scripts/` into each worktree
- **`files.symlink`**: Symlinks `node_modules` and `.svelte-kit` to avoid reinstalling per worktree

## Login

Default credentials: `admin@windmill.dev` / `changeme`
