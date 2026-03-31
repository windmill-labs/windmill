# CLI Flow Logs — Friction Discovery Report

Date: 2026-03-31
Branch: `alp/cli_flow_logs`

## Test Setup

- Backend: localhost:8010 (EE v1.669.1), workspace `admins`
- CLI: wmill v1.669.1
- Two test flows:
  - `f/testing/test_flow` — 3-step flow with for-loop (happy path)
  - `f/testing/fail_flow` — 3-step flow where step b throws (error path)

---

## Friction Points (prioritized by severity)

### P0 — Critical Bugs

#### 1. `flow run` never terminates on failure

**File:** `cli/src/commands/flow/flow.ts:355-371`

When a flow step fails, the `while (true)` loop in `run()` gets stuck. The loop iterates over `flow_status.modules[]`, and when a module's type is not yet "completed" (e.g., still `WaitingForPriorSteps` because the flow engine is winding down), it prints the status and sleeps 100ms indefinitely. The flow is already complete (125ms), but the CLI never breaks out of the module-tracking loop.

The outer "wait for completion" retry loop (lines 375-402) is only reached after all modules have been iterated, which never happens because later modules remain in `WaitingForPriorSteps` after an earlier step fails.

**Reproduction:**
```bash
wmill flow run f/testing/fail_flow  # hangs forever printing WaitingForPriorSteps
```

**Impact:** Users must Ctrl-C and then manually retrieve the error. The flow's error message (which is excellent — includes step_id and stack trace) is never shown.

**Fix:** Check flow completion status inside the module-tracking loop. If `jobInfo.type === "CompletedJob"` (or `flow_status.failure_module` is set), break out immediately and print the error.

---

#### 2. `job list --all` does NOT show sub-jobs

**File:** `cli/src/commands/job/job.ts:84`

The `--all` flag sets `hasNullParent: undefined` in the API call, but the `listJobs` API with `job_kinds=script,flow,singlestepflow` still doesn't return sub-jobs. Sub-jobs have `job_kind` values like `flowdependencies`, `singlestepflow`, or just `script` with a non-null `parent_job`, but the default `job_kinds` filter excludes them.

**Evidence:** Flow `019d447a-20e3-47c3-84cf-884acf9de1ac` has 5 sub-jobs (confirmed via API), but `wmill job list --all` shows none of them.

**Impact:** `wmill job logs <flow_id>` tells users to "Use 'wmill job list --all' to see sub-jobs" — but that command doesn't actually show them. Users have no CLI path to find sub-job IDs.

**Fix:** When `--all` is set, also include sub-job-relevant `job_kinds` (or remove the filter entirely).

---

### P1 — Major UX Issues

#### 3. `WaitingForPriorSteps` spam floods the terminal

**File:** `cli/src/commands/flow/flow.ts:365-371`

When a module isn't ready yet, the code prints `module.type` and sleeps 100ms. For a 3-second for-loop, this produces ~600+ lines of `WaitingForPriorSteps`. The output for our 3-iteration for-loop was:

```
====== Job 1 ======           (step a — good)
...logs...
Job Completed
====== Job 2 ======           (loop iteration 0 — good)
...logs...
Job Completed
WaitingForPriorSteps          (repeated ~600 times)
WaitingForPriorSteps
WaitingForPriorSteps
...
====== Job 3 ======           (step c — good)
```

**Impact:** Completely buries the useful output. Makes the CLI unusable for piping/scripting.

**Fix:** Deduplicate consecutive identical status messages. Print `WaitingForPriorSteps` once, then show a spinner or progress indicator.

---

#### 4. For-loop iterations 1..N are invisible

**File:** `cli/src/commands/flow/flow.ts:353-371`

The module-tracking loop iterates `flow_status.modules[]`, which contains one entry per top-level module (a, b, c). Module `b` (the for-loop) has a single `.job` entry that points to iteration 0. Iterations 1 and 2 are tracked inside `module.iterator` or `module.flow_jobs` but are never shown.

For our 3-iteration loop, only iteration 0's logs were streamed. Iterations 1 and 2 ran silently.

**Impact:** Users only see 1/N of their for-loop execution. If iteration 2 fails, they'd never see why.

**Fix:** For `forloopflow` modules, iterate over all iteration jobs (available in the flow_status) and track each one.

---

#### 5. No step labels in streaming output

**File:** `cli/src/commands/flow/flow.ts:358`

Steps are labeled "Job 1", "Job 2", "Job 3" with no connection to the flow definition. The module's `id` (a, b, c) and `summary` ("Generate list", "Process each item", "Aggregate results") are available in `flow_status.modules[]` but not printed.

**Impact:** Users can't correlate streaming output with their flow definition.

**Fix:** Print `====== Step a: Generate list ======` instead of `====== Job 1 ======`.

---

### P2 — Minor Issues

#### 6. `job logs` prints "to remove ansi colors" hint to stderr twice

**File:** `cli/src/commands/job/job.ts:199`

The hint `to remove ansi colors, use: | sed '...'` is printed once to stderr. But in practice it appears twice in the output (observed in testing). Likely the function is called twice or there's a duplicate print path.

**Impact:** Minor clutter, but looks buggy.

---

#### 7. `job get` for a failed flow doesn't suggest how to find the failing step

When running `wmill job get <flow_id>`, the result JSON includes `"step_id": "b"` but there's no user-friendly message like "Step b failed. Run `wmill job logs <sub_job_id>` to see the error."

The user must manually parse the JSON result, then figure out how to find the sub-job ID for step b (which is impossible via CLI — see #2).

---

#### 8. No `--parent` or `--flow` filter on `job list`

There's no way to list sub-jobs of a specific flow job:
```bash
wmill job list --parent 019d447a-20e3-47c3-84cf-884acf9de1ac  # doesn't exist
```

The only way to find sub-jobs is via the raw API.

---

#### 9. `flow run` exit code doesn't reflect failure (sometimes)

**File:** `cli/src/commands/flow/flow.ts:393`

The code sets `process.exitCode = 1` on failure, but since the module-tracking loop hangs on failure (#1), this code is never reached. The user must Ctrl-C, which gives exit code 130 (SIGINT), making it impossible to distinguish "flow failed" from "user cancelled".

---

#### 10. Spurious `No wmill.yaml found` warning on every command

Every command prints `No wmill.yaml found. Use 'wmill init' to bootstrap it.` even when running standalone commands like `job list`. This is noise that adds up.

---

## Summary Matrix

| # | Issue | Severity | Category | Fix Effort |
|---|-------|----------|----------|------------|
| 1 | `flow run` hangs on failure | P0 | Bug | Small |
| 2 | `job list --all` doesn't show sub-jobs | P0 | Bug | Small |
| 3 | WaitingForPriorSteps spam | P1 | UX | Small |
| 4 | For-loop iterations invisible | P1 | Missing feature | Medium |
| 5 | No step labels in output | P1 | UX | Small |
| 6 | Double "ansi colors" hint | P2 | Bug | Trivial |
| 7 | No sub-job guidance on failure | P2 | UX | Small |
| 8 | No `--parent` filter | P2 | Missing feature | Small |
| 9 | Exit code wrong on failure | P2 | Bug | Trivial (blocked by #1) |
| 10 | Spurious wmill.yaml warning | P2 | UX | Small |

## Recommended Fix Order

1. **#1 + #9** — Fix flow run hang on failure (unblocks everything)
2. **#2** — Fix `job list --all` to actually show sub-jobs
3. **#3 + #5** — Clean up streaming output (dedupe + labels)
4. **#4** — Stream all for-loop iterations
5. Remaining P2 issues as time allows
