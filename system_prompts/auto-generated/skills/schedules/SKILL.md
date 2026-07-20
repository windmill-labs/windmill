---
name: schedules
description: MUST use when configuring schedules.
---

# Windmill Schedules

Schedules run scripts and flows automatically on a cron schedule.

## File Naming

Schedule files use the pattern: `{path}.schedule.yaml`

Example: `f/folder/daily_sync.schedule.yaml`

Note: The path is derived from the filename, not stored in the file content.

## Cron Expression Format

Windmill uses 6-field cron expressions (includes seconds):

```
 ┌───────────── second (0-59)
 │ ┌───────────── minute (0-59)
 │ │ ┌───────────── hour (0-23)
 │ │ │ ┌───────────── day of month (1-31)
 │ │ │ │ ┌───────────── month (1-12 or jan-dec)
 │ │ │ │ │ ┌───────────── day of week (0-6, 0=Sunday, or sun-sat)
 │ │ │ │ │ │
 * * * * * *
```

**Common Examples:**
- `0 0 0 * * *` - Daily at midnight
- `0 0 12 * * *` - Daily at noon
- `0 */5 * * * *` - Every 5 minutes
- `0 0 9 * * 1-5` - Weekdays at 9 AM
- `0 0 0 1 * *` - First day of each month

## CLI Commands

Deploying local changes to the workspace can be destructive to remote state — only suggest/run a deploy when the user explicitly asks to deploy/publish/push, not when they say "run", "try", or "test". If the repo deploys on git push (backend auto-pull or CI — check `wmill gitsync-settings status`), deploy with `git push`; otherwise use `wmill sync push`. The commands below never mutate remote state, so they're safe to run yourself — note that `sync pull` does overwrite local files to match the remote (use `sync pull --dry-run` to only preview), while `schedule` just lists.

```bash
# Push schedules to Windmill — only when the user explicitly asks to deploy
wmill sync push

# Pull schedules from Windmill
wmill sync pull

# List schedules
wmill schedule
```
