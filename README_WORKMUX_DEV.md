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
| ---- | ------- | -------- |
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

The `post_create` hook also copies `frontend/node_modules` using `cp -a` (preserves `.bin/` symlinks that `cp -r` would dereference).

## Enterprise (EE) Code Access

The enterprise source code lives in the `windmill-ee-private` repository (sibling to this repo). When you create a worktree, `scripts/worktree-env` automatically creates a matching EE worktree on the same branch and configures Claude Code's `additionalDirectories` to grant access.

### Sandbox setup

When using sandbox mode, the container needs explicit mounts to access the EE repo. Add the following to your global workmux config (`~/.config/workmux/config.yaml`):

```yaml
sandbox:
  extra_mounts:
    - host_path: ~/windmill-ee-private
      writable: true
    - host_path: ~/windmill-ee-private__worktrees
      writable: true
```

This mounts both the main EE repo (used by the main worktree) and the EE worktrees directory (used by feature worktrees) into every sandbox container.

## Cursor SSH Integration (`wmc`)

`wm-cursor` (aliased as `wmc`) gives each worktree its own Cursor SSH remote window with an independently-focused tmux session. All windows are visible in the status bar across all Cursor terminals, but each one is focused on its own worktree.

This uses **grouped tmux sessions** — multiple sessions that share the same window list but track focus independently:

```
tmux session: main          <-- your main Cursor terminal
tmux session: cursor-feat-a <-- Cursor window for feat-a (focused on wm-feat-a)
tmux session: cursor-feat-b <-- Cursor window for feat-b (focused on wm-feat-b)
    \__ all three share the same windows in the status bar
```

### Setup

Run once from inside tmux on the remote:

```bash
./scripts/wm-cursor setup /home/hugo/projects/windmill
```

This:

1. **Merges `.vscode/settings.json`** — adds the `wm-tmux` terminal profile (auto-attaches to the `main` tmux session), disables auto port forwarding, configures forwarding for ports 8000/3000/5432, and stops rust-analyzer from auto-starting. Existing settings are preserved.
2. **Creates `.vscode/tasks.json`** — auto-starts the dev database (`start-dev-db.sh`) when the folder opens.
3. **Adds `wmc` alias to `~/.zshrc`** — so you can use `wmc` from any tmux window.
4. **Adds `eval "$(wmc completions)"`** to `~/.zshrc` — provides tab-completion for subcommands and worktree names (for `open`, `open-ee`, and `close`).

After setup, reopen Cursor's terminal to pick up the new profile.

### Usage

All commands run from inside a tmux session (i.e., from Cursor's integrated terminal after setup).

**Create a new worktree + open Cursor:**

```bash
wmc add -A -p "implement feature X"
```

This runs `workmux add`, creates a grouped tmux session, writes `.vscode/settings.json` in the worktree (with port forwarding matching the worktree's assigned ports), and opens a new Cursor window.

**Open Cursor for an existing worktree:**

```bash
wmc open my-feature
```

**Open the EE worktree in Cursor (no tmux session):**

```bash
wmc open-ee my-feature
```

This finds the matching `windmill-ee-private__worktrees/<name>` directory and opens it in a new Cursor window.

**Close a worktree's Cursor window and tmux window (keeps the worktree):**

```bash
wmc close my-feature
```

This kills the grouped tmux session and calls `workmux close` to close the tmux window. The worktree and branch are preserved. Grouped sessions are also automatically cleaned up when you `workmux rm` a worktree (via `scripts/worktree-cleanup`).

## Cargo Features

To build the backend with specific Cargo features (e.g., `enterprise`, `parquet`), pass them via `CARGO_FEATURES`. The backend pane reads this from `.env.local` and appends `--features <value>` to the `cargo watch` command.

**With `wm` (workmux):**

Set `CARGO_FEATURES` as an environment variable before creating the worktree:

```bash
CARGO_FEATURES="enterprise,parquet" wm add my-feature
```

This gets written to `.env.local` by the `post_create` hook (`scripts/worktree-env`), and the backend pane picks it up automatically.

**With `wmc` (wm-cursor):**

Use the `--features` flag:

```bash
# Create a new worktree with features
wmc add --features "enterprise,parquet" -A -p "implement feature X"

# Open an existing worktree with different features
wmc open my-feature --features "enterprise,parquet"
```

The `--features` flag exports `CARGO_FEATURES` so the `post_create` hook writes it to `.env.local`. When using `wmc open`, it updates the existing `.env.local` with the new features.

## Login

Default credentials: `admin@windmill.dev` / `changeme`
