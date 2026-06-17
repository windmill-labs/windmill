/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";
import { Action } from "./action.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";
import { VERSION, createBenchScript } from "./lib.ts";
import { MinikubeProvisioner } from "./sim/k8s_provisioner.ts";
import { capturePgLog, enableVerbosePgLogging } from "./sim/pg_logging.ts";
import { runPgbadger } from "./sim/pgbadger.ts";
import { collectCpuSamples } from "./sim/cpu_sampler_k8s.ts";
import { capturePodInventory } from "./sim/pod_inventory.ts";
import { startPodTimeline, type PodTimelinePoller } from "./sim/pod_timeline.ts";
import { checkReadiness, waitForReady } from "./sim/readiness.ts";
import { startOomPoller, type OomPoller } from "./sim/oom_poller.ts";
import { startPgLatencyPoller, type PgLatencyPoller } from "./sim/pg_latency_poller.ts";
import { startPgConnPoller, type PgConnPoller } from "./sim/pg_conn_poller.ts";
import { startNodeLoadPoller, type NodeLoadPoller } from "./sim/node_load_poller.ts";
import { collectOomEvents } from "./sim/oom_events.ts";
import { collectFailedJobs } from "./sim/failed_jobs.ts";
import { renderReport } from "./sim/render_report.ts";
export {
  DenoLandProvider,
  UpgradeCommand,
} from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

async function login(email: string, password: string): Promise<string> {
  return await windmill.UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });
}

