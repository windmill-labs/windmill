// Per-node CPU sampler for the k8s sim path.
//
// A privileged DaemonSet (one pod per node) mounts the host's
// `/sys/fs/cgroup` and reads `cpu.stat` for every Windmill pod's cgroup at a
// configurable cadence (default 100 ms — busybox shell sleep granularity is
// the practical floor; finer than ~50 ms tends to drift. For true 10 ms
// fidelity we'd need a small Go/Rust binary, queued as a follow-on).
//
// Output is CSV on stdout, captured at end-of-bench via `kubectl logs`:
//   <ts_ns> <node> <pod_uid> <container_id> <usage_usec>
//
// The sampler walks `/sys/fs/cgroup/kubepods.slice/**/cpu.stat`. cgroup v2's
// hierarchy under kubepods.slice is:
//   kubepods.slice/
//     kubepods-besteffort.slice/
//       kubepods-besteffort-pod<pod-uid>.slice/
//         cri-containerd-<container-id>.scope/cpu.stat
//     kubepods-burstable.slice/...
//     kubepods.slice/kubepods-pod<pod-uid>.slice/...  (guaranteed)
// The sampler emits raw paths and lets the consumer post-process.

import { stringify as yamlStringify } from "https://deno.land/std@0.224.0/yaml/mod.ts";
import { MinikubeProvisioner } from "./k8s_provisioner.ts";

const NAMESPACE = "kube-system";
const DS_NAME = "wm-sim-cpu-sampler";
const IMAGE = "python:3.12-alpine";

