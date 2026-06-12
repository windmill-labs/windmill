// minikube-backed provisioner for the k8s sim path.
//
// Brings up a local multi-node Kubernetes cluster from the existing topology
// format using minikube + the kvm2 driver (VM-per-node, which sidesteps the
// rootless-cgroup wall that blocks k3d/kind on this host). Windmill itself is
// deployed onto the cluster via helm; toxiproxy and the measurement layer sit
// on top. This module owns ONLY the cluster substrate: start, wait-ready,
// teardown.
//
// Topology mapping (the topology is the source of truth — no --nodes flag):
//   - Each topology node  ->  one minikube VM (substrate) node, sized from its
//     `cpu` (whole vCPUs) and `memory`. Node count = topology.nodes.length.
//   - Each node's `db_latency_ms` -> that node's RTT, applied via a per-node
//     toxiproxy (workers scheduled on the node route their DB traffic through
//     it). Handled in the toxiproxy step, not here.
//   - The 250 workers are PODS deployed by helm and spread across these nodes
//     by the scheduler — worker count/resources come from helm, not topology.
//
// kvm2 driver + libvirt resolution (flake integration deferred): the caller
// supplies these via env (or options):
//   SIM_KVM2_DRIVER_DIR  - dir containing `docker-machine-driver-kvm2`
//   SIM_LIBVIRT_LIB_DIR  - dir containing `libvirt.so.0` (-> LD_LIBRARY_PATH;
//                          the libvirt-go binding dlopens it by soname)
//   SIM_MINIKUBE_BIN     - minikube binary (default: `minikube` on PATH)

import { type Topology, type NodeSpec, parseMemory } from "./topology.ts";

const MINIKUBE = Deno.env.get("SIM_MINIKUBE_BIN") ?? "minikube";

export type K8sProvisionerOptions = {
  profile?: string; // minikube profile name; default "wm-sim"
  cni?: string; // default "flannel" — REQUIRED for cross-node pod networking
  kubernetesVersion?: string; // pin k8s version; optional
  driverDir?: string; // override SIM_KVM2_DRIVER_DIR
  libvirtLibDir?: string; // override SIM_LIBVIRT_LIB_DIR
};

export type ProvisionedCluster = {
  profile: string;
  nodes: string[]; // node names as kubectl sees them
};

// minikube wants whole vCPUs per VM and a plain MB memory value; the topology
// uses fractional historically and "16G"/"512M" strings. Normalise.
function cpusOf(n: NodeSpec): number {
  return Math.max(1, Math.round(n.cpu));
}
function memMbOf(n: NodeSpec): string {
  return `${Math.round(parseMemory(n.memory) / (1024 * 1024))}mb`;
}

export class MinikubeProvisioner {
  readonly profile: string;
  private readonly cni: string;
  private readonly kubernetesVersion?: string;
  private readonly driverDir?: string;
  private readonly libvirtLibDir?: string;

  constructor(opts: K8sProvisionerOptions = {}) {
    this.profile = opts.profile ?? "wm-sim";
    this.cni = opts.cni ?? "flannel";
    this.kubernetesVersion = opts.kubernetesVersion;
    this.driverDir = opts.driverDir;
    this.libvirtLibDir = opts.libvirtLibDir;
  }

  // Environment for every minikube invocation: prepend the kvm2 driver dir to
  // PATH and set LD_LIBRARY_PATH so the libvirt-go dlopen resolves.
  private env(): Record<string, string> {
    const base = Deno.env.toObject();
    const driverDir = this.driverDir ?? Deno.env.get("SIM_KVM2_DRIVER_DIR");
    const libDir = this.libvirtLibDir ?? Deno.env.get("SIM_LIBVIRT_LIB_DIR");
    if (driverDir) base.PATH = `${driverDir}:${base.PATH ?? ""}`;
    if (libDir) {
      base.LD_LIBRARY_PATH = base.LD_LIBRARY_PATH
        ? `${libDir}:${base.LD_LIBRARY_PATH}`
        : libDir;
    }
    return base;
  }

  private async run(
    args: string[],
    { check = true }: { check?: boolean } = {},
  ): Promise<{ stdout: string; stderr: string; code: number }> {
    const p = new Deno.Command(MINIKUBE, {
      args,
      env: this.env(),
      stdout: "piped",
      stderr: "piped",
    });
    const { code, stdout, stderr } = await p.output();
    const out = new TextDecoder().decode(stdout);
    const err = new TextDecoder().decode(stderr);
    if (check && code !== 0) {
      throw new Error(`minikube ${args.join(" ")} failed (code ${code}):\n${err || out}`);
    }
    return { stdout: out, stderr: err, code };
  }