export async function main({
  host,
  workers: num_workers,
  seconds,
  timeout,
  email,
  password,
  token,
  workspace,
  metrics,
  exportJson,
  exportCsv,
  exportHistograms,
  exportSimple,
  histogramBuckets,
  maximumThroughput,
  useFlows,
  flowPattern,
  scriptPattern,
  zombieTimeout,
  continous,
  max,
  custom,
  minikubeProfile,
  workloadConfig: workloadConfigPath,
  waitReady,
}: {
  host: string;
  workers: number;
  seconds: number;
  timeout?: number;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  metrics: string;
  exportJson?: string;
  exportCsv?: string;
  exportHistograms?: string[];
  exportSimple?: string[];
  histogramBuckets: string[];
  maximumThroughput: number;
  useFlows?: boolean;
  flowPattern?: string;
  scriptPattern?: string;
  zombieTimeout: number;
  continous?: boolean;
  max?: number;
  custom?: string;
  // When set, bench captures cluster-side measurements (per-node CPU/mem,
  // pod inventory, PG log) via kubectl scoped to the bench window, and
  // composes the full dashboard. Without it, only Throughput + Queue depth.
  minikubeProfile?: string;
  // Path to a JSON workload config — required for `--script-pattern random`.
  // Defines per-parameter distributions (ram_mb, duration_ms, mode).
  workloadConfig?: string;
  // Seconds to poll cluster readiness before firing. 0 = single check, fail
  // immediately on any unmet condition. >0 = poll every 5s until ready or
  // timeout. See sim/readiness.ts.
  waitReady?: number;
}) {
  windmill.setClient("", host);
  const versionResp = await fetch(`${host}/api/version`);
  console.log("Backend version: " + (await versionResp.text()));

  const custom_content: Action | undefined = custom
    ? JSON.parse(await Deno.readTextFile(custom))
    : undefined;

  if (!Array.isArray(histogramBuckets)) {
    histogramBuckets = [];
  }

  if (!Array.isArray(exportHistograms)) {
    exportHistograms = [];
  }

  if (!Array.isArray(exportSimple)) {
    exportSimple = [];
  }

  let metrics_worker: Worker | undefined = undefined;
  if (!continous) {
    if (exportJson || exportCsv) {
      metrics_worker = new Worker(
        new URL("./scraper.ts", import.meta.url).href,
        {
          type: "module",
        }
      );

      metrics_worker.postMessage({
        exportHistograms,
        histogramBuckets,
        exportSimple,
        host: metrics,
      });
    }
  }

  console.log(
    "Started with options",
    JSON.stringify(
      {
        host,
        num_workers,
        seconds,
        email,
        workspace,
        metrics,
        exportJson,
        exportCsv,
        exportHistograms,
        exportSimple,
        maximumThroughput,
        useFlows,
        flowPattern,
        scriptPattern,
        zombieTimeout,
        continous,
      },
      null,
      4
    )
  );

  const config = {
    token: "",
    server: host,
    workspace_id: workspace,
  };

  let final_token: string;
  if (!token) {
    if (email && password) {
      console.log("Logging in with email and password...");
      final_token = await login(email, password);
      console.log("Logged in!");
    } else {
      console.error("Token or email with password are required.");
      return;
    }
  } else {
    final_token = token;
  }

  console.log("Using token", final_token);

  config.token = final_token;
  windmill.setClient(final_token, host);

  const per_worker_throughput = maximumThroughput / num_workers;
  const max_per_worker = max ? max / num_workers : undefined;
  // For `--script-pattern random`: load the workload config so each worker
  // can sample (ram_mb, duration_ms, mode) per job push from the configured
  // distributions. Other patterns ignore this.
  let workloadConfig: unknown = undefined;
  if (scriptPattern === "random") {
    if (!workloadConfigPath) {
      throw new Error("--script-pattern random requires --workload-config <path>");
    }
    workloadConfig = JSON.parse(await Deno.readTextFile(workloadConfigPath));
    console.log(`Loaded workload config from ${workloadConfigPath}`);
  }

  // Phased workload: take ownership of --seconds and --timeout so every phase
  // gets to run. --seconds becomes the sum of phase durations; --timeout gets
  // bumped to phase-sum + 30s drain budget if the user passed a smaller value.
  // Without this, a short --timeout would silently cut off later phases.
  const phasedCfg = workloadConfig as { phases?: Array<{ duration_s: number; pushers: number; name?: string }> } | undefined;
  if (phasedCfg?.phases && phasedCfg.phases.length > 0) {
    const phaseSumS = phasedCfg.phases.reduce((a, p) => a + p.duration_s, 0);
    const maxPushers = phasedCfg.phases.reduce((a, p) => Math.max(a, p.pushers), 0);
    console.log(`[phased] ${phasedCfg.phases.length} phase(s), total push window = ${phaseSumS}s`);
    for (const p of phasedCfg.phases) {
      console.log(`  - ${p.name ?? "(unnamed)"}: ${p.duration_s}s @ ${p.pushers} pushers`);
    }
    if (num_workers < maxPushers) {
      console.warn(`[phased] --workers ${num_workers} is below max phase pushers (${maxPushers}); some phases will be undersaturated. Bumping --workers to ${maxPushers}.`);
      num_workers = maxPushers;
    }
    if (seconds !== phaseSumS) {
      console.log(`[phased] overriding --seconds ${seconds} -> ${phaseSumS} (phase total)`);
      seconds = phaseSumS;
    }
    const minTimeoutS = phaseSumS + 30; // 30s drain budget for queue to empty
    if (timeout !== undefined && timeout < minTimeoutS) {
      console.warn(`[phased] --timeout ${timeout}s would cut off later phases (need >= ${minTimeoutS}s for phases + drain). Bumping to ${minTimeoutS}s.`);
      timeout = minTimeoutS;
    }
  }

  const shared_config = {
    server: host,
    token: final_token,
    workspace_id: config.workspace_id,
    per_worker_throughput,
    max_per_worker,
    useFlows,
    flowPattern,
    scriptPattern,
    continous,
    custom: custom_content,
    workloadConfig,
  };

  if (
    !useFlows &&
    (scriptPattern === undefined ||
      ["deno", "deno_sleep_150", "random", "python", "go", "bash", "bun", "dedicated"].includes(
        scriptPattern
      ))
  ) {
    await createBenchScript(scriptPattern || "deno", workspace);
  }

  let workers: Worker[] = new Array(num_workers);
  for (let i = 0; i < num_workers; i++) {
    workers[i] = new Worker(new URL("./worker.ts", import.meta.url).href, {
      type: "module",
    });
  }

  let start: number | undefined = undefined;

  const jobsSent = Array(num_workers).fill(0);
  const enc = (s: string) => new TextEncoder().encode(s);

  async function getQueueCount() {
    return (
      await (
        await fetch(
          config.server + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length;
  }

  const initial_queue_length = await getQueueCount();

  console.log("Initial queue length:", initial_queue_length);

  // Throughput-over-time samples captured at each updateState tick. Stored as
  // raw cumulative counts; the report renderer computes the windowed rate.
  // Window-from-start cumulative numbers get diluted by warmup → useless for
  // dashboard purposes; the renderer does a rolling-window diff instead.
  const throughputSamples: { ts: number; processed: number; sum: number; queue: number }[] = [];

  const updateState = setInterval(async () => {
    const elapsed = start ? Math.ceil((Date.now() - start) / 1000) : 0;
    const sum = jobsSent.reduce((a, b) => a + b, 0);
    let queue_length = -1;
    while (queue_length === -1) {
      try {
        queue_length = await getQueueCount();
      } catch (e) {
        console.log(
          `queue count not reachable. waiting...                                                           `
        );
        await sleep(0.5);
        continue;
      }
    }
    // Only sample after the start clock is armed so warmup ticks aren't
    // pulled into the windowed series.
    if (start !== undefined) {
      throughputSamples.push({
        ts: Date.now(),
        processed: sum - queue_length,
        sum,
        queue: queue_length,
      });
    }
    await Deno.stdout.write(
      enc(
        `elapsed: ${elapsed}/${seconds} | jobs sent: ${JSON.stringify(
          jobsSent
        )} (sum: ${sum} thr: ${(sum / elapsed).toFixed(2)}) - processed (sum: ${
          sum - queue_length
        } thr: ${((sum - queue_length) / elapsed).toFixed(
          2
        )}) | queue: ${queue_length}                          \r`
      )
    );
  }, 100);

  // Re-apply pgBadger PG settings before each bench. They live in PGDATA's
  // postgresql.auto.conf, which a fresh PVC (helm upgrade flipping persistence,
  // chart-bundled PG image upgrade) silently wipes. Settings are SIGHUP-able,
  // so this is a no-cost no-op when they're already set.
  if (minikubeProfile) {
    try {
      const provForPg = new MinikubeProvisioner({ profile: minikubeProfile });
      await enableVerbosePgLogging(provForPg);
    } catch (e) {
      console.warn(`[bench] could not enable verbose PG logging: ${(e as Error).message}`);
    }
  }

  // Pre-flight readiness check — MUST run BEFORE worker.postMessage() below
  // (which triggers pushers). Earlier versions ran this after postMessage and
  // the workers pushed thousands of jobs into PG during the wait window,
  // making `queue=0` impossible to ever observe → the check timed out
  // every time even on a perfectly healthy cluster.
  if (minikubeProfile) {
    const provForReady = new MinikubeProvisioner({ profile: minikubeProfile });
    const waitSec = (waitReady ?? 0) as number;
    const report = waitSec > 0
      ? await waitForReady(provForReady, { timeoutMs: waitSec * 1000 })
      : await checkReadiness(provForReady);
    console.log(`[readiness] samplers=${report.details.samplers_running}/${report.details.samplers_total}, workers=${report.details.workers_ready}/${report.details.workers_total}, pg=${report.details.pg_phase}(responsive=${report.details.pg_responsive}), queue=${report.details.queue_depth}, tox=${report.details.toxiproxy_ready}, app=${report.details.app_ready}`);
    if (!report.ready) {
      console.error("[readiness] cluster NOT ready — refusing to start bench:");
      for (const issue of report.issues) console.error(`  - ${issue}`);
      console.error("(re-run with --wait-ready <seconds> to poll until ready, or fix the issues and retry.)");
      Deno.exit(2);
    }
    console.log("[readiness] cluster ready, proceeding");
  }

  // Truncate the sampler host-log file on every worker node so this bench
  // gets a clean file. The sampler dual-writes to /var/log/wm-sim-cpu-sampler/
  // sampler.tsv via hostPath; without rotation it grows unbounded across runs
  // (~1.7 GB/day per node under load) and stale data from prior benches mixes
  // with the current run's collection. The collector still filters by
  // ts_ns ≥ bench_start_ms, so failure to truncate is recoverable — best-
  // effort only.
  if (minikubeProfile) {
    try {
      const provForRotate = new MinikubeProvisioner({ profile: minikubeProfile });
      const nodesRes = await provForRotate.kubectl([
        "get", "nodes",
        "-o", "jsonpath={range .items[*]}{.metadata.name}{\"|\"}{.status.addresses[?(@.type==\"InternalIP\")].address}{\"\\n\"}{end}",
      ]);
      if (nodesRes.code === 0) {
        const home = Deno.env.get("HOME") ?? "";
        await Promise.all(
          nodesRes.stdout.split("\n").filter(Boolean).map(async (line) => {
            const [name, ip] = line.split("|");
            if (!name || !ip) return;
            const key = `${home}/.minikube/machines/${name.trim()}/id_rsa`;
            const cmd = new Deno.Command("ssh", {
              args: [
                "-o", "StrictHostKeyChecking=no",
                "-o", "UserKnownHostsFile=/dev/null",
                "-o", "ConnectTimeout=5",
                "-o", "LogLevel=ERROR",
                "-i", key,
                `docker@${ip.trim()}`,
                "sudo truncate -s 0 /var/log/wm-sim-cpu-sampler/sampler.tsv 2>/dev/null || true",
              ],
              stdout: "null",
              stderr: "null",
            });
            await cmd.output();
          }),
        );
        console.log("[bench] sampler host-log truncated on all nodes");
      }
    } catch (e) {
      console.warn(`[bench] sampler host-log truncate failed (continuing): ${(e as Error).message}`);
    }
  }

  workers.forEach((worker, i) => {
    worker.addEventListener("message", (evt: MessageEvent<any>) => {
      if (evt.data.type === "jobs_sent") {
        jobsSent[i] = evt.data.jobs_sent;
      }
    });
    worker.postMessage({ ...shared_config, i });
  });

  // outDir is created BEFORE the bench begins so the in-bench pollers (pod
  // timeline, OOM live) can stream JSONL into it as the run progresses.
  // Previously this was created post-bench and the pollers crashed with a
  // TDZ "Cannot access 'outDir' before initialization" — silently failing,
  // which is why pod_timeline.jsonl was missing on every report and the
  // Workers-per-node panel fell back to the (wrong) cgroup-derived count.
  const benchStartIso = new Date(start ?? Date.now()).toISOString();
  const isoStamp = new Date().toISOString().replace(/[:.]/g, "-").replace(/Z$/, "");
  const outDir = `reports/${isoStamp}`;
  await Deno.mkdir(outDir, { recursive: true });
  console.log(`[bench] reports dir: ${outDir}`);

  // Pin bench-context metadata into the report so the dashboard header can
  // surface it. Includes paths so the report dir is self-describing.
  try {
    const meta = {
      topology: minikubeProfile ?? "n/a",
      host,
      workspace,
      bench_cmd: ["main.ts", ...Deno.args].join(" "),
      workload_path: workloadConfigPath ?? null,
      script_pattern: scriptPattern ?? null,
      ts_iso: new Date().toISOString(),
      // Shared relative-time origin. Every chart in render_report.ts uses this
      // as "0s" on its x-axis, so identical x positions across panels mean the
      // same wall-clock moment. Without this, each panel picked its own origin
      // from its earliest sample and panels drifted by tens of seconds (poller
      // startup vs first-push vs first-CPU-sample timings).
      bench_start_ms: Date.parse(benchStartIso),
    };
    await Deno.writeTextFile(`${outDir}/meta.json`, JSON.stringify(meta, null, 2));
  } catch (e) {
    console.warn(`[bench] meta.json write failed: ${(e as Error).message}`);
  }

  // Start the kubectl-based pod-timeline poller (1Hz) so the renderer can plot
  // a truly-Ready workers-per-node line — the cgroup sampler's view conflates
  // "container slice exists" with "worker is healthy", so a CrashLoopBackOff
  // cluster looks fully staffed when it isn't.
  let podTimelinePoller: PodTimelinePoller | undefined;
  let oomPoller: OomPoller | undefined;
  let pgLatencyPoller: PgLatencyPoller | undefined;
  let pgConnPoller: PgConnPoller | undefined;
  let nodeLoadPoller: NodeLoadPoller | undefined;
  if (minikubeProfile) {
    try {
      const provForPoll = new MinikubeProvisioner({ profile: minikubeProfile });
      podTimelinePoller = startPodTimeline(provForPoll, `${outDir}/pod_timeline.jsonl`);
    } catch (e) {
      console.warn(`[bench] pod timeline poller failed to start: ${(e as Error).message}`);
    }
    // OOM poller — catches container OOMKills as they happen so the end-of-
    // bench scan (which only sees the MOST RECENT lastState.terminated) can
    // merge in mid-bench kills that have already been overwritten by the
    // post-restart Running state.
    try {
      const provForOom = new MinikubeProvisioner({ profile: minikubeProfile });
      oomPoller = startOomPoller(provForOom, `${outDir}/oom_events_live.jsonl`);
    } catch (e) {
      console.warn(`[bench] OOM poller failed to start: ${(e as Error).message}`);
    }
    // PG latency poller — every 1s runs `SELECT 1` against PG, records wall-
    // clock latency. Surfaces PG-responsiveness over time on a dedicated
    // panel: flat baseline when healthy, spikes when connection storm/
    // contention hits.
    try {
      const provForPg = new MinikubeProvisioner({ profile: minikubeProfile });
      pgLatencyPoller = startPgLatencyPoller(provForPg, `${outDir}/pg_latency.jsonl`);
    } catch (e) {
      console.warn(`[bench] PG latency poller failed to start: ${(e as Error).message}`);
    }
    try {
      const provForConn = new MinikubeProvisioner({ profile: minikubeProfile });
      pgConnPoller = startPgConnPoller(provForConn, `${outDir}/pg_connections.jsonl`);
    } catch (e) {
      console.warn(`[bench] PG conn poller failed to start: ${(e as Error).message}`);
    }
    try {
      const provForLoad = new MinikubeProvisioner({ profile: minikubeProfile });
      nodeLoadPoller = startNodeLoadPoller(provForLoad, `${outDir}/node_load.jsonl`);
    } catch (e) {
      console.warn(`[bench] node-load poller failed to start: ${(e as Error).message}`);
    }
  }

  start = Date.now();

  // Hard wall-clock deadline for the whole bench (push + drain). When --timeout
  // is set, every blocking await below caps at the remaining time so the bench
  // always terminates predictably, regardless of how long the queue takes to
  // drain. Whatever was sampled by then is what the report uses.
  const deadlineMs = timeout !== undefined ? start + timeout * 1000 : Infinity;
  const remainingS = () => Math.max(0, (deadlineMs - Date.now()) / 1000);

  console.log("collecting samples...");
  if (continous) {
    while (true) {
      await sleep(Infinity);
    }
  }

  await sleep(Math.min(seconds, remainingS()));

  let sum = jobsSent.reduce((a, b) => a + b, 0);
  await Deno.stdout.write(
    enc(" ".padStart(30) + `\rduration: ${seconds} | jobs sent: ${sum}\n`)
  );

  // Tell pusher workers to stop. Sampler interval keeps running through the
  // drain phase so the throughput / queue panels show the queue actually
  // emptying instead of stopping at push-end.
  const shutdown_start = Date.now();
  workers.forEach((worker, i) => {
    const l = (evt: MessageEvent<any>) => {
      if (evt.data.type === "done") {
        worker.removeEventListener("message", l);
        workers = workers.filter((w) => w != worker);
        jobsSent[i] = evt.data.jobs_sent;
        worker.terminate();
      }
    };
    worker.addEventListener("message", l);
    worker.postMessage("done");
  });

  console.log("waiting for shutdown\n");
  while (workers.length > 0 && remainingS() > 0) {
    await sleep(0.1);
  }
  if (workers.length > 0) {
    console.log(`\n[timeout] ${workers.length} driver-worker(s) still running — terminating`);
    workers.forEach((w) => w.terminate());
    workers = [];
  }

  // Drain phase: keep waiting until the queue is empty so the metric reflects
  // sustained throughput, not "jobs queued / push window." Backstop is 10x the
  // push duration — bails if the workload genuinely can't keep up so the run
  // doesn't hang forever. --timeout still applies as the outer wallclock cap.
  const drainStart = Date.now();
  const drainDeadlineMs = drainStart + seconds * 10 * 1000;
  let queue_length = await getQueueCount();
  while (queue_length > 0 && remainingS() > 0 && Date.now() < drainDeadlineMs) {
    await sleep(0.1);
    try {
      queue_length = await getQueueCount();
    } catch (_) { /* transient — keep going */ }
    await Deno.stdout.write(enc(`draining: queue=${queue_length}                    \r`));
  }
  clearInterval(updateState);
  if (queue_length > 0) {
    const reason = remainingS() <= 0 ? "--timeout reached" : "10x push duration safety cap";
    console.log(`\n[drain] ${queue_length} job(s) still pending (${reason}) — reporting what's done`);
  }

  sum = jobsSent.reduce((a, b) => a + b, 0);
  const drainTime = (Date.now() - drainStart) / 1000;
  const totalTime = (Date.now() - start) / 1000;
  const processed = sum - queue_length;

  const pushRate = sum / seconds;
  const sustainedRate = processed / totalTime;

  console.log("\nshutdown wait (s):", (drainStart - shutdown_start) / 1000);
  console.log("drain time (s):", drainTime);
  console.log("total wall time (s):", totalTime);
  console.log("jobs sent:", sum, "  processed:", processed, "  remaining:", queue_length);
  console.log("push rate (sent/push_seconds):", pushRate.toFixed(2));
  console.log("sustained throughput (processed/total_time):", sustainedRate.toFixed(2));

  console.log(
    "queue length:",
    (
      await (
        await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length
  );

  if (metrics_worker) {
    metrics_worker.postMessage("stop");
    console.log("waiting for metrics");
    const { columns, transfer_values } = await new Promise<{
      columns: string[];
      transfer_values: ArrayBufferLike[];
    }>((resolve, _reject) => {
      if (metrics_worker) {
        metrics_worker.onmessage = (e) => {
          resolve(e.data);
          metrics_worker?.terminate();
        };
      }
    });
    const values = transfer_values.map((x) => new Float32Array(x));

    if (exportJson) {
      console.log("exporting mean & stdev to json");
      const obj: any = {};
      for (let i = 0; i < columns.length; i++) {
        const name = columns[i]!;
        const value = values[i]!;
        const mean = value.reduce((acc, e) => acc + e, 0) / values.length;
        const stdev = Math.sqrt(
          value.reduce((acc, e) => acc + (e - mean) ** 2) / values.length
        );
        obj[name] = { mean, stdev };
      }

      await Deno.writeTextFile(exportJson, JSON.stringify(obj));
    }

    if (exportCsv) {
      const f = await Deno.open(exportCsv, {
        write: true,
        create: true,
        truncate: true,
      });
      const encoder = new TextEncoder();
      const newline = new Uint8Array(1);
      newline[0] = 0x0a;
      await f.write(encoder.encode(columns.join(",")));
      await f.write(newline);

      for (let i = 0; i < values.length; i++) {
        await f.write(encoder.encode(values[i].join(",")));
        await f.write(newline);
      }

      f.close();
    }
  }
  // ---------- post-bench: capture cluster measurements + render report ----------
  // outDir + benchStartIso were declared above (pre-bench) so pollers could
  // stream into them as the run progressed.

  if (throughputSamples.length > 0) {
    await Deno.writeTextFile(
      `${outDir}/throughput_samples.json`,
      JSON.stringify(throughputSamples),
    );
  }

  // Persist the workload config alongside the report so the renderer can show
  // the per-parameter distributions.
  if (workloadConfig) {
    await Deno.writeTextFile(
      `${outDir}/workload.json`,
      JSON.stringify(workloadConfig, null, 2),
    );
  }

  // Cluster-side capture only if the caller pointed us at a minikube profile.
  let cpusPerNode: number | Record<string, number> = 1;
  if (minikubeProfile) {
    const prov = new MinikubeProvisioner({ profile: minikubeProfile });
    // Cores per node — used by the renderer to scale Node CPU to % of VM.
    // Heterogeneous topologies (small tainted control plane + larger workers)
    // need a per-node map so each node is normalized against its own capacity.
    try {
      const r = await prov.kubectl([
        "get", "nodes",
        "-o", "jsonpath={range .items[*]}{.metadata.name}={.status.capacity.cpu}{\"\\n\"}{end}",
      ]);
      const byNode: Record<string, number> = {};
      for (const line of r.stdout.split("\n")) {
        const [name, cpuStr] = line.split("=");
        const n = parseInt((cpuStr ?? "").trim());
        if (name && Number.isFinite(n) && n > 0) byNode[name.trim()] = n;
      }
      if (Object.keys(byNode).length > 0) cpusPerNode = byNode;
    } catch { /* leave default 1 */ }
    // Stop the pod-timeline poller before grabbing the final inventory — both
    // write to outDir and we want the poller's last sample on disk first.
    if (podTimelinePoller) {
      podTimelinePoller.cont.value = false;
      try {
        await podTimelinePoller.done;
      } catch (e) {
        console.warn(`[bench] pod timeline finalize failed: ${(e as Error).message}`);
      }
    }
    if (oomPoller) {
      oomPoller.cont.value = false;
      try {
        await oomPoller.done;
      } catch (e) {
        console.warn(`[bench] OOM poller finalize failed: ${(e as Error).message}`);
      }
    }
    if (pgLatencyPoller) {
      pgLatencyPoller.cont.value = false;
      try {
        await pgLatencyPoller.done;
      } catch (e) {
        console.warn(`[bench] PG latency poller finalize failed: ${(e as Error).message}`);
      }
    }
    if (pgConnPoller) {
      pgConnPoller.cont.value = false;
      try {
        await pgConnPoller.done;
      } catch (e) {
        console.warn(`[bench] PG conn poller finalize failed: ${(e as Error).message}`);
      }
    }
    if (nodeLoadPoller) {
      nodeLoadPoller.cont.value = false;
      try {
        await nodeLoadPoller.done;
      } catch (e) {
        console.warn(`[bench] node-load poller finalize failed: ${(e as Error).message}`);
      }
    }
    try {
      await capturePodInventory(prov, `${outDir}/pods.json`);
    } catch (e) {
      console.warn(`[bench] pod inventory failed: ${(e as Error).message}`);
    }
    try {
      await collectCpuSamples(prov, `${outDir}/cpu_samples.tsv`, { sinceTime: benchStartIso });
    } catch (e) {
      console.warn(`[bench] CPU sample capture failed: ${(e as Error).message}`);
    }
    try {
      await collectOomEvents(prov, `${outDir}/oom_events.json`, { sinceMs: start });
    } catch (e) {
      console.warn(`[bench] OOM event capture failed: ${(e as Error).message}`);
    }
    try {
      await collectFailedJobs({
        host,
        token: final_token,
        workspace,
        sinceMs: start,
        outPath: `${outDir}/failed_jobs.jsonl`,
      });
    } catch (e) {
      console.warn(`[bench] failed-jobs capture failed: ${(e as Error).message}`);
    }
  } else {
    console.log(`[bench] --minikube-profile not set — skipping cluster-side panels (CPU/mem/workers/PG)`);
  }

  // Render the dashboard FIRST — it depends only on cpu_samples + pods +
  // throughput. pgBadger is slow (~30s parse on a busy run's 50MB log) and
  // produces a separate HTML; do it last so the dashboard is openable
  // immediately.
  try {
    const benchWalltimeS = (Date.now() - start) / 1000;
    await renderReport({
      outDir,
      topology: "bench",
      walltimeS: benchWalltimeS,
      finalThroughput: sustainedRate,
      cpusPerNode,
    });
  } catch (e) {
    console.warn(`[bench] report render failed: ${(e as Error).message}`);
  }

  // Background the pg-log capture + pgBadger render — the kubectl logs pull
  // on a busy run takes ~40s and is dominated by network/log volume, not the
  // bench. Spawn a detached subshell so wm-bench can exit while pgbadger
  // continues; the report dir gets `pg.log` then `pgbadger.html` when ready.
  if (minikubeProfile) {
    const minikube = Deno.env.get("SIM_MINIKUBE_BIN") ?? "minikube";
    const pgbadger = Deno.env.get("SIM_PGBADGER_BIN") ?? "pgbadger";
    const logPath = `${outDir}/pg.log`;
    const htmlPath = `${outDir}/pgbadger.html`;
    const since = benchStartIso;
    // Subshell — kubectl logs piped to file, then pgbadger; backgrounded with
    // `&` so the parent sh exits immediately and the child becomes orphaned
    // (and inherited by init). Output muted so a closed terminal doesn't kill
    // it via SIGPIPE.
    const script =
      `( ${minikube} kubectl -p ${minikubeProfile} -- -n default logs ` +
      `-l app=windmill-postgresql-demo-app --all-containers=true --tail=-1 ` +
      `--since-time=${since} > "${logPath}" 2>/dev/null && ` +
      `${pgbadger} "${logPath}" -o "${htmlPath}" -q >/dev/null 2>&1 ) </dev/null >/dev/null 2>&1 &`;
    const cmd = new Deno.Command("sh", {
      args: ["-c", script],
      stdin: "null", stdout: "null", stderr: "null",
    });
    await cmd.output();
    console.log(`[bench] pg.log + pgbadger.html will be written to ${outDir}/ in the background.`);
  }

  console.log("done");
  return {
    throughput: sustainedRate,
    throughputSamples,
  };
}

// runWithTopology has moved entirely to `wm_sim up` — wm-bench no longer
// provisions clusters or installs helm. Point it at a running cluster via
// `--host` (and optionally `--minikube-profile` for the cluster-side panels).

if (import.meta.main) {
  await new Command()
    .name("wmillbench")
    .description("Run Benchmark to measure throughput of windmill.")
    .version(VERSION)
    .option("--host <url:string>", "The windmill host to benchmark.", {
      default: "http://127.0.0.1:8000",
    })
    .option(
      "--workers <workers:number>",
      "The number of workers to run at once.",
      {
        default: 1,
      }
    )
    .option(
      "-s --seconds <seconds:number>",
      "How long the workers push jobs for (in seconds).",
      {
        default: 30,
      }
    )
    .option(
      "--timeout <timeout:number>",
      "Hard wall-clock cap on the whole bench (push + drain). On hit, sampling stops and the report is rendered from what was collected. Default 120s.",
      { default: 120 }
    )
    .option("--max <max:number>", "Maximum number of operations performed.")
    .option("-e --email <email:string>", "The email to use to login.")
    .option("-p --password <password:string>", "The password to use to login.")
    .env(
      "WM_TOKEN=<token:string>",
      "The token to use when talking to the API server. Preferred over manual login."
    )
    .option(
      "-t --token <token:string>",
      "The token to use when talking to the API server. Preferred over manual login."
    )
    .env(
      "WM_WORKSPACE=<workspace:string>",
      "The workspace to spawn scripts from."
    )
    .option(
      "-w --workspace <workspace:string>",
      "The workspace to spawn scripts from.",
      { default: "admins" }
    )
    .option(
      "-m --metrics <metrics:string>",
      "The url to scrape metrics from.",
      {
        default: "http://localhost:8001/metrics",
      }
    )
    .option(
      "--export-json <export_json:string>",
      "If set, exports will be into a JSON file."
    )
    .option(
      "--export-csv <export_csv:string>",
      "If set, exports will be into a csv file."
    )
    .option(
      "--export-histograms <export_histograms:string[]>",
      "Mark metrics (without label) that are reported as histograms to export."
    )
    .option(
      "--export-simple <export_simple:string[]>",
      "Mark metrics (without label) that are reported as simple values."
    )
    .option(
      "--maximum-throughput <maximum_throughput:number>",
      "Maximum number of jobs/flows to start in one second.",
      {
        default: Infinity,
      }
    )
    .option("--use-flows", "Run flows instead of jobs.")
    .option(
      "--flow-pattern <pattern:string>",
      "Use a different flow pattern among: 2steps, onebranch (Default 2steps)"
    )
    .option(
      "--script-pattern <pattern:string>",
      "Use a different script pattern among: deno, identity, python, go, bash, dedicated, bun (Default deno)"
    )
    .option("--custom <custom_path:string>", "Use custom actions during bench")
    .option(
      "--zombie-timeout <zombie_timeout:number>",
      "The maximum time in ms to wait for jobs to complete.",
      {
        default: 90000,
      }
    )
    .option(
      "-c --continuous",
      "Run the benchmark forever. This effectively disables metric collection & exports. No zombie jobs will be tracked."
    )
    .option(
      "--histogram-buckets <histogram_buckets:string[]>",
      "Define what buckets to collect from histograms.",
      {
        default: [
          "+Inf",
          "10",
          "5",
          "2.5",
          "2.5",
          "1",
          "0.5",
          "0.25",
          "0.1",
          "0.05",
          "0.025",
          "0.01",
          "0.005",
        ],
      }
    )
    .option("--hide-progress", "Hide worker progress logs")
    .option(
      "--minikube-profile <name:string>",
      "Enable cluster-side panels (per-node CPU/mem, workers per node, PG/pgBadger) by giving the minikube profile name. Required for the full dashboard; without it only Throughput + Queue depth are rendered.",
    )
    .option(
      "--workload-config <path:string>",
      "Path to a JSON workload config (distributions for ram_mb, duration_ms, mode). Required when --script-pattern is `random`. Examples in benchmarks/workloads/.",
    )
    .option(
      "--wait-ready <seconds:number>",
      "Before firing the bench, poll the cluster every 5s and only start once samplers (4/4 Running), workers (ready==replicas), PG (responsive), toxiproxy + app are healthy AND the queue is empty. Errors with the unmet conditions on timeout. Skip the check entirely with --wait-ready 0.",
      { default: 0 },
    )
    .action((opts: any) => main(opts))
    .command(
      "upgrade",
      new UpgradeCommand({
        main: "main.ts",
        args: [
          "--allow-net",
          "--allow-read",
          "--allow-write",
          "--allow-env",
          "--unstable",
        ],
        provider: new DenoLandProvider({ name: "wmillbench" }),
      })
    )
    .parse();
}