// Single-process Python sampler. Replaces the prior bash loop that forked
// awk/cat/find every tick — under heavy node CPU pressure the shell pipeline
// alone could consume seconds at a stretch (m03 had 13.7s sample gaps even
// after Guaranteed QoS). Python's persistent FDs + os.read on lseek(0) costs
// ~0 between samples: ~1 syscall per file, no path resolution, no subprocess.
const SAMPLE_SCRIPT = `#!/usr/bin/env python3
import os, sys, time, glob

INTERVAL_S = float(os.environ.get("INTERVAL_S", "0.1"))
NODE_NAME  = os.environ.get("NODE_NAME", "unknown")
CG_HOST    = "/host-sys/fs/cgroup"
CG_KP      = CG_HOST + "/kubepods.slice"
PROC_MEM   = "/host-proc/meminfo"
NODE_TAG   = "__node_root__"
# Host-mounted append-only log. Bypasses kubelet's log file rotation —
# heavy benches were rotating early samples out of kubelet's 10MB×5
# window before the bench finished, leaving multi-minute gaps for the
# busiest node in the report. The collector scp's this file from each
# node at end-of-bench; kubectl logs is kept as a fallback path.
HOST_LOG   = "/host-logs/sampler.tsv"

# Rescan kubepods every N seconds to pick up new pods. Cheap glob walk,
# but no need to do it every tick — pod churn is slow vs the sample rate.
RESCAN_S = 5.0

# Two timestamps per row:
#   ts_ns   = CLOCK_REALTIME (wall) — needed for x-axis alignment with
#             other bench panels (throughput, OOM events, etc).
#   mono_ns = CLOCK_MONOTONIC — used by the consumer for delta math.
# Why both: in qemu/minikube guests the wall clock can step backwards on
# NTP correction, producing apparent dt < real_wall and impossible
# >100% per-VM CPU spikes (du/dt where du is real but dt is shrunk by
# the backwards clock jump). MONOTONIC is immune by spec.
print("ts_ns mono_ns node pod_slice container_scope usage_usec mem_bytes", flush=True)
# Open host log append-only; line-buffered. The header is printed only once
# per container life (re-opening on restart appends a new header — collector
# tolerates this since it filters by ts_ns >= bench_start).
host_log_fd = None
try:
    host_log_fd = open(HOST_LOG, "a", buffering=1)
    host_log_fd.write("ts_ns mono_ns node pod_slice container_scope usage_usec mem_bytes\\n")
except OSError:
    host_log_fd = None

def parse_usage(fd):
    os.lseek(fd, 0, 0)
    # cpu.stat first line is "usage_usec <N>"; reads <1KiB
    buf = os.read(fd, 1024)
    # bytes split is faster than decode+split here
    nl = buf.find(b"\\n")
    line = buf if nl < 0 else buf[:nl]
    sp = line.find(b" ")
    return line[sp + 1:].decode() if sp > 0 else ""

def parse_meminfo(fd):
    os.lseek(fd, 0, 0)
    buf = os.read(fd, 4096)
    t = a = -1
    for line in buf.split(b"\\n"):
        if line.startswith(b"MemTotal:"):
            t = int(line.split()[1])
        elif line.startswith(b"MemAvailable:"):
            a = int(line.split()[1])
        if t >= 0 and a >= 0:
            break
    return str((t - a) * 1024) if (t >= 0 and a >= 0) else "-"

def read_mem(fd):
    os.lseek(fd, 0, 0)
    return os.read(fd, 64).strip().decode()

# Persistent FDs — open once, reuse across ticks.
root_cpu_fd = os.open(CG_HOST + "/cpu.stat", os.O_RDONLY)
mem_fd      = os.open(PROC_MEM, os.O_RDONLY)
# pod_slice -> (cpu_fd, mem_fd or None)
pods: dict[str, tuple[int, int | None]] = {}
last_rescan = 0.0

def rescan_pods():
    """Walk kubepods.slice once, find leaf pod cgroups, open their FDs."""
    seen = set()
    # Two-level glob: kubepods-<qos>-pod*.slice + kubepods-pod*.slice (guaranteed).
    for cpu_stat in glob.iglob(CG_KP + "/**/cpu.stat", recursive=True):
        d = os.path.dirname(cpu_stat)
        slice_name = os.path.basename(d)
        # Skip container scopes (cri-containerd-*.scope); we want pod slices.
        if not slice_name.endswith(".slice") or "-pod" not in slice_name:
            continue
        seen.add(slice_name)
        if slice_name in pods:
            continue
        try:
            cfd = os.open(cpu_stat, os.O_RDONLY)
        except OSError:
            continue
        mfd = None
        try:
            mfd = os.open(d + "/memory.current", os.O_RDONLY)
        except OSError:
            pass
        pods[slice_name] = (cfd, mfd)
    # Drop FDs for pods that disappeared (terminated).
    for name in list(pods.keys()):
        if name not in seen:
            cfd, mfd = pods.pop(name)
            try: os.close(cfd)
            except OSError: pass
            if mfd is not None:
                try: os.close(mfd)
                except OSError: pass

# Build output in-memory then single write — fewer syscalls than print-per-row.
out_buf = []
while True:
    now = time.monotonic()
    if now - last_rescan >= RESCAN_S:
        rescan_pods()
        last_rescan = now
    ts = time.time_ns()
    mono = time.monotonic_ns()
    try:
        ru = parse_usage(root_cpu_fd)
    except OSError:
        ru = "-"
    try:
        rm = parse_meminfo(mem_fd)
    except OSError:
        rm = "-"
    out_buf.append(f"{ts} {mono} {NODE_NAME} {NODE_TAG} - {ru} {rm}\\n")
    for slice_name, (cfd, mfd) in pods.items():
        try:
            u = parse_usage(cfd)
        except OSError:
            continue
        if not u:
            continue
        try:
            m = read_mem(mfd) if mfd is not None else "-"
        except OSError:
            m = "-"
        out_buf.append(f"{ts} {mono} {NODE_NAME} {slice_name} - {u} {m}\\n")
    blob = "".join(out_buf)
    sys.stdout.write(blob)
    sys.stdout.flush()
    if host_log_fd is not None:
        try:
            host_log_fd.write(blob)
        except OSError:
            pass
    out_buf.clear()
    time.sleep(INTERVAL_S)
`;

function configMapManifest(): string {
  return yamlStringify({
    apiVersion: "v1",
    kind: "ConfigMap",
    metadata: { name: DS_NAME, namespace: NAMESPACE },
    data: { "sample.py": SAMPLE_SCRIPT },
  });
}


