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

```bash
# Push schedules to Windmill
wmill sync push

# Pull schedules from Windmill
wmill sync pull

# List schedules
wmill schedule
```


## Schedule (`*.schedule.yaml`)

Must be a YAML file that adheres to the following schema:

```json
{
  "type": "object",
  "properties": {
    "schedule": {
      "type": "string",
      "description": "Cron expression with 6 fields (seconds, minutes, hours, day of month, month, day of week). Example '0 0 12 * * *' for daily at noon"
    },
    "timezone": {
      "type": "string",
      "description": "IANA timezone for the schedule (e.g., 'UTC', 'Europe/Paris', 'America/New_York')"
    },
    "script_path": {
      "type": "string",
      "description": "Path to the script or flow to execute when triggered"
    },
    "is_flow": {
      "type": "boolean",
      "description": "True if script_path points to a flow, false if it points to a script"
    },
    "args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "on_failure": {
      "type": "string",
      "description": "Path to a script or flow to run when the scheduled job fails"
    },
    "on_failure_times": {
      "type": "number",
      "description": "Number of consecutive failures before the on_failure handler is triggered (default 1)"
    },
    "on_failure_exact": {
      "type": "boolean",
      "description": "If true, trigger on_failure handler only on exactly N failures, not on every failure after N"
    },
    "on_failure_extra_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "on_recovery": {
      "type": "string",
      "description": "Path to a script or flow to run when the schedule recovers after failures"
    },
    "on_recovery_times": {
      "type": "number",
      "description": "Number of consecutive successes before the on_recovery handler is triggered (default 1)"
    },
    "on_recovery_extra_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "on_success": {
      "type": "string",
      "description": "Path to a script or flow to run after each successful execution"
    },
    "on_success_extra_args": {
      "type": "object",
      "description": "The arguments to pass to the script or flow"
    },
    "ws_error_handler_muted": {
      "type": "boolean",
      "description": "If true, the workspace-level error handler will not be triggered for this schedule's failures"
    },
    "retry": {
      "type": "object",
      "properties": {
        "constant": {
          "type": "object",
          "description": "Retry with constant delay between attempts",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "seconds": {
              "type": "integer",
              "description": "Seconds to wait between retries"
            }
          }
        },
        "exponential": {
          "type": "object",
          "description": "Retry with exponential backoff (delay doubles each time)",
          "properties": {
            "attempts": {
              "type": "integer",
              "description": "Number of retry attempts"
            },
            "multiplier": {
              "type": "integer",
              "description": "Multiplier for exponential backoff"
            },
            "seconds": {
              "type": "integer",
              "minimum": 1,
              "description": "Initial delay in seconds"
            },
            "random_factor": {
              "type": "integer",
              "minimum": 0,
              "maximum": 100,
              "description": "Random jitter percentage (0-100) to avoid thundering herd"
            }
          }
        },
        "retry_if": {
          "$ref": "#/components/schemas/RetryIf"
        }
      },
      "description": "Retry configuration for failed module executions"
    },
    "summary": {
      "type": "string",
      "description": "Short summary describing the purpose of this schedule"
    },
    "description": {
      "type": "string",
      "description": "Detailed description of what this schedule does"
    },
    "no_flow_overlap": {
      "type": "boolean",
      "description": "If true, skip this schedule's execution if the previous run is still in progress (prevents concurrent runs)"
    },
    "tag": {
      "type": "string",
      "description": "Worker tag to route jobs to specific worker groups"
    },
    "paused_until": {
      "type": "string",
      "description": "ISO 8601 datetime until which the schedule is paused. Schedule resumes automatically after this time"
    },
    "cron_version": {
      "type": "string",
      "description": "Cron parser version. Use 'v2' for extended syntax with additional features"
    },
    "dynamic_skip": {
      "type": "string",
      "description": "Path to a script that validates scheduled datetimes. Receives scheduled_for datetime and returns boolean to skip (true) or run (false)"
    }
  },
  "required": [
    "schedule",
    "script_path",
    "timezone",
    "is_flow"
  ]
}
```