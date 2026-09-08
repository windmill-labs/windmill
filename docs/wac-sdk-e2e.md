# Exercising an unreleased SDK change end to end

A job installs the **published** `windmill-client` / `wmill`, so a change in this
repo is invisible to a real job until it is injected into the worker's dependency
cache. The SDK unit suites cover neither the worker nor the SDK the worker
installs, and that gap is where the Workflow-as-Code failure contract kept coming
apart: every finding behind #10366, #10367 and #10368 came from review or from a
run like the one below, never from a green suite.

## Recipe

Use a private `WINDMILL_DIR`. `/tmp/windmill/` is shared by every worktree's
backend, so patching it in place leaks a modified SDK into other people's jobs.

```bash
# Run from the repository root. The backend holds a terminal of its own; every
# other command is a subshell, so nothing depends on where the last one left you.

# 1. a backend of your own, with its own cache root — in its own terminal
(cd backend && DATABASE_URL=... PORT=8062 WINDMILL_DIR=/tmp/windmill-mytest \
   cargo run --features quickjs)          # add ,python to run python jobs

# 2. one job to populate the cache with the published SDK
#    (any preview job importing the client will do)

# 3. build the client and overwrite what the cache holds
C=$(echo /tmp/windmill-mytest/cache_nomount/bun/windmill-client@*@@@1)
(cd typescript-client && ./build.sh && npx tsdown --format esm --no-dts \
   && cp dist/index.mjs "$C/dist/index.mjs" \
   && cp dist/index.mjs "$C/dist/client.mjs")   # code-split package: cover both

# python instead: copy the source file straight over, then drop the bytecode
cp python-client/wmill/wmill/client.py \
   /tmp/windmill-mytest/cache/python_3_12/wmill==*/wmill/client.py
find /tmp/windmill-mytest -name __pycache__ -type d -exec rm -rf {} +

# 4. RESTART the backend — see below
# 5. run your scenarios, and rm -rf /tmp/windmill-mytest when done
```

## Restart the workers after injecting

A worker materializes the package once and keeps using its copy, so patching the
cache under a running backend leaves some workers on the old code. With more than
one worker the results then **alternate run to run** as jobs land on one worker or
the other, which reads like flakiness in the product rather than in the harness.
Restarting after the swap makes it deterministic.

Symptom worth recognising: identical jobs returning two different answers in a
stable pattern, with each run internally consistent.

## Run your scenarios twice

Once against the published SDK, once against the injected one. A scenario that
behaves the same either way is not testing what you think it is, and it is easy
to write several of those without noticing.

As a calibration: a spread of WAC scenarios written this way scored 10/17 (bun)
and 7/18 (python) against the published SDK and 17/17 and 18/18 against a client
carrying #10366, #10367 and #10368. The ones that did not move were covering
behaviour those PRs never touched — worth knowing before concluding that a green
run means anything.

## Worth covering, and easy to miss

The deno path, `taskScript` / `taskFlow`, `waitForApproval`, and failures
interleaved with parallelism. None of these were exercised while the failure
record was being unified.
