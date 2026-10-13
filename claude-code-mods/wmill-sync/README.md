# wmill-sync

A Claude Code mod that shows what differs between a wmill project folder and its Windmill workspace.

Claude Code loads it on its own from the project's `.claude/skills/wmill-sync/` once the folder is trusted. It does nothing in a folder without a `wmill.yaml` in it or above it.

## What it shows

- **A band above the prompt** that appears only while something needs action: `wmill demo 1 to push · 1 to pull · 1 conflict [Push 1] [Pull 1] [Refresh] [Details]`. Details lists one row per item with its own Push, Pull, Diff or Resolve button.
- **Resolve** (on a conflict) opens the diff in a pane with three choices: Ask Claude to merge, Keep local (push), Keep workspace (pull).
- **`/wmill-sync`** runs a check now and prints the result, including "In sync".
- **Notes to Claude.** When Claude reads or edits a file (or names it in a shell command) whose item changed on the workspace, the tool result carries a note telling it to pull first. A `wmill … push` Claude runs goes through the permission prompt, named as a deploy.

## How it decides the direction

After every check, each item that matches the workspace is recorded with its local file hash and its workspace version (script hash, flow `edited_at`, app version). For an item that differs:

- If its local hash moved since that record, it was edited locally, so it needs a push.
- If its workspace version moved, it changed on the workspace, so it needs a pull.
- If both moved, it's a conflict.

Variables, resources, schedules and triggers have no version, so they never show as a conflict: a local change reads as a push, anything else as a pull. With no record yet, git decides: a file that differs from `HEAD` is a local edit.

When the workspace has Git Sync configured, a git row shows how far the branch is ahead of and behind its upstream, with `git pull` and `git push` buttons.

## Checks

- At session start, and right after any `wmill` or `git` command Claude runs.
- About 2s after Claude edits a project file.
- Every 2 minutes. The timer runs the full check (the `wmill sync pull --dry-run` download) only when a workspace version or a local file changed, plus every fifth tick for the kinds without a version.

## Requirements

- The `wmill` CLI, logged in (`wmill workspace add`).
- If Claude Code can't find `wmill` on its PATH, set the plugin option `wmillPath` in `~/.claude/settings.json` under `pluginConfigs["wmill-sync@skills-dir"].options`.

## Development

```
claude plugin validate .
claude plugin test .
```