function daemonSetManifest(intervalSeconds: number): string {
  return yamlStringify({
    apiVersion: "apps/v1",
    kind: "DaemonSet",
    metadata: { name: DS_NAME, namespace: NAMESPACE, labels: { app: DS_NAME } },
    spec: {
      selector: { matchLabels: { app: DS_NAME } },
      template: {
        metadata: { labels: { app: DS_NAME } },
        spec: {
          // Schedule onto every node including the control-plane.
          tolerations: [{ operator: "Exists" }],
          hostPID: true,
          // Same priorityClass as PG/app (wm-critical = 1B). Sampler is tiny
          // (25m/32Mi request) and PG/app are big, so they fit together on
          // any worker node trivially. The shared priority means scheduler
          // can preempt workers (priority 0) to land the sampler if the
          // request budget gets squeezed — and the sampler is never the
          // pod that loses out to PG/app.
          priorityClassName: "wm-critical",
          containers: [{
            name: "sampler",
            image: IMAGE,
            securityContext: { privileged: true },
            // Burstable QoS: requests give a CPU-scheduling floor, but we
            // INTENTIONALLY omit limits.cpu — a hard CPU cap throttles the
            // sampler via CFS, which interacts badly with NTP wall-clock
            // corrections to produce >100% per-VM artifacts (the sampler
            // emits ts immediately after a backwards clock jump while the
            // cgroup counter has accumulated normally).
            //
            // Memory: 256Mi limit was still OOM-looping on busy worker nodes
            // (m02/m03/m04 all hit exit 137 mid-bench, dropping CPU panels
            // for those nodes). Bumped 5× to 1.25Gi limit / 320Mi request —
            // way over what an idle sampler uses (~15Mi RSS) but the
            // observed restart pattern shows transient spikes the previous
            // ceiling couldn't absorb. CPU bumped 5× too (200m→1000m req)
            // so the sampler never CPU-throttles even under hot-node load.
            // Burstable QoS; combined with the postStart oom_score_adj=-999
            // below, the sampler is also last-to-pick for kernel OOM.
            // Request is intentionally small so this DaemonSet pod schedules
            // on heavily-booked worker nodes (where workers' 256Mi×N requests
            // eat most of the per-node request budget). Limit stays generous
            // so the sampler can burst when reading lots of cgroup files
            // under load. Burstable QoS.
            // Tiny requests so this DaemonSet pod can ALWAYS fit, even on
            // worker nodes where PG (cpu=3) + workers (33×50m) + other
            // pods leave only a sliver of CPU/mem request budget.
            // Without this, m02 (where PG lives) ran out of CPU request
            // budget and the m02 sampler stayed Pending the entire bench
            // → no m02 data → Workers-per-node chart wrong.
            //
            // Limit bumped to 4Gi after repeated OOMKilled/exit-137 with
            // peak usage <30Mi — likely node-kernel OOM picking the sampler
            // before its postStart -999 hook ran. Wide ceiling makes the
            // sampler an unappealing OOM victim.
            // 500m (half a core) reserved. Originally tried 1 full core but
            // m02 hosts PG (cpu=3) so 3+1=4 = the entire node, leaving zero
            // room for ANY worker AND the sampler couldn't even schedule
            // there (Pending). 500m fits comfortably on m02 (PG 3 + sampler
            // 0.5 + ~0.5 for workers = 4), still 20× the idle ~25m, and is
            // enough headroom that the 100ms tick won't CFS-throttle.
            //
            // No memory limit — cgroup OOM kept getting tripped under busy-
            // node memory pressure even at 4Gi. OOM-immunity now comes from
            // priorityClassName + oom_score_adj=-999 set inline at PID-1
            // startup.
            resources: {
              requests: { cpu: "500m", memory: "32Mi" },
            },
            // OOM-immunity: lower this container's PID-1 oom_score_adj to
            // -999 so the kernel picks anything else first under node memory
            // pressure. `privileged: true` already grants CAP_SYS_RESOURCE.
            // postStart-exec is safe here — python:3.12-alpine ships
            // /bin/sh, unlike distroless toxiproxy where we had to use a
            // sidecar approach.
            lifecycle: {
              postStart: {
                exec: { command: ["/bin/sh", "-c", "echo -999 > /proc/1/oom_score_adj || true"] },
              },
            },
            env: [
              { name: "INTERVAL_S", value: String(intervalSeconds) },
              {
                name: "NODE_NAME",
                valueFrom: { fieldRef: { fieldPath: "spec.nodeName" } },
              },
            ],
            // Setting oom_score_adj as the FIRST thing PID 1 does eliminates
            // a startup race the postStart hook couldn't: under node memory
            // pressure (m04 sits at ~98% committed), the kernel OOM-killer
            // can pick the sampler container BEFORE postStart fires, since
            // postStart runs after container start. Doing it inline in the
            // exec means the very first syscall the container makes is the
            // adj write, before any allocation that could trigger OOM.
            command: ["/bin/sh", "-c", "echo -999 > /proc/self/oom_score_adj 2>/dev/null || true; exec python3 -u /scripts/sample.py"],
            volumeMounts: [
              { name: "cgroup", mountPath: "/host-sys/fs/cgroup", readOnly: true },
              { name: "hostproc", mountPath: "/host-proc", readOnly: true },
              { name: "script", mountPath: "/scripts" },
              { name: "hostlogs", mountPath: "/host-logs" },
            ],
          }],
          volumes: [
            { name: "cgroup", hostPath: { path: "/sys/fs/cgroup", type: "Directory" } },
            { name: "hostproc", hostPath: { path: "/proc", type: "Directory" } },
            { name: "script", configMap: { name: DS_NAME, defaultMode: 0o755 } },
            // Append-only log dir on the node. DirectoryOrCreate makes
            // first-time minikube nodes happy. The sampler writes its TSV
            // here so the collector can scp it instead of relying on kubelet
            // log rotation (which loses data under heavy benches).
            { name: "hostlogs", hostPath: { path: "/var/log/wm-sim-cpu-sampler", type: "DirectoryOrCreate" } },
          ],
        },
      },
    },
  });
}