  // `minikube kubectl -p <profile> -- <args>` — minikube's bundled kubectl, so
  // we don't depend on a separate kubectl on PATH.
  async kubectl(args: string[]): Promise<{ stdout: string; stderr: string; code: number }> {
    const { stdout, stderr, code } = await this.run(
      ["kubectl", "-p", this.profile, "--", ...args],
      { check: false },
    );
    return { stdout, stderr, code };
  }

  // Best-effort: force-destroy any libvirt domains left over from a prior run
  // that didn't tear down cleanly (SIGKILL/OOM/host reboot before `minikube
  // delete` could run). `minikube start` fails with cryptic disk/registration
  // errors when stale domains squat on the names. Silent no-op if virsh isn't
  // reachable — the regular start path will surface a real failure.
  private async sweepLingeringDomains(): Promise<void> {
    const virsh = Deno.env.get("SIM_VIRSH_BIN") ?? "virsh";
    const uri = ["-c", "qemu:///system"];
    let list: { stdout: string; code: number };
    try {
      const p = new Deno.Command(virsh, {
        args: [...uri, "list", "--all", "--name"],
        stdout: "piped", stderr: "piped",
      });
      const out = await p.output();
      list = { stdout: new TextDecoder().decode(out.stdout), code: out.code };
    } catch {
      return; // virsh not on PATH — skip silently
    }
    if (list.code !== 0) return;
    const lingerers = list.stdout
      .split("\n").map((s) => s.trim())
      .filter((n) => n === this.profile || n.startsWith(`${this.profile}-m`));
    if (lingerers.length === 0) return;
    console.log(`[k8s] sweeping ${lingerers.length} lingering libvirt domain(s): ${lingerers.join(", ")}`);
    for (const name of lingerers) {
      // destroy = force power-off; undefine = remove definition. Don't pass
      // --remove-all-storage — minikube's kvm2 VMs have an unmanaged hdc
      // (boot2docker.iso) that fails the whole undefine when libvirt tries to
      // free it. We sweep the leftover machine dir afterwards instead.
      for (const op of [
        ["destroy", name],
        ["undefine", "--managed-save", "--snapshots-metadata", name],
      ]) {
        try {
          await new Deno.Command(virsh, { args: [...uri, ...op], stdout: "null", stderr: "null" }).output();
        } catch { /* best-effort */ }
      }
      // Drop the machine dir minikube created for this VM (boot2docker.iso +
      // disk image + config). Best-effort — silent if it doesn't exist.
      const home = Deno.env.get("HOME") ?? "";
      if (home) {
        try {
          await Deno.remove(`${home}/.minikube/machines/${name}`, { recursive: true });
        } catch { /* not present */ }
      }
    }
  }

  // Bring up a cluster with one VM node per topology node.
  //
  // minikube `start` sizes all nodes uniformly, so for heterogeneous topologies
  // (e.g. a "far" node sized differently) we start the first node, then
  // `node add` each remaining node with its own cpu/memory. Homogeneous
  // topologies take the fast `--nodes=N` path.
  //
  // NOTE: `minikube start --nodes=N` / `node add` can exit non-zero on a
  // healthy cluster (worker-label race — it labels the node before the
  // apiserver registers it). We DON'T trust the exit code; the readiness gate
  // (kubectl) is the real success signal.
  async provision(topology: Topology): Promise<ProvisionedCluster> {
    const nodes = topology.nodes;
    if (nodes.length === 0) throw new Error("[k8s] topology has no nodes");
    const n = nodes.length;
    const homogeneous = nodes.every(
      (x) => cpusOf(x) === cpusOf(nodes[0]) && memMbOf(x) === memMbOf(nodes[0]),
    );

    console.log(
      `[k8s] provisioning ${n} VM node(s) for topology "${topology.name}" ` +
        `(profile=${this.profile}, cni=${this.cni}, ${homogeneous ? "homogeneous" : "heterogeneous"})`,
    );

    await this.sweepLingeringDomains();

    // Always 1-by-1: `minikube start --nodes=N` silently creates fewer VMs
    // than asked under load (3rd node never spawns, label race masks it,
    // waitNodesReady times out 10min later). `node add` one-by-one is reliable
    // but slower; we accept the +30s/node for determinism.
    const startArgs = [
      "start", "-p", this.profile,
      "--driver=kvm2",
      `--cni=${this.cni}`,
      `--cpus=${cpusOf(nodes[0])}`,
      `--memory=${memMbOf(nodes[0])}`,
    ];
    if (this.kubernetesVersion) startArgs.push(`--kubernetes-version=${this.kubernetesVersion}`);

    const start = await this.run(startArgs, { check: false });
    this.handleStartResult(start, "start");

    // Add the remaining nodes one at a time. `minikube node add` doesn't
    // accept --cpus/--memory in this minikube version — added VMs come up
    // with the minikube defaults (2 vCPU / 2 GiB) regardless of what we want.
    // We virsh-resize each one after the add so the topology's sizes actually
    // take effect.
    for (let i = 1; i < n; i++) {
      console.log(`[k8s]   adding node for topology node "${nodes[i].id}"`);
      const add = await this.run(
        ["node", "add", "-p", this.profile],
        { check: false },
      );
      this.handleStartResult(add, "node add");
      // Resize the just-added VM to the topology's spec.
      await this.resizeNodeVm(i, cpusOf(nodes[i]), parseMemory(nodes[i].memory));
    }

    const ready = await this.waitNodesReady(n);
    console.log(`[k8s] cluster ready: ${ready.length} node(s) [${ready.join(", ")}]`);

    // Taint the control-plane node so worker pods can't be scheduled there.
    // Without this, the heavy-bench failure mode is: workers land on the
    // control-plane VM, CPU-starve apiserver/etcd, kubectl port-forward dies,
    // bench loses connection and crashes. Static pods (apiserver, etcd, etc.)
    // and DaemonSets-with-tolerations (kube-proxy, flannel, our sampler) stay.
    if (ready.length > 1) {
      const cpName = ready[0]; // minikube's first node is the control plane
      console.log(`[k8s] tainting control-plane "${cpName}" — workloads go to worker nodes only`);
      const taint = await this.kubectl([
        "taint", "node", cpName,
        "node-role.kubernetes.io/control-plane=:NoSchedule",
        "--overwrite",
      ]);
      if (taint.code !== 0) {
        console.warn(`[k8s] control-plane taint failed (non-fatal): ${taint.stderr || taint.stdout}`);
      }
    }

    return { profile: this.profile, nodes: ready };
  }

