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
  WORKER_GROUP=agentgrp WORKER_TAGS=dbt PORT=8499 ./target/debug/windmill
```

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

A dbt script whose profile comes from a Windmill resource (`$res:`) is **refused**
there, by design: the graph is keyed on the relations the profile resolves to, and
an agent can neither read the stored root to verify it nor re-ingest a new one, so
a profile edited since the last ingest would silently cascade from the wrong
relations. A project carrying its own `profiles.yml` has no such key and runs.

Beyond that, and independent of the profile: no live progress (the reporter needs
a SQL connection, so per-model state is settled from `run_results.json` at the
end), no per-run graph snapshot, and retry state that lives only in the
worker-local generation — which is why `state_dir` is keyed by principal.
