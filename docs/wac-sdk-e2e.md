# Exercising an unreleased SDK change end to end

A WAC job installs the **published** `windmill-client` / `wmill`, so a change in
this repo is invisible to a real job until it is injected into the worker's
dependency cache. Unit tests cover neither the worker nor the SDK the worker
installs, and that gap is exactly where the WAC failure contract kept coming
apart: every finding behind #10366, #10367 and #10368 was found by review or by
a run like the one below, never by a green suite.

`integration_tests/wac_matrix_bun.py` and `integration_tests/wac_matrix_python.py`
are the behaviour matrices. Each case is a real preview job.

## Recipe

Use a private `WINDMILL_DIR`. `/tmp/windmill/` is shared by every worktree's
backend, so patching it in place leaks a modified SDK into other people's jobs.

```bash
# 1. a backend of your own, with its own cache root
cd backend
DATABASE_URL=... PORT=8062 WINDMILL_DIR=/tmp/windmill-mytest \
  cargo run --features quickjs            # add ,python to run python jobs

# 2. one job to populate the cache with the published SDK
python3 integration_tests/wac_matrix_bun.py --base-url http://localhost:8062

# 3. build the client and overwrite what the cache holds
cd typescript-client && ./build.sh && npx tsdown --format esm --no-dts
C=/tmp/windmill-mytest/cache_nomount/bun/windmill-client@*@@@1
cp dist/index.mjs $C/dist/index.mjs
cp dist/index.mjs $C/dist/client.mjs   # the package is code-split; cover both

# python instead: copy the source file straight over, then drop the bytecode
cp python-client/wmill/wmill/client.py \
   /tmp/windmill-mytest/cache/python_3_12/wmill==*/wmill/client.py
find /tmp/windmill-mytest -name __pycache__ -type d -exec rm -rf {} +

# 4. RESTART the backend — see below
# 5. run the matrix again, and rm -rf /tmp/windmill-mytest when done
```

## Restart the workers after injecting

A worker materializes the package once and keeps using its copy, so patching the
cache under a running backend leaves some workers on the old code. With more
than one worker the results then **alternate run to run** as jobs land on one
worker or the other, which reads like flakiness in the product rather than in the
harness. Restarting after the swap makes it deterministic.

Symptom worth recognising: identical jobs returning two different answers in a
stable pattern, with each run internally consistent.

## Run it twice

Run the matrix against the published SDK before injecting, and again after. A
case that passes both ways is not testing what you think it is, and the matrices
hold only cases that move: against the published SDK they score 1/8 and 1/12,
where the one is a smoke case that exists so a harness that ran nothing at all is
obvious. Against a client carrying #10366, #10367 and #10368 they score 8/8 and
12/12.

Keep that property when adding a case. If a new one passes before your change,
it belongs in the SDK unit suites, not here — nothing runs these automatically,
so every case has to earn the attention of whoever runs them.

## What the matrices do not cover

The deno path, `taskScript` / `taskFlow`, `waitForApproval`, and failures
interleaved with parallelism.
