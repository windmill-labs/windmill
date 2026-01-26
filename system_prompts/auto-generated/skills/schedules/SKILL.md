---
name: schedules
description: Configure cron schedules for automated script and flow execution.
---

# Windmill Schedules

Schedules run scripts and flows automatically on a cron schedule.

## File Format

Schedule files use the pattern: `{path}.schedule.json`

Example: `u/user/daily_sync.schedule.json`

## Schedule Structure

```json
{
  "path": "u/user/daily_sync",
  "script_path": "u/user/sync_script",
  "is_flow": false,
  "schedule": "0 0 0 * * *",
  "timezone": "UTC",
  "enabled": true,
  "args": {
    "source": "api",
    "destination": "database"
  },
  "summary": "Daily data synchronization",
  "description": "Syncs data from API to database every day at midnight"
}
```

## Cron Expression Format

Windmill uses 6-field cron expressions (includes seconds):

```
 ┌───────────── second (0-59)
 │ ┌───────────── minute (0-59)
 │ │ ┌───────────── hour (0-23)
 │ │ │ ┌───────────── day of month (1-31)
 │ │ │ │ ┌───────────── month (1-12)
 │ │ │ │ │ ┌───────────── day of week (0-6, 0=Sunday)
 │ │ │ │ │ │
 * * * * * *
```

**Common Examples:**
- `0 0 0 * * *` - Daily at midnight
- `0 0 12 * * *` - Daily at noon
- `0 */5 * * * *` - Every 5 minutes
- `0 0 9 * * 1-5` - Weekdays at 9 AM
- `0 0 0 1 * *` - First day of each month
- `0 30 8 * * *` - Daily at 8:30 AM

## Required Fields

- `path` - Unique schedule identifier
- `script_path` - Path to the script or flow to run
- `is_flow` - true if targeting a flow, false for script
- `schedule` - Cron expression (6 fields, includes seconds)
- `timezone` - Timezone for schedule (e.g., "UTC", "America/New_York")
- `enabled` - Whether the schedule is active

## Arguments

Pass arguments to the script/flow:

```json
{
  "args": {
    "param1": "value1",
    "param2": 42,
    "config": {
      "nested": "object"
    }
  }
}
```

Arguments can reference variables:
```json
{
  "args": {
    "api_key": "$var:g/all/api_key"
  }
}
```

## Error Handling

Configure handlers for failures and recovery:

```json
{
  "on_failure": "u/user/notify_failure",
  "on_failure_times": 3,
  "on_failure_exact": false,
  "on_failure_extra_args": {
    "channel": "#alerts"
  },
  "on_recovery": "u/user/notify_recovery",
  "on_recovery_times": 1,
  "on_recovery_extra_args": {}
}
```

**Fields:**
- `on_failure` - Script/flow to run after failures
- `on_failure_times` - Number of consecutive failures before triggering
- `on_failure_exact` - If true, only trigger on exactly N failures
- `on_recovery` - Script/flow to run when schedule recovers after failure

## Success Handler

Run a script after successful completion:

```json
{
  "on_success": "u/user/post_process",
  "on_success_extra_args": {
    "notify": true
  }
}
```

## Retry Configuration

Configure automatic retries on failure:

```json
{
  "retry": {
    "max_retries": 3,
    "max_wait": 300,
    "multiplier": 2
  }
}
```

- `max_retries` - Maximum retry attempts
- `max_wait` - Maximum wait time in seconds between retries
- `multiplier` - Exponential backoff multiplier

## Flow Overlap Prevention

Prevent concurrent executions of the same flow:

```json
{
  "no_flow_overlap": true
}
```

## Pausing Schedules

Temporarily pause a schedule:

```json
{
  "paused_until": "2024-12-31T23:59:59Z"
}
```

## Worker Tag

Run on specific worker groups:

```json
{
  "tag": "high-memory"
}
```

## CLI Commands

```bash
# List schedules
wmill schedule list

# Push schedule configuration
wmill sync push

# Preview next run times
wmill schedule preview "0 0 * * * *"
```
