// Toxiproxy admin client + per-node proxy planning.
//
// One toxiproxy container is run for the whole sim. For every node that
// declares db_latency_ms > 0, we create a proxy with a unique listener inside
// the toxiproxy container; the node's DATABASE_URL points at
// `toxiproxy:<port>` instead of `postgres:5432`. The provisioner is
// responsible for starting the container; this module only configures it.
//
// Wire format docs: https://github.com/Shopify/toxiproxy#http-api

export type ProxyPlan = {
  proxyName: string;     // toxiproxy proxy id, e.g. "node-far"
  listenPort: number;    // port inside the toxiproxy container
  upstream: string;      // e.g. "pg:5432"
  latencyMs: number;     // 0 means no toxic, plain pass-through
};

// Allocate ports starting at this offset so they don't collide with toxiproxy's
// admin port (8474) or anything else common. 15400 is unused in well-known
// service registries.
export const FIRST_PROXY_PORT = 15400;

export function buildProxyPlans(
  nodes: { id: string; db_latency_ms?: number }[],
  upstream: string,
): ProxyPlan[] {
  const plans: ProxyPlan[] = [];
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    if (!n.db_latency_ms || n.db_latency_ms <= 0) continue;
    plans.push({
      proxyName: `node-${n.id}`,
      listenPort: FIRST_PROXY_PORT + i,
      upstream,
      latencyMs: n.db_latency_ms,
    });
  }
  return plans;
}

export class ToxiproxyClient {
  constructor(public adminUrl: string) {}

  async waitReady(timeoutMs = 30_000): Promise<void> {
    const start = Date.now();
    while (Date.now() - start < timeoutMs) {
      try {
        const r = await fetch(`${this.adminUrl}/version`);
        if (r.ok) {
          await r.body?.cancel();
          return;
        }
        await r.body?.cancel();
      } catch (_) { /* not ready yet */ }
      await new Promise((r) => setTimeout(r, 250));
    }
    throw new Error(`toxiproxy admin API not reachable at ${this.adminUrl}`);
  }

  async reset(): Promise<void> {
    const r = await fetch(`${this.adminUrl}/reset`, { method: "POST" });
    if (!r.ok) throw new Error(`toxiproxy reset failed: ${r.status} ${await r.text()}`);
    await r.body?.cancel();
  }

  async createProxy(plan: ProxyPlan): Promise<void> {
    const r = await fetch(`${this.adminUrl}/proxies`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        name: plan.proxyName,
        listen: `0.0.0.0:${plan.listenPort}`,
        upstream: plan.upstream,
        enabled: true,
      }),
    });
    if (!r.ok) {
      const body = await r.text();
      throw new Error(`createProxy(${plan.proxyName}) failed: ${r.status} ${body}`);
    }
    await r.body?.cancel();
  }

  async addLatencyToxic(proxyName: string, latencyMs: number): Promise<void> {
    const r = await fetch(`${this.adminUrl}/proxies/${proxyName}/toxics`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        // The default downstream direction adds latency to traffic going from
        // upstream (PG) to client (worker), which is what matters for query RTT.
        type: "latency",
        attributes: { latency: latencyMs, jitter: 0 },
      }),
    });
    if (!r.ok) {
      const body = await r.text();
      throw new Error(`addLatencyToxic(${proxyName}) failed: ${r.status} ${body}`);
    }
    await r.body?.cancel();
  }

  // Convenience: clear all proxies and recreate the plan. Idempotent across
  // sim runs that reuse a long-lived toxiproxy container.
  async apply(plans: ProxyPlan[]): Promise<void> {
    await this.reset();
    for (const p of plans) {
      await this.createProxy(p);
      if (p.latencyMs > 0) {
        await this.addLatencyToxic(p.proxyName, p.latencyMs);
      }
    }
  }
}
