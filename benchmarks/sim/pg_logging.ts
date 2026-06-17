// k8s glue around the bundled PG for pgBadger-grade reporting.
//
// The Windmill helm chart's `postgresql.enabled` deploys a vanilla `postgres:18`
// Deployment (called `windmill-postgresql-demo-app` by default — confusingly,
// despite the chart's comment claiming cloudnative-pg). We need it to emit
// every statement + connection so pgBadger can render a real report, which
// means setting PostgreSQL `-c` flags. The chart doesn't expose those, so we
// patch the Deployment after `helm install`.
//
// At end of bench we `kubectl logs` the PG pod and run pgBadger on it. The
// `runPgbadger` helper lives in pgbadger.ts so it's reusable.

import { MinikubeProvisioner } from "./k8s_provisioner.ts";
import { PGBADGER_PG_SETTINGS } from "./pgbadger.ts";

const DEFAULT_PG_DEPLOYMENT = "windmill-postgresql-demo-app";
const DEFAULT_NAMESPACE = "default";
const DEFAULT_PG_USER = "postgres";
const DEFAULT_PG_DB = "windmill";
const PG_POD_LABEL = "app=windmill-postgresql-demo-app";

// The chart switches kind/name based on `postgresql.persistence.enabled`:
//   persistence=false  -> Deployment/windmill-postgresql-demo-app
//   persistence=true   -> StatefulSet/windmill-postgresql
// Both use the same pod label, so exec/logs can take the label selector path
// — but `kubectl patch` and `rollout status` need the right kind/name. Probe.
async function findPgResource(
  prov: MinikubeProvisioner,
  namespace: string,
): Promise<{ kind: "statefulset" | "deployment"; name: string }> {
  const ss = await prov.kubectl([
    "-n", namespace, "get", "statefulset", "windmill-postgresql",
    "--ignore-not-found", "-o", "name",
  ]);
  if (ss.code === 0 && ss.stdout.trim()) {
    return { kind: "statefulset", name: "windmill-postgresql" };
  }
  const dep = await prov.kubectl([
    "-n", namespace, "get", "deployment", DEFAULT_PG_DEPLOYMENT,
    "--ignore-not-found", "-o", "name",
  ]);
  if (dep.code === 0 && dep.stdout.trim()) {
    return { kind: "deployment", name: DEFAULT_PG_DEPLOYMENT };
  }
  throw new Error(`PG not found in ns/${namespace}: neither StatefulSet nor Deployment present`);
}

