# Worker pool saturation on October 28

On October 28 between 14:05 and 14:52 UTC, jobs in the `default` worker group waited up to 11 minutes before starting. No job failed, but 3 schedules missed their window.

## Impact

| Measure            | Normal | During the incident |
| :----------------- | -----: | ------------------: |
| Median queue wait  |  0.4 s |          4 min 12 s |
| Longest queue wait |    3 s |         11 min 40 s |
| Jobs delayed       |      — |              18,420 |
| Schedules missed   |      0 |                   3 |

## Timeline

All times are UTC.

| Time  | Event                                     |
| :---- | :---------------------------------------- |
| 14:05 | A backfill flow queues 6,000 jobs at once |
| 14:09 | The queue wait alert fires                |
| 14:16 | The on-call engineer pauses the backfill  |
| 14:31 | 4 extra workers join the `default` group  |
| 14:52 | Queue wait is back under 1 s              |

## Root cause

The backfill flow ran its for-loop with no parallelism limit, so every iteration was queued at once. The `default` group had 8 workers, and the backfill jobs sat in the queue ahead of every other job in the group.

> The old script had a parallelism limit. It did not carry over when the script became a flow, because the for-loop step has its own setting.
>
> — Platform team

## What went well

- The queue wait alert fired 4 minutes after the backfill started
- Pausing the schedule stopped new jobs immediately

## Action items

- [x] Set parallelism to 16 on the backfill for-loop
- [ ] Move backfills to a dedicated `batch` worker group
- [ ] Alert when one flow holds more than half of a group's queue

> [!TIP]
> A worker group's tags decide which jobs it runs. Tagging the backfill flow `batch` keeps it off the `default` workers.

Report written on October 30.[^1]

[^1]: Queue figures come from the instance's job history, sampled every 30 seconds.