  // Distinguish the cosmetic node-label race (which is harmless — minikube
  // tries to label the new node a beat before the apiserver registers it,
  // and exits non-zero even though the cluster converges) from a real
  // failure. The race always shows BOTH "applying worker node label" and
  // "nodes \"...\" not found" in stderr; anything else is a real error and
  // we throw instead of silently moving on to waitNodesReady (which would
  // then sit for 10 min before timing out).
  private handleStartResult(
    res: { code: number; stderr: string },
    op: string,
  ): void {
    if (res.code === 0) return;
    if (isCosmeticRace(res.stderr)) {
      console.warn(
        `[k8s] minikube ${op} exited ${res.code} (cosmetic node-label race) — ` +
          `kubectl readiness gate will confirm the cluster.`,
      );
      return;
    }
    const detail = extractRelevantErrorLines(res.stderr);
    throw new Error(
      `[k8s] minikube ${op} failed (exit ${res.code}). Not the label-race.\n` +
        (detail ? `Diagnostics:\n${detail}\n` : "(no diagnostics in stderr)\n") +
        `Recommended next step: \`minikube delete -p ${this.profile}\` and retry.`,
    );
  }

  // Poll `kubectl get nodes` until `expected` nodes are present and all Ready.
  async waitNodesReady(expected: number, timeoutMs = 600_000): Promise<string[]> {
    const deadline = Date.now() + timeoutMs;
    let last = "";
    while (Date.now() < deadline) {
      const { stdout, code } = await this.kubectl([
        "get", "nodes",
        "-o", "jsonpath={range .items[*]}{.metadata.name}{\"=\"}{.status.conditions[?(@.type==\"Ready\")].status}{\"\\n\"}{end}",
      ]);
      if (code === 0) {
        const lines = stdout.trim().split("\n").filter(Boolean);
        const ready = lines.filter((l) => l.endsWith("=True")).map((l) => l.split("=")[0]);
        last = lines.join(", ");
        if (lines.length >= expected && ready.length >= expected) return ready;
      }
      await new Promise((r) => setTimeout(r, 3000));
    }
    throw new Error(
      `[k8s] timed out waiting for ${expected} Ready node(s) after ${timeoutMs}ms. Last: ${last}`,
    );
  }

