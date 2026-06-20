## The core idea: a two-stage prefetch pipeline so nobody ever waits

Jobs flow through four stages:

```
Postgres  →  [server-side buffer]  →  HTTP  →  [agent-side buffer]  →  worker loop
```

At **each** stage a background producer keeps the **next** consumer's buffer full *ahead of
demand*. In steady state every consumer pops a job that's already sitting in front of it and
never blocks on the stage behind it: the worker never waits on HTTP, and the server never waits
on Postgres. Everything below is in service of making that true, and then dealing with what's
left once it is.

**Baseline (stock).** None of this exists yet: the worker is a serial request/response loop —
pull one job (HTTP to the app), run it, report it (HTTP to the app), repeat — blocking on each
call. For a noop that's **2 HTTP round-trips per job and ~0 compute**: **2,539 j/s**, ~75% of the
cycle in the pull RPC, ~25% in the completion RPC. The whole problem is "stop paying a round-trip
per job," then "stop being limited by the database behind the app."

> All single-agent numbers below are from one environment: a single box, native, with app +
> bundled Postgres + agent all on `localhost`, prefill-then-drain, tables truncated per run.

## 1. Make the unit of transfer a batch, not a job (both sides)

- **Agent side:** ask for **N jobs in one request** and drain them from a local buffer; accumulate
  completions and send them back **N in one request** (flush triggered by either a size threshold
  or a short max-latency timer, so completion latency stays bounded at low rates).
- **Server side:** answer a pull with a single `LIMIT N … FOR UPDATE SKIP LOCKED` (one dequeue
  query yields N jobs), and accept N completions in one call.

This amortizes the fixed round-trip cost across N jobs (2 round-trips per *N* jobs) and collapses N
separate dequeue queries into one. → first cut ~23k j/s; ~62–64k once the bench method was clean.

## 2. Server-side prefetch — the server never queries Postgres on the request path

Even batched, a pull still triggers a fresh dequeue query *while the agent waits*. Fix: a **single
background "puller" task per job-tag-set** runs the dequeue query in a loop and fills an in-memory
buffer of ready jobs. An agent's pull is served **from that buffer**, not from a live query, and
multiple agents sharing the same tags are **coalesced onto the one puller** (so 25 agents don't
become 25 concurrent `SKIP LOCKED` queries fighting each other). The Postgres work moves off the
request's critical path and into the background refill.

## 3. Agent-side prefetch — the worker never waits on HTTP

Symmetrically on the agent: a **background refiller** pulls batches from the server ahead of demand
into the agent's local buffer, and the worker loop pops from that buffer with **no network call**.
The HTTP latency is now **overlapped with job execution** instead of serialized in front of it.
→ ~64k → ~89k.

## 4. Keep both buffers deep so prefetch actually stays ahead

A background producer only helps if its buffer never empties — otherwise the consumer falls back to
a blocking fetch and you've lost the benefit. Two rules, applied to the prefetchers:

- **Refill early, not on empty** — trigger the next refill when the buffer falls below a *high*
  low-watermark (several batches deep), not when it hits zero.
- **Allow several refills in flight at once** — a refill takes a few ms to land; if only one can be
  outstanding, a fast consumer drains the buffer before it arrives. Concurrent refills keep
  inventory above `drain-rate × refill-latency` with margin.

→ ~89k → ~127k; the agent essentially never blocks (popping + running a job is sub-µs to low-µs).
**At this point the single-agent path is solved.**

## 5. Where it stands: single agent in good shape, scaling up is the next focus

The two-stage prefetch pipeline takes a single agent from **2,539 j/s → ~125k+** — at that point it
essentially never blocks, and neither the agent nor the server sits on the network or Postgres on the
critical path. It can still be optimized further (the completion/write path isn't prefetched yet, and
there's per-job cost left to squeeze), but we haven't gone there — the more valuable direction is to
scale up, running many agents together, and that's where we turned next.

That's where the hard part is. Multiple agents don't yet scale cleanly — aggregate throughput is
unstable and doesn't multiply the way one well-fed agent predicts — and we've spent real effort trying
to pin down why **without success**. It's a stubborn problem that hasn't yielded to the instrumentation
we've thrown at it; the leads we have are unverified, and it's not established to be Postgres, possibly:

- An idle-style backoff sleep that fires on a transient empty pull, which a prefetching consumer hits
  under contention.
- Periodic exclusive-lock stalls from vacuum reclaiming the empty tail of the queue tables.

## Current state / next

1. **Single agent: in good shape** — fully prefetched on both sides, ~50× over stock, no blocking on
   the critical path; can still be improved, but with diminishing returns.
2. **Scaling to many agents: the open problem** — aggregate throughput doesn't multiply cleanly, and
   we've investigated without pinning the cause (likely not Postgres). This is where the priority is.
3. **Completion (write) path** is the one part of the agent not yet given the same async/prefetch
   treatment as the read path — a known candidate to harden, independent of the collapse.
4. **Everything is experimental** and gated off by default; nothing changes stock behavior. Any of
   it becoming a default needs the collapse understood first, plus a fairness/latency review of
   batching.

