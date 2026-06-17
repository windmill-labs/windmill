// Helm deploy of Windmill onto a provisioned minikube cluster, + API URL
// resolution via port-forward. Part of the k8s sim path (task #49).
//
// Chart source (`chart`):
//   - omitted            -> the public remote chart "windmill/windmill"
//                           (repo auto-added)
//   - a local path       -> used directly (e.g. ../windmill-helm-charts/charts/windmill)
//   - "windmill/<name>"  -> remote; the windmill repo is auto-added
//   - any other "repo/x" -> used as-is (assumes the repo is already configured)
//
// PG: the chart bundles Postgres (postgresql.enabled, via cloudnative-pg). API:
// the chart exposes a ClusterIP Service `windmill-app` on port 8000; we
// port-forward it to localhost so the existing bench can hit it over http.

import { parse as yamlParse } from "https://deno.land/std@0.224.0/yaml/mod.ts";

const HELM = Deno.env.get("SIM_HELM_BIN") ?? "helm";
const MINIKUBE = Deno.env.get("SIM_MINIKUBE_BIN") ?? "minikube";
const WINDMILL_REPO = "https://windmill-labs.github.io/windmill-helm-charts/";
const APP_SERVICE = "windmill-app";
const APP_PORT = 8000;

export type HelmDeployOptions = {
  profile: string; // minikube profile == kube context
  chart?: string; // local path or "repo/name"; default = vendored chart at
                  // benchmarks/sim/charts/windmill (forked from upstream
                  // 4.0.169 with `priorityClassName`, PG `maxConnections`,
                  // and `priorityClasses.classes[]` exposed via values).
  release?: string; // default "wm"
  namespace?: string; // default "default"
  set?: string[]; // extra --set key=value
  valuesFiles?: string[]; // extra -f values.yaml
  timeoutSec?: number; // helm --wait timeout; default 600
};

// minikube needs the kvm2 driver dir on PATH + LD_LIBRARY_PATH for libvirt
// (see k8s_provisioner.ts). `minikube kubectl` inherits it; helm doesn't need it.
function minikubeEnv(): Record<string, string> {
  const base = Deno.env.toObject();
  const driverDir = Deno.env.get("SIM_KVM2_DRIVER_DIR");
  const libDir = Deno.env.get("SIM_LIBVIRT_LIB_DIR");
  if (driverDir) base.PATH = `${driverDir}:${base.PATH ?? ""}`;
  if (libDir) base.LD_LIBRARY_PATH = base.LD_LIBRARY_PATH ? `${libDir}:${base.LD_LIBRARY_PATH}` : libDir;
  return base;
}

async function run(
  bin: string,
  args: string[],
  { check = true, env }: { check?: boolean; env?: Record<string, string> } = {},
): Promise<{ stdout: string; stderr: string; code: number }> {
  const p = new Deno.Command(bin, { args, env, stdout: "piped", stderr: "piped" });
  const { code, stdout, stderr } = await p.output();
  const out = new TextDecoder().decode(stdout);
  const err = new TextDecoder().decode(stderr);
  if (check && code !== 0) {
    throw new Error(`${bin} ${args.join(" ")} failed (code ${code}):\n${err || out}`);
  }
  return { stdout: out, stderr: err, code };
}

async function pathExists(p: string): Promise<boolean> {
  try {
    await Deno.stat(p);
    return true;
  } catch {
    return false;
  }
}

async function pickFreePort(): Promise<number> {
  const l = Deno.listen({ port: 0 });
  const port = (l.addr as Deno.NetAddr).port;
  l.close();
  return port;
}

