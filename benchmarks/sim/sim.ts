// Windmill benchmark sim — CLI entry, provisioning ONLY (no benchmarking).
//
// Provisions a minikube cluster from a topology and deploys Windmill onto it
// via helm (bundled PG). Leaves the cluster running until Ctrl-C, then tears
// it down. Benchmarking is a separate concern — point the bench at the URL
// this prints (`benchmarks/main.ts --host <url> ...` / `wm-bench`).
//
// Usage (wired via the `wm_sim` flake helper):
//   wm_sim --topology sim/topologies/k8s_smoke.json
//   wm_sim --topology <t>.json --helm <local-chart-path>
//   wm_sim --topology <t>.json --helm <chart> --helm-values values.yaml
//   wm_sim --topology <t>.json --helm-values v.yaml \
//          --helm-set-override windmill.workerGroups[0].replicas=250
//
// The helm chart's `values.yaml` is the primary source of deployment config
// (replicas, resources, image). `--helm-set-override` is for surgical CLI
// tweaks on top — naming reflects "override, not primary source". The values
// files are archived under results/<stamp>_<topology>/ so each run records
// exactly what was deployed.
//
// Requires minikube + the kvm2 driver on PATH and libvirtd reachable. The
// `wm_sim` wrapper sets SIM_KVM2_DRIVER_DIR / SIM_LIBVIRT_LIB_DIR.

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { loadTopology, validateTopology } from "./topology.ts";
import { MinikubeProvisioner } from "./k8s_provisioner.ts";
import {
  helmDeployWindmill,
  portForwardApi,
  waitPgReady,
  waitWindmillAppReady,
  type ApiEndpoint,
} from "./helm_deploy.ts";
import { buildToxiproxyManifests } from "./toxiproxy_k8s.ts";
import { enableVerbosePgLogging } from "./pg_logging.ts";
import { applyCpuSampler } from "./cpu_sampler_k8s.ts";
import { loadCachedImages, saveImagesToCache } from "./image_cache.ts";

// minikube profile names allow only alphanumerics + dashes; topology names
// often contain underscores, so sanitise.
function profileFor(prefix: string, name: string): string {
  const slug = name.toLowerCase().replace(/[^a-z0-9-]+/g, "-").replace(/^-+|-+$/g, "");
  return `${prefix}-${slug || "topology"}`;
}

async function loadValidTopology(path: string) {
  const topology = await loadTopology(path);
  const issues = await validateTopology(topology);
  for (const w of issues.filter((i) => i.severity === "warn")) {
    console.warn(`[topology] WARN: ${w.message}`);
  }
  const errors = issues.filter((i) => i.severity === "error");
  if (errors.length > 0) {
    for (const e of errors) console.error(`[topology] ERROR: ${e.message}`);
    Deno.exit(1);
  }
  return topology;
}

type DeployArgs = {
  helm?: string;
  helmValues?: string[];
  helmSetOverride?: string[];
};

async function deployWindmill(prov: MinikubeProvisioner, args: DeployArgs): Promise<ApiEndpoint> {
  await helmDeployWindmill({
    profile: prov.profile,
    chart: args.helm,
    valuesFiles: args.helmValues,
    set: args.helmSetOverride,
  });
  // helm runs without --wait — gate PG + app readiness ourselves so port-forward
  // doesn't race startup. PG max_connections is templated into the chart now
  // (postgresql.maxConnections value), no post-install patching needed.
  await waitPgReady(prov.profile);
  await waitWindmillAppReady(prov.profile);
  return await portForwardApi(prov.profile);
}

type UpOpts = DeployArgs & { topology: string };

