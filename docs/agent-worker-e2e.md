# Running an agent worker locally, for e2e

An agent worker reaches the database only through the API, so whole code paths
(`Connection::Http`) are never taken by a normal `cargo run`. Exercising them
needs a real one. Every step below has a failure mode that looks like something
else; they are listed with the error each produces.

## 1. Build with the right features

Four features, and the agent's own mode gate is the one that is easy to miss:

```bash
cd backend
cargo build --features quickjs,private,enterprise,license,agent_worker_server
```

- `agent_worker_server` mounts `/api/agent_workers/*` on the SERVER. Without it,
  `create_agent_token` returns **404** with an empty body.
- `enterprise` + `license` compile the agent MODE into the binary. Without them
  the worker exits immediately with `Agent mode is only available in the EE`,
  even though the server side works and mints tokens happily.

Verify before spending time on the handshake — the panic string must be absent:

```bash
strings target/debug/windmill | grep -c "Agent mode is only available in the EE"   # want 0
```

**Pin this feature set for the whole session.** `target/debug/windmill` is one
path shared by every feature combination, and cargo swaps the cached artifact in
and out as the set changes — a `cargo build --features quickjs` (or any build
with a different set) in another pane silently replaces the binary the server and
agent are about to run, and the swap back "completes" in under a second, so it
does not look like a rebuild happened. The symptom is the agent 401ing again
after it had been working, or the EE panic reappearing. Re-run the `strings`
check above whenever anything unexpected regresses, and start the server and the
agent from the SAME build.

## 2. Run the server without a local worker

`MODE=server` so nothing else drains the queue and the agent is provably the one
that ran the job:

```bash
DATABASE_URL=<this worktree's db> PORT=8420 MODE=server ./target/debug/windmill
```

## 3. Mint a token — with an expiry, and unquoted

```bash
TOK=$(curl -s -X POST localhost:8420/api/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"admin@windmill.dev","password":"changeme"}')

AT=$(curl -s -X POST localhost:8420/api/agent_workers/create_agent_token \
  -H "Authorization: Bearer $TOK" -H 'Content-Type: application/json' \
  -d '{"worker_group":"agentgrp","tags":["dbt"],"exp":1900000000}' | tr -d '"')
```

Two traps, both of which surface as a bare `401` on the agent and a decoded
reason only in the SERVER log:

- **`exp` must be a real timestamp.** `"exp": null` mints a token the validator
  rejects with `Missing required claim: exp`.
- **The response is JSON, so it arrives quoted.** Keeping the `"` gives
  `Base64 error: Invalid byte 34, offset 0` — hence the `tr -d '"'`.

Pass the token **exactly as minted**. It looks like `jwt_agent_<jwt>` and the
client appends its own hostname-derived suffix to form `jwt_agent_<suffix>_<jwt>`,
which is what the server splits on. Adding a suffix yourself yields
`Base64 error: Encoded text cannot have a 6-bit remainder`.

## 4. Start the agent

`WORKER_TAGS` must contain the tag the JOBS carry, not a tag you invent — a job
whose tag nothing serves sits in `v2_job_queue` forever and looks like a hang.
dbt scripts default to the `dbt` tag.

```bash
AGENT_TOKEN="$AT" BASE_INTERNAL_URL=http://localhost:8420 MODE=agent \
  WINDMILL_DIR=/home/$USER/wmagent \
  WORKER_GROUP=agentgrp WORKER_TAGS=dbt PORT=8499 ./target/debug/windmill
```

`WINDMILL_DIR` off `/tmp` matters on a dev box. Jobs fail with `IoErr: Disk quota
exceeded (os error 122)` while writing the project's files, and `df` looks
healthy — free space and free inodes both. `/tmp` is a tmpfs and Linux supports
per-user quotas on it, so the limit is the user's, not the filesystem's; several
agent sessions' caches under `/tmp` are enough to reach it. Point the worker at a
real disk instead of trying to clean up under the quota.

Confirm it registered rather than trusting a quiet log:

```sql
SELECT worker FROM worker_ping
 WHERE worker_group = 'agentgrp' AND ping_at > now() - interval '2 min';
-- ag-agentgrp-<host>-<rand>
```

## Reading failures

The agent only ever prints `Agent worker cannot connect to server. Please check
AGENT_TOKEN and BASE_INTERNAL_URL`. The actual reason is in the server log, from
`windmill-api-agent-workers/src/ee.rs` — grep it for `JWT_AGENT auth error`.

## Confirming the agent is what ran the job

`worker` on the completed job starts with `ag-`:

```bash
curl -s -H "Authorization: Bearer $TOK" \
  "localhost:8420/api/w/<ws>/jobs_u/completed/get/<job>" | jq -r .worker
```

## What dbt does on an agent worker

Runs, retries, and publishes its graph — including a per-run snapshot for a
dynamic descriptor, which it POSTs to `/api/agent_workers/dbt_graph/{workspace}`
rather than writing itself.

What it does not get is LIVE progress: the reporter tails a JSON event log and
needs a SQL connection, so per-model state is settled from `run_results.json`
when the run ends. Retry state lives only in the worker-local generation, since
there is no database row to arbitrate against — which is why `state_dir` is keyed
by principal.

Confirming a run really exercised that path:

```sql
SELECT job_id, count(*) FROM dbt_node WHERE script_path = '<path>' GROUP BY job_id;
-- a row keyed to the JOB id (not the zero UUID) means the agent published a snapshot
```