// Parse a local chart's Chart.yaml, register each dep's helm repo. Idempotent
// (`helm repo add --force-update` overwrites). Skips oci:// URLs (helm fetches
// those directly without `repo add`).
async function registerLocalChartDepRepos(chartDir: string): Promise<void> {
  let raw: string;
  try {
    raw = await Deno.readTextFile(`${chartDir}/Chart.yaml`);
  } catch (e) {
    console.warn(`[helm] could not read ${chartDir}/Chart.yaml: ${(e as Error).message}`);
    return;
  }
  let parsed: unknown;
  try { parsed = yamlParse(raw); } catch (e) {
    console.warn(`[helm] Chart.yaml parse failed: ${(e as Error).message}`);
    return;
  }
  const deps = (parsed as { dependencies?: Array<{ name?: string; repository?: string }> })
    .dependencies ?? [];
  const seen = new Set<string>();
  for (const d of deps) {
    const url = d.repository;
    if (!url) continue;
    if (!/^https?:\/\//i.test(url)) continue;
    if (seen.has(url)) continue;
    seen.add(url);
    // Use the dep's chart name as the repo alias. Helm allows any alias.
    const alias = d.name ?? new URL(url).hostname.replace(/[^a-z0-9]/g, "");
    console.log(`[helm]   repo add ${alias} ${url}`);
    await run(HELM, ["repo", "add", alias, url, "--force-update"], { check: false });
  }
  await run(HELM, ["repo", "update"], { check: false });
}

// helm install Windmill (bundled PG on). Waits for the release to be ready.
export async function helmDeployWindmill(opts: HelmDeployOptions): Promise<void> {
  // Default to the in-repo vendored chart so sim runs are reproducible without
  // depending on upstream chart version drift (we hit 4.0.165 → 4.0.166 silent
  // breakage during this project; the vendored copy is our pinned baseline).
  const chart = opts.chart ?? new URL("./charts/windmill", import.meta.url).pathname;
  const release = opts.release ?? "wm";
  const namespace = opts.namespace ?? "default";

  const isLocalPath = chart.startsWith(".") || chart.startsWith("/") || (await pathExists(chart));
  if (!isLocalPath && chart.startsWith("windmill/")) {
    // Idempotent: `repo add` errors if it exists with a different URL; --force-update avoids that.
    await run(HELM, ["repo", "add", "windmill", WINDMILL_REPO, "--force-update"], { check: false });
    await run(HELM, ["repo", "update", "windmill"], { check: false });
  }
  if (isLocalPath) {
    // The chart has subchart dependencies declared in Chart.yaml. For a remote
    // chart helm fetches them at pull/install; for a LOCAL chart path we must
    // (a) register each dependency's repository with helm (it errors otherwise
    // with "no repository definition for ..."), and (b) `helm dependency build`
    // to vendor the subcharts into the local chart's ./charts/ dir.
    console.log(`[helm] resolving chart dependencies for local chart at ${chart}`);
    await registerLocalChartDepRepos(chart);
    await run(HELM, ["dependency", "build", chart]);
  }

  // `upgrade --install`: install when absent, upgrade (reconcile) when
  // present. Required for the cluster-reuse path — bare `install` errors with
  // "cannot re-use a name that is still in use" on a second run.
  //
  // No `--wait`. With many workers the chart's default --wait blocks until
  // EVERY deployment is Ready, which deadlocks if workers crashloop on the
  // default PG max_connections (we bump that AFTER helm in main.ts). The
  // caller waits for PG + windmill-app explicitly via kubectl wait, which is
  // both faster and avoids the worker-readiness lock-step.
  const args = [
    "--kube-context", opts.profile,
    "upgrade", "--install", release, chart,
    "--namespace", namespace, "--create-namespace",
    "--set", "postgresql.enabled=true",
    "--timeout", `${opts.timeoutSec ?? 600}s`,
  ];
  for (const s of opts.set ?? []) args.push("--set", s);
  for (const f of opts.valuesFiles ?? []) args.push("-f", f);

  console.log(`[helm] upgrade --install Windmill (release=${release}, chart=${chart}, ns=${namespace})`);
  await run(HELM, args);
  console.log(`[helm] manifests applied — caller waits for PG + app readiness`);
}

// Wait for the bundled PG (StatefulSet or Deployment) to report Ready. Used
// after `helmDeployWindmill` so the PG-bound steps that follow (max_conn
// patch, verbose logging) can run.
export async function waitPgReady(
  profile: string,
  namespace = "default",
  timeoutSec = 300,
): Promise<void> {
  // Probe both — only one of these will exist depending on persistence flag.
  for (const ref of ["statefulset/windmill-postgresql", "deployment/windmill-postgresql-demo-app"]) {
    const r = await run(
      MINIKUBE,
      [
        "kubectl", "-p", profile, "--",
        "-n", namespace, "rollout", "status", ref,
        `--timeout=${timeoutSec}s`,
      ],
      { check: false },
    );
    if (r.code === 0) {
      console.log(`[helm] PG ready (${ref})`);
      return;
    }
  }
  throw new Error("[helm] PG not found or not ready (neither statefulset nor deployment)");
}

// Wait for windmill-app Deployment to report at least 1 Ready replica.
export async function waitWindmillAppReady(
  profile: string,
  namespace = "default",
  timeoutSec = 300,
): Promise<void> {
  const r = await run(
    MINIKUBE,
    [
      "kubectl", "-p", profile, "--",
      "-n", namespace, "rollout", "status", "deployment/windmill-app",
      `--timeout=${timeoutSec}s`,
    ],
    { check: false },
  );
  if (r.code !== 0) {
    throw new Error(`[helm] windmill-app not ready after ${timeoutSec}s`);
  }
  console.log(`[helm] windmill-app ready`);
}

export type ApiEndpoint = { host: string; stop: () => void };

// Port-forward svc/windmill-app:8000 to a free localhost port and wait until
// the API answers. Returns the URL + a stop() to kill the forward.
export async function portForwardApi(profile: string, namespace = "default"): Promise<ApiEndpoint> {
  const localPort = await pickFreePort();
  const child = new Deno.Command(MINIKUBE, {
    args: [
      "kubectl", "-p", profile, "--",
      "port-forward", "-n", namespace, `svc/${APP_SERVICE}`, `${localPort}:${APP_PORT}`,
    ],
    env: minikubeEnv(),
    stdout: "piped",
    stderr: "piped",
  }).spawn();

  const host = `http://127.0.0.1:${localPort}`;
  const deadline = Date.now() + 60_000;
  let lastErr = "";
  while (Date.now() < deadline) {
    try {
      const r = await fetch(`${host}/api/version`, { signal: AbortSignal.timeout(2000) });
      if (r.ok) {
        await r.body?.cancel();
        console.log(`[helm] API reachable at ${host} (port-forward svc/${APP_SERVICE})`);
        return { host, stop: () => { try { child.kill("SIGTERM"); } catch { /* already gone */ } } };
      }
    } catch (e) {
      lastErr = (e as Error).message;
    }
    await new Promise((r) => setTimeout(r, 1000));
  }
  try { child.kill("SIGTERM"); } catch { /* ignore */ }
  throw new Error(`[helm] port-forward to svc/${APP_SERVICE} never became reachable (last: ${lastErr})`);
}