async function upMain(opts: UpOpts) {
  const topology = await loadValidTopology(opts.topology);

  // Per-run reports dir — archives the inputs (topology + values files) so
  // each cluster bring-up records exactly what was deployed.
  const isoStamp = new Date().toISOString().replace(/[:.]/g, "-").replace(/Z$/, "");
  const outDir = `reports/${isoStamp}_${topology.name}`;
  await Deno.mkdir(outDir, { recursive: true });
  await Deno.copyFile(opts.topology, `${outDir}/topology.json`);
  for (const v of opts.helmValues ?? []) {
    const base = v.split("/").pop() || "values.yaml";
    await Deno.copyFile(v, `${outDir}/${base}`);
  }
  console.log(`[sim] inputs archived to ${outDir}/`);

  const prov = new MinikubeProvisioner({ profile: profileFor("wm-sim", topology.name) });
  // Clean any leftover state (crashed prior run, leaked libvirt domains)
  // before provisioning. teardown() now sweeps both minikube + libvirt.
  console.log(`[sim] cleaning stale state for profile=${prov.profile}...`);
  await prov.teardown();
  await prov.provision(topology);
  try {
    await loadCachedImages(prov.profile);
  } catch (e) {
    console.warn(`[sim] image cache preload failed: ${(e as Error).message}`);
  }

  // Toxiproxy: single Deployment + Service, no nodeSelector. Worker count +
  // resources + DATABASE_URL live in the user's values.yaml.
  const toxManifestPath = `${outDir}/toxiproxy.yaml`;
  await Deno.writeTextFile(toxManifestPath, buildToxiproxyManifests(topology));
  console.log(`[sim] applying toxiproxy...`);
  const applyRes = await prov.kubectl(["apply", "-f", toxManifestPath]);
  if (applyRes.code !== 0) {
    console.warn(`[sim] toxiproxy apply non-zero: ${applyRes.stdout}`);
  }

  // helm: user values are the only source of truth (workerGroups +
  // DATABASE_URL routing live there now).
  const endpoint = await deployWindmill(prov, opts);

  // PG max_connections bump + readiness gates are inside deployWindmill now.

  // Enable verbose PG logging so a pgBadger report can be generated on
  // teardown — best-effort; if the chart's PG layout differs, log and
  // continue (the cluster is still usable for ad-hoc work).
  try {
    await enableVerbosePgLogging(prov);
  } catch (e) {
    console.warn(`[sim] could not enable verbose PG logging: ${(e as Error).message}`);
  }

  // Apply the sampler DaemonSet once per provision — it runs continuously
  // between bench runs; wm-bench scopes its output via `--since-time` so each
  // report only sees rows from its own window. No rollout-restart on reuse.
  try {
    await applyCpuSampler(prov, outDir);
  } catch (e) {
    console.warn(`[sim] CPU sampler not applied: ${(e as Error).message}`);
  }

  await Deno.writeTextFile(
    `${outDir}/meta.json`,
    JSON.stringify(
      {
        topology: topology.name,
        profile: prov.profile,
        api_host: endpoint.host,
        helm_chart: opts.helm ?? "windmill/windmill",
        helm_values_files: opts.helmValues ?? [],
        helm_set_overrides: opts.helmSetOverride ?? [],
        started_at: new Date().toISOString(),
      },
      null,
      2,
    ),
  );

  console.log("\n" + "=".repeat(56));
  console.log(`  Topology "${topology.name}" is up.`);
  console.log(`  Windmill API:     ${endpoint.host}`);
  console.log(`  minikube profile: ${prov.profile}`);
  console.log(`  Login:            admin@windmill.dev / changeme`);
  console.log("");
  console.log(`  To bench against this cluster:`);
  console.log(`    wm-bench --host ${endpoint.host} --minikube-profile ${prov.profile} ...`);
  console.log("");
  console.log(`  Ctrl-C to tear down the cluster.`);
  console.log("=".repeat(56));
  await new Promise<void>((resolve) => {
    const handler = () => {
      Deno.removeSignalListener("SIGINT", handler);
      Deno.removeSignalListener("SIGTERM", handler);
      resolve();
    };
    Deno.addSignalListener("SIGINT", handler);
    Deno.addSignalListener("SIGTERM", handler);
  });

  // Save images BEFORE minikube delete — they're gone with the VM otherwise.
  try {
    await saveImagesToCache(prov.profile, (args) => prov.kubectl(args));
  } catch (e) {
    console.warn(`[sim] image cache save failed: ${(e as Error).message}`);
  }
  console.log("\nTearing down...");
  endpoint.stop();
  await prov.teardown();
  Deno.exit(0);
}

if (import.meta.main) {
  // `up` subcommand mirrors the root action — both forms work:
  //   wm_sim --topology X        (root action)
  //   wm_sim up --topology X     (explicit subcommand)
  const upSub = new Command()
    .description("Provision the cluster + deploy Windmill, leave running (Ctrl-C tears down).")
    .option("--topology <path:string>", "Topology JSON.", { required: true })
    .option(
      "--helm <chart:string>",
      "Helm chart for Windmill: local path (preferred) or remote 'repo/name'. Default 'windmill/windmill'.",
    )
    .option(
      "--helm-values <path:string>",
      "Helm values.yaml — primary source of deployment config (replicas/resources/image). Repeatable; later files take precedence. Copied into results/ as the run record.",
      { collect: true },
    )
    .option(
      "--helm-set-override <kv:string>",
      "Inline key=value override on top of --helm-values (repeatable). For surgical CLI tweaks; the values file is the source of truth.",
      { collect: true },
    )
    .action(upMain);

  await new Command()
    .name("wm_sim")
    .description(
      "Windmill benchmark sim: provision a minikube cluster from a topology and deploy Windmill on it. Provisioning only — no benchmarking.",
    )
    .command("up", upSub)
    .default("up")
    .parse();
}