// Apply pgBadger-grade settings to the running PG via `ALTER SYSTEM SET ... +
// pg_reload_conf()`. All PGBADGER_PG_SETTINGS are SIGHUP-able, so no restart
// is needed — which is critical: the chart's PG uses an emptyDir (persistence
// disabled by default), so a container restart wipes PGDATA and destroys the
// migrations Windmill just ran. ALTER SYSTEM writes to postgresql.auto.conf
// inside PGDATA and is picked up via SIGHUP without dropping connections.
export async function enableVerbosePgLogging(
  prov: MinikubeProvisioner,
  opts: { namespace?: string; user?: string; db?: string } = {},
): Promise<void> {
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE;
  const user = opts.user ?? DEFAULT_PG_USER;
  const db = opts.db ?? DEFAULT_PG_DB;

  // ALTER SYSTEM SET cannot run inside a transaction block, so each statement
  // must go in its own psql `-c`. With multiple `-c`, psql commits each
  // separately.
  const cFlags: string[] = [];
  for (const kv of PGBADGER_PG_SETTINGS) {
    const eq = kv.indexOf("=");
    const name = kv.slice(0, eq);
    const value = kv.slice(eq + 1).replace(/'/g, "''");
    cFlags.push("-c", `ALTER SYSTEM SET ${name} = '${value}';`);
  }
  cFlags.push("-c", "SELECT pg_reload_conf();");

  console.log(`[pg-logging] applying pgBadger settings via ALTER SYSTEM + SIGHUP (no restart)`);
  // `kubectl exec` doesn't take --selector — resolve the pod name from the
  // label first so this works for both Deployment and StatefulSet layouts.
  const podRes = await prov.kubectl([
    "-n", namespace, "get", "pods", "-l", PG_POD_LABEL,
    "-o", "jsonpath={.items[0].metadata.name}",
  ]);
  const podName = podRes.stdout.trim();
  if (!podName) {
    throw new Error(`[pg-logging] no pod found for selector ${PG_POD_LABEL}`);
  }
  const res = await prov.kubectl([
    "-n", namespace, "exec", podName,
    "--",
    "psql", "-U", user, "-d", db, "-v", "ON_ERROR_STOP=1", ...cFlags,
  ]);
  if (res.code !== 0) {
    throw new Error(
      `[pg-logging] psql failed (code ${res.code}): ${res.stderr || res.stdout || "(no output)"}`,
    );
  }
  console.log(`[pg-logging] settings reloaded — verbose logging active without restart`);
}

// Bump max_connections on the bundled PG. The chart doesn't expose a way to
// pass postmaster args, so we kubectl-patch the Deployment to inject
// `args: ["postgres", "-c", "max_connections=N"]`.
//
// PG persistence is off in the smoke values, so any PG restart wipes PGDATA.
// When the patch is a real change, the new PG pod comes up with an empty DB
// and windmill-app's stale connections fail — so we also rollout-restart
// windmill-app and wait for it to re-run migrations.
//
// On reuse (Deployment already has these args) we detect the no-op via the
// generation counter and skip the windmill-app restart entirely.
export async function patchPgMaxConnections(
  prov: MinikubeProvisioner,
  maxConnections: number,
  opts: { namespace?: string; appDeployment?: string } = {},
): Promise<void> {
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE;
  const appDeployment = opts.appDeployment ?? "windmill-app";
  const pg = await findPgResource(prov, namespace);
  const ref = `${pg.kind}/${pg.name}`;

  const genRes = await prov.kubectl([
    "-n", namespace, "get", pg.kind, pg.name,
    "-o", "jsonpath={.metadata.generation}",
  ]);
  const beforeGen = parseInt(genRes.stdout.trim()) || 0;

  console.log(`[pg-logging] patching PG (${ref}) to max_connections=${maxConnections}`);
  const patch = JSON.stringify({
    spec: { template: { spec: { containers: [{
      name: "postgres",
      args: ["postgres", "-c", `max_connections=${maxConnections}`],
    }] } } },
  });
  const r = await prov.kubectl([
    "-n", namespace, "patch", pg.kind, pg.name,
    "--type=strategic", "-p", patch,
  ]);
  if (r.code !== 0) {
    throw new Error(
      `[pg-logging] PG patch failed (code ${r.code}): ${r.stderr || r.stdout || "(no output)"}`,
    );
  }

  const genRes2 = await prov.kubectl([
    "-n", namespace, "get", pg.kind, pg.name,
    "-o", "jsonpath={.metadata.generation}",
  ]);
  const afterGen = parseInt(genRes2.stdout.trim()) || 0;
  if (afterGen === beforeGen) {
    console.log(`[pg-logging] PG patch was a no-op (max_connections already set)`);
    return;
  }

  // Real rollout — wait for PG. With persistence enabled, the rollout
  // preserves PGDATA so windmill-app doesn't need to re-migrate; without
  // persistence we'd also need a windmill-app restart, but that path is no
  // longer used (smoke.yaml enables postgresql.persistence).
  const w = await prov.kubectl([
    "-n", namespace, "rollout", "status", ref, "--timeout=180s",
  ]);
  if (w.code !== 0) {
    throw new Error(
      `[pg-logging] PG rollout did not complete: ${w.stderr || w.stdout || "(no output)"}`,
    );
  }
  // Bounce windmill-app to drop stale connections that point at the old PG pod
  // — fast (the new pod reuses the persistent migrations).
  console.log(`[pg-logging] PG restarted — bouncing ${appDeployment} to reconnect`);
  await prov.kubectl([
    "-n", namespace, "rollout", "restart", `deployment/${appDeployment}`,
  ]);
  const aw = await prov.kubectl([
    "-n", namespace, "rollout", "status", `deployment/${appDeployment}`,
    "--timeout=300s",
  ]);
  if (aw.code !== 0) {
    throw new Error(
      `[pg-logging] ${appDeployment} rollout did not complete: ${aw.stderr || aw.stdout || "(no output)"}`,
    );
  }
}

// Capture the PG pod's stdout/stderr (kubectl logs) to a file.
//
// k8s/containerd has its own log-rotation gotcha similar to the journald drop
// we hit earlier: the default `containerLogMaxSize` is 10Mi, so very-busy PG
// logs can get rotated and the `connection received` burst at fleet-boot
// disappears. For a smoke / short run that's fine; long benches need to raise
// that on the kubelet or write PG to a PVC file (TODO #51 follow-on).
export async function capturePgLog(
  prov: MinikubeProvisioner,
  outPath: string,
  opts: { namespace?: string; sinceTime?: string } = {},
): Promise<void> {
  const namespace = opts.namespace ?? DEFAULT_NAMESPACE;
  console.log(`[pg-logging] capturing PG logs (selector ${PG_POD_LABEL}) -> ${outPath}`);
  // Label selector works for both StatefulSet and Deployment PG layouts.
  const args = [
    "-n", namespace,
    "logs", "-l", PG_POD_LABEL,
    "--all-containers=true",
    "--tail=-1",
  ];
  if (opts.sinceTime) args.push(`--since-time=${opts.sinceTime}`);
  const res = await prov.kubectl(args);
  if (res.code !== 0) {
    throw new Error(
      `[pg-logging] kubectl logs failed: ${res.stderr || res.stdout || "(no output)"}`,
    );
  }
  await Deno.writeTextFile(outPath, res.stdout);
}