// Apply the sampler. Best-effort — if the cluster doesn't have a usable cgroup
// layout the sampler pods will run but produce empty output, which we tolerate.
export async function applyCpuSampler(
  prov: MinikubeProvisioner,
  outDir: string,
  opts: { intervalSeconds?: number } = {},
): Promise<void> {
  const intervalSeconds = opts.intervalSeconds ?? 0.1;
  const cm = configMapManifest();
  const ds = daemonSetManifest(intervalSeconds);
  const path = `${outDir}/cpu-sampler.yaml`;
  await Deno.writeTextFile(path, `${cm}---\n${ds}`);
  console.log(`[cpu] applying CPU sampler DaemonSet (interval ${intervalSeconds}s)`);
  const res = await prov.kubectl(["apply", "-f", path]);
  if (res.code !== 0) {
    throw new Error(`[cpu] kubectl apply failed: ${res.stdout}`);
  }
  await prov.kubectl([
    "-n", NAMESPACE,
    "rollout", "status", `daemonset/${DS_NAME}`,
    "--timeout=120s",
  ]);
  console.log("[cpu] sampler running on all nodes");
}

// Collect the sampler output as CSV. Captures all pods' logs across the
// DaemonSet via `kubectl logs --selector` so multi-node clusters are merged.
// `sinceTime` (ISO 8601, RFC 3339) scopes the output to the bench's wall
// window — the sampler runs continuously between bench runs, so without this
// every report would include all-time cumulative data.
export async function collectCpuSamples(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { sinceTime?: string } = {},
): Promise<void> {
  // Primary path: scp the hostPath log file from each node. This bypasses
  // kubelet log rotation (which loses early samples on heavily-loaded nodes —
  // saw m04 routinely missing the first ~120s of a bench because the kubelet
  // log file rotated past those rows before bench end). If scp succeeds for
  // all nodes we use that data; otherwise fall back to per-pod kubectl logs.
  const sinceNs = opts.sinceTime ? Date.parse(opts.sinceTime) * 1e6 : 0;
  const nodesRes = await prov.kubectl([
    "get", "nodes",
    "-o", "jsonpath={range .items[*]}{.metadata.name}{\"|\"}{.status.addresses[?(@.type==\"InternalIP\")].address}{\"\\n\"}{end}",
  ]);
  const nodes: { name: string; ip: string }[] = [];
  if (nodesRes.code === 0) {
    for (const line of nodesRes.stdout.split("\n")) {
      if (!line.trim()) continue;
      const [n, ip] = line.split("|");
      if (n && ip) nodes.push({ name: n.trim(), ip: ip.trim() });
    }
  }
  const home = Deno.env.get("HOME") ?? "";
  let scpFullSuccess = nodes.length > 0;
  const scpPieces: string[] = [];
  for (const { name: n, ip } of nodes) {
    const key = `${home}/.minikube/machines/${n}/id_rsa`;
    const cmd = new Deno.Command("ssh", {
      args: [
        "-o", "StrictHostKeyChecking=no",
        "-o", "UserKnownHostsFile=/dev/null",
        "-o", "ConnectTimeout=5",
        "-o", "LogLevel=ERROR",
        "-i", key,
        `docker@${ip}`,
        "sudo cat /var/log/wm-sim-cpu-sampler/sampler.tsv 2>/dev/null || true",
      ],
      stdout: "piped",
      stderr: "null",
    });
    try {
      const out = await cmd.output();
      const txt = new TextDecoder().decode(out.stdout);
      if (!txt) {
        scpFullSuccess = false;
        continue;
      }
      // Filter to bench window (ts_ns ≥ sinceNs), prefix lines with the pod
      // marker the renderer expects ([pod/wm-sim-cpu-sampler-...]).
      const filtered: string[] = [];
      const marker = `[pod/wm-sim-cpu-sampler-on-${n}/sampler]`;
      for (const line of txt.split("\n")) {
        if (!line) continue;
        const sp = line.indexOf(" ");
        if (sp < 0) continue;
        const tsStr = line.slice(0, sp);
        if (!/^\d+$/.test(tsStr)) continue; // header / non-data row
        if (sinceNs && Number(tsStr) < sinceNs) continue;
        filtered.push(marker + " " + line);
      }
      if (filtered.length > 0) scpPieces.push(filtered.join("\n") + "\n");
    } catch (_e) {
      scpFullSuccess = false;
    }
  }
  if (scpFullSuccess && scpPieces.length > 0) {
    await Deno.writeTextFile(outPath, scpPieces.join(""));
    console.log(`[cpu] collected from host log files via scp (${scpPieces.length} node(s))`);
    return;
  }
  console.warn(`[cpu] scp host-log fetch incomplete (success=${scpFullSuccess}, pieces=${scpPieces.length}) — falling back to kubectl logs`);

  // Discover the sampler pods so we can pull previous-container logs for any
  // that restarted mid-bench. The `-l` selector form only returns logs from
  // the *current* container — if a sampler pod restarted during the bench
  // window, samples from before the restart are silently dropped.
  const podsRes = await prov.kubectl([
    "-n", NAMESPACE,
    "get", "pods",
    "-l", `app=${DS_NAME}`,
    "-o", "jsonpath={range .items[*]}{.metadata.name}={.status.containerStatuses[0].restartCount}{\"\\n\"}{end}",
  ]);
  const pods: { name: string; restarts: number }[] = [];
  for (const line of podsRes.stdout.split("\n")) {
    const [name, rcStr] = line.split("=");
    const rc = parseInt((rcStr ?? "0").trim());
    if (name?.trim()) pods.push({ name: name.trim(), restarts: Number.isFinite(rc) ? rc : 0 });
  }
  if (pods.length === 0) {
    throw new Error(`[cpu] no sampler pods found via -l app=${DS_NAME}`);
  }

  const baseArgs = ["-n", NAMESPACE, "logs", "--tail=-1", "--prefix=true"];
  if (opts.sinceTime) baseArgs.push(`--since-time=${opts.sinceTime}`);

  const pieces: string[] = [];
  let restartedPodsWithPrev = 0;
  let failedPods = 0;
  // Per-pod fetch is resilient: a single sampler pod's kubectl-logs failing
  // (commonly transient TLS handshake timeout to a kubelet) must NOT abort
  // the whole capture — that was costing whole-bench reports (the renderer
  // skips when cpu_samples.tsv is missing entirely). Falling back to partial
  // data + warning is the right tradeoff.
  for (const p of pods) {
    if (p.restarts > 0) {
      try {
        const prev = await prov.kubectl([...baseArgs, p.name, "--previous"]);
        if (prev.code === 0 && prev.stdout) {
          pieces.push(prev.stdout);
          restartedPodsWithPrev++;
        }
      } catch (e) {
        console.warn(`[cpu] --previous fetch for ${p.name} failed: ${(e as Error).message}`);
      }
    }
    try {
      const cur = await prov.kubectl([...baseArgs, p.name]);
      if (cur.code !== 0) {
        console.warn(`[cpu] kubectl logs ${p.name} failed (code ${cur.code}): ${(cur.stderr || cur.stdout).slice(0, 200)} — skipping this pod`);
        failedPods++;
        continue;
      }
      pieces.push(cur.stdout);
    } catch (e) {
      console.warn(`[cpu] kubectl logs ${p.name} threw: ${(e as Error).message} — skipping this pod`);
      failedPods++;
    }
  }
  await Deno.writeTextFile(outPath, pieces.join(""));
  const tag = restartedPodsWithPrev > 0
    ? ` (recovered --previous logs from ${restartedPodsWithPrev} restarted pod(s))`
    : "";
  const failTag = failedPods > 0 ? ` — ${failedPods}/${pods.length} pod(s) FAILED, report will have partial data` : "";
  console.log(`[cpu] CPU samples captured -> ${outPath}${tag}${failTag}`);
}

export async function removeCpuSampler(prov: MinikubeProvisioner): Promise<void> {
  await prov.kubectl([
    "-n", NAMESPACE,
    "delete", "daemonset", DS_NAME,
    "--ignore-not-found",
  ]);
  await prov.kubectl([
    "-n", NAMESPACE,
    "delete", "configmap", DS_NAME,
    "--ignore-not-found",
  ]);
}
