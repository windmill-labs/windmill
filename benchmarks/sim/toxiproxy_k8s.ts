// Toxiproxy for the k8s sim path.
//
// Architecture: ONE toxiproxy Deployment (no nodeSelector, k8s schedules it
// wherever), one Service in front of it, and ONE worker group named "default"
// with N replicas — also no nodeSelector, so the scheduler spreads workers
// across nodes naturally. All workers route through the single toxiproxy
// Service to PG, with the topology's RTT applied as a latency toxic.
//
// Caveat: per-node `db_latency_ms` variance from the topology is collapsed —
// we use the first node's value as the shared latency. Heterogeneous-RTT
// topologies need a per-node-host toxiproxy DaemonSet + downward-API NODE_IP;
// defer until a topology actually needs it.

import type { Topology } from "./topology.ts";
import { stringify as yamlStringify } from "https://deno.land/std@0.224.0/yaml/mod.ts";

const TOXIPROXY_IMAGE = "ghcr.io/shopify/toxiproxy:2.9.0";
const PROXY_LISTEN_PORT = 15400;
const TOXIPROXY_ADMIN_PORT = 8474;
const PG_SERVICE = "windmill-postgresql";
const PG_PORT = 5432;
const TOXIPROXY_NAME = "toxiproxy";

// Build the toxiproxy Deployment+Service+ConfigMap only. Worker count,
// resources, and the DATABASE_URL that routes through toxiproxy all live in
// the user's values.yaml — the chart is the source of truth for windmill
// config.
//
// Service URL the values.yaml should target:
//   postgres://postgres:windmill@toxiproxy.default.svc:15400/windmill?sslmode=disable
export function buildToxiproxyManifests(
  topology: Topology,
  namespace = "default",
): string {
  const latencyMs = topology.nodes[0]?.db_latency_ms ?? 0;
  return [
    buildConfigMap({ name: TOXIPROXY_NAME, namespace, proxyName: "pg", latencyMs }),
    buildDeployment({ name: TOXIPROXY_NAME, namespace }),
    buildService({ name: TOXIPROXY_NAME, namespace }),
  ].join("---\n");
}

// toxiproxy-server `-config` JSON: pre-populates proxies + toxics at startup,
// no shell or admin-API call needed. The shopify/toxiproxy image is distroless
// (no /bin/sh), so this is the way.
function buildConfigMap(args: {
  name: string;
  namespace: string;
  proxyName: string;
  latencyMs: number;
}): string {
  const proxy: Record<string, unknown> = {
    name: args.proxyName,
    listen: `0.0.0.0:${PROXY_LISTEN_PORT}`,
    upstream: `${PG_SERVICE}:${PG_PORT}`,
    enabled: true,
  };
  if (args.latencyMs > 0) {
    proxy.toxics = [{
      name: "latency",
      type: "latency",
      stream: "downstream",
      attributes: { latency: args.latencyMs, jitter: 0 },
    }];
  }
  const config = JSON.stringify([proxy], null, 2);
  return yamlStringify({
    apiVersion: "v1",
    kind: "ConfigMap",
    metadata: { name: args.name, namespace: args.namespace },
    data: { "config.json": config },
  });
}

function buildDeployment(args: {
  name: string;
  namespace: string;
}): string {
  // OOM-immunity for toxiproxy:
  //   1. priorityClassName=wm-critical — kubelet L1 eviction sorts pods by
  //      priority, so toxiproxy gets evicted after worker pods.
  //   2. A busybox sidecar (`oom-immune`) writes -999 to the toxiproxy
  //      process's /proc/<pid>/oom_score_adj for kernel-OOM (L2) protection.
  //      The shopify/toxiproxy image is distroless (no shell), so we can't
  //      use a postStart `exec` hook on it directly — `shareProcessNamespace:
  //      true` lets the sidecar see toxiproxy's PID, and CAP_SYS_RESOURCE on
  //      the sidecar allows lowering oom_score_adj below 0.
  return yamlStringify({
    apiVersion: "apps/v1",
    kind: "Deployment",
    metadata: { name: args.name, namespace: args.namespace, labels: { app: args.name } },
    spec: {
      replicas: 1,
      selector: { matchLabels: { app: args.name } },
      template: {
        metadata: { labels: { app: args.name } },
        spec: {
          priorityClassName: "wm-critical",
          shareProcessNamespace: true,
          containers: [
            {
              name: "toxiproxy",
              image: TOXIPROXY_IMAGE,
              // toxiproxy-server is the image's ENTRYPOINT; we just pass flags.
              args: [
                "-host", "0.0.0.0",
                "-port", String(TOXIPROXY_ADMIN_PORT),
                "-config", "/etc/toxiproxy/config.json",
              ],
              ports: [
                { name: "proxy", containerPort: PROXY_LISTEN_PORT },
                { name: "admin", containerPort: TOXIPROXY_ADMIN_PORT },
              ],
              // No memory limit — even 1280Mi was being tripped on busy
              // nodes (toxiproxy went CrashLoopBackOff multiple times this
              // session). Without a cgroup ceiling, only the node-level
              // kernel OOM can take it down, and the wm-critical priority
              // class + oom_score_adj=-999 sidecar make it last-to-pick.
              resources: {
                requests: { cpu: "1000m", memory: "320Mi" },
              },
              readinessProbe: {
                httpGet: { path: "/proxies", port: TOXIPROXY_ADMIN_PORT },
                initialDelaySeconds: 1,
                periodSeconds: 2,
              },
              volumeMounts: [
                { name: "config", mountPath: "/etc/toxiproxy", readOnly: true },
              ],
            },
            {
              name: "oom-immune",
              image: "busybox:1.36",
              securityContext: { capabilities: { add: ["SYS_RESOURCE"] } },
              // With shareProcessNamespace=true the sidecar sees toxiproxy's
              // PID. Retry up to 60s for the process to come up, then write
              // -999 and sleep forever (sidecar must stay alive — exit would
              // trigger pod restart logic).
              command: ["/bin/sh", "-c"],
              args: [
                // The shopify/toxiproxy image's binary is `/toxiproxy` (not
                // `toxiproxy-server` — that's the older name). pidof matches
                // on basename, so `pidof toxiproxy` is what we need.
                [
                  "set -e",
                  "for i in $(seq 1 60); do",
                  "  pid=$(pidof toxiproxy || true)",
                  "  if [ -n \"$pid\" ]; then",
                  "    for p in $pid; do",
                  "      echo -999 > /proc/$p/oom_score_adj && \\",
                  "        echo \"[oom-immune] pid=$p adj=$(cat /proc/$p/oom_score_adj)\" || \\",
                  "        echo \"[oom-immune] failed on pid=$p\"",
                  "    done",
                  "    break",
                  "  fi",
                  "  sleep 1",
                  "done",
                  "exec sleep infinity",
                ].join("\n"),
              ],
            },
          ],
          volumes: [
            { name: "config", configMap: { name: args.name } },
          ],
        },
      },
    },
  });
}

function buildService(args: { name: string; namespace: string }): string {
  return yamlStringify({
    apiVersion: "v1",
    kind: "Service",
    metadata: { name: args.name, namespace: args.namespace, labels: { app: args.name } },
    spec: {
      type: "ClusterIP",
      selector: { app: args.name },
      ports: [
        { name: "proxy", port: PROXY_LISTEN_PORT, targetPort: PROXY_LISTEN_PORT },
        { name: "admin", port: TOXIPROXY_ADMIN_PORT, targetPort: TOXIPROXY_ADMIN_PORT },
      ],
    },
  });
}