  // Resize a just-added minikube node VM via virsh: shutdown → set max/min
  // memory + vcpus → start. Required because `minikube node add` ignores
  // sizing flags in this minikube version, so additional nodes come up at
  // the default 2 GiB / 2 vCPU regardless of what we want.
  //
  // The new node's name is `<profile>-m{02,03,...}` per minikube convention.
  private async resizeNodeVm(index: number, cpus: number, memBytes: number): Promise<void> {
    const vmName = `${this.profile}-m${String(index + 1).padStart(2, "0")}`;
    const memKb = Math.floor(memBytes / 1024);
    const virsh = Deno.env.get("SIM_VIRSH_BIN") ?? "virsh";
    const uri = ["-c", "qemu:///system"];

    console.log(`[k8s]   resizing ${vmName} → ${cpus} vCPU / ${Math.round(memBytes / 1024 / 1024 / 1024)} GiB`);

    async function v(args: string[], opts: { check?: boolean } = {}): Promise<{ code: number; stdout: string; stderr: string }> {
      const p = new Deno.Command(virsh, { args: [...uri, ...args], stdout: "piped", stderr: "piped" });
      const out = await p.output();
      const r = {
        code: out.code,
        stdout: new TextDecoder().decode(out.stdout),
        stderr: new TextDecoder().decode(out.stderr),
      };
      if (opts.check && r.code !== 0) {
        throw new Error(`virsh ${args.join(" ")} failed (code ${r.code}): ${r.stderr || r.stdout}`);
      }
      return r;
    }

    // 1. Stop the minikube node first — this runs kubelet/crio teardown
    //    hooks AND shuts down the VM at the libvirt level. A bare
    //    `virsh shutdown` skips minikube's bootstrap context, and a bare
    //    `virsh start` afterward brings the VM up without kubelet starting,
    //    so the node never re-registers.
    const nodeName = vmName; // minikube node name == libvirt domain name
    await this.run(["node", "stop", "-p", this.profile, nodeName], { check: false });

    // Wait for libvirt to confirm it's actually shut off (minikube node stop
    // returns before libvirt finishes).
    let shutoff = false;
    for (let attempt = 0; attempt < 30; attempt++) {
      const state = await v(["domstate", vmName]);
      if (state.stdout.trim() === "shut off") { shutoff = true; break; }
      await new Promise((r) => setTimeout(r, 1000));
    }
    if (!shutoff) {
      console.warn(`[k8s]   ${vmName} did not shut down cleanly — forcing destroy`);
      await v(["destroy", vmName]);
    }

    // 2. Resize at the libvirt level. `--config` persists across reboots.
    //    For vcpus, --maximum must be set before raising the current count.
    await v(["setmaxmem", vmName, String(memKb), "--config"], { check: true });
    await v(["setmem", vmName, String(memKb), "--config"], { check: true });
    await v(["setvcpus", vmName, String(cpus), "--config", "--maximum"], { check: true });
    await v(["setvcpus", vmName, String(cpus), "--config"], { check: true });

    // 3. Start back via minikube so kubelet/crio bootstrap runs properly.
    //    `waitNodesReady` will pick it up once kubelet re-registers.
    await this.run(["node", "start", "-p", this.profile, nodeName], { check: false });
  }

  async teardown(): Promise<void> {
    console.log(`[k8s] minikube delete (profile=${this.profile})`);
    await this.run(["delete", "-p", this.profile], { check: false });
    // minikube's kvm2 driver doesn't always destroy libvirt domains on delete
    // (process killed mid-cleanup, driver bug, etc). Sweep them so a follow-up
    // `provision()` starts from a truly clean slate.
    await this.sweepLingeringDomains();
  }
}

// --- diagnostics helpers (module-level) -----------------------------------

// Actual minikube text: `error applying worker node "m02" label: apply node labels: ...`
// — the node name is quoted between "node" and "label", so allow chars in between.
const RACE_LABEL_RE = /error applying worker node\b[\s\S]*?\blabel\b/i;
const RACE_NOT_FOUND_RE = /nodes? "[^"]+" not found/i;

function isCosmeticRace(stderr: string): boolean {
  const clean = stripAnsiAndBoxes(stderr);
  return RACE_LABEL_RE.test(clean) && RACE_NOT_FOUND_RE.test(clean);
}

// minikube wraps suggestions in a Unicode-box; the literal box characters and
// ANSI colour codes pollute "stderr tail" output. Strip them so we can show a
// useful diagnostic.
function stripAnsiAndBoxes(s: string): string {
  return s
    // deno-lint-ignore no-control-regex
    .replace(/\x1b\[[0-9;]*[A-Za-z]/g, "")
    .replace(/[│┌┐└┘├┤┬┴┼─╭╮╯╰━]/g, "");
}

// Pick out the lines that actually describe what went wrong from minikube's
// long, decorated stderr. We grab error markers + adjacent context.
function extractRelevantErrorLines(stderr: string): string {
  const clean = stripAnsiAndBoxes(stderr);
  const lines = clean.split("\n").map((l) => l.trim()).filter(Boolean);
  const keepRe = /(^[X!*]\s)|Exiting due to|Error[: ]|FAIL|panic|missing |not found|status [1-9]|Suggestion:|Documentation:/i;
  const picked: string[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (keepRe.test(lines[i])) picked.push(lines[i]);
    if (picked.length >= 15) break;
  }
  return picked.join("\n");
}
