import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";

import { buildProxyPlans, FIRST_PROXY_PORT } from "./toxiproxy.ts";

Deno.test("buildProxyPlans: skips nodes with no latency", () => {
  const plans = buildProxyPlans(
    [
      { id: "a" }, // no latency field
      { id: "b", db_latency_ms: 0 },
      { id: "c", db_latency_ms: 50 },
    ],
    "pg:5432",
  );
  assertEquals(plans.length, 1);
  assertEquals(plans[0].proxyName, "node-c");
  assertEquals(plans[0].latencyMs, 50);
  assertEquals(plans[0].upstream, "pg:5432");
});

Deno.test("buildProxyPlans: assigns unique ports starting at FIRST_PROXY_PORT", () => {
  const plans = buildProxyPlans(
    [
      { id: "x", db_latency_ms: 10 },
      { id: "y", db_latency_ms: 20 },
      { id: "z", db_latency_ms: 30 },
    ],
    "pg:5432",
  );
  assertEquals(plans.length, 3);
  const ports = plans.map((p) => p.listenPort);
  const unique = new Set(ports);
  assertEquals(unique.size, 3, "ports must be unique");
  assertEquals(ports[0] >= FIRST_PROXY_PORT, true);
});

Deno.test("buildProxyPlans: port index follows node array position, not skip index", () => {
  // Edge case: node[1] is skipped (no latency), but node[2] gets a port
  // derived from its position. This is important because the provisioner
  // maps proxies back to nodes via the "node-<id>" naming, not via index.
  const plans = buildProxyPlans(
    [
      { id: "a" },
      { id: "b" },
      { id: "c", db_latency_ms: 50 },
    ],
    "pg:5432",
  );
  assertEquals(plans.length, 1);
  // c is at index 2 in the input → port FIRST + 2
  assertEquals(plans[0].listenPort, FIRST_PROXY_PORT + 2);
});

Deno.test("buildProxyPlans: empty input yields empty plans", () => {
  assertEquals(buildProxyPlans([], "pg:5432").length, 0);
});

Deno.test("buildProxyPlans: all-zero latency yields empty plans (no toxiproxy needed)", () => {
  assertEquals(
    buildProxyPlans(
      [
        { id: "a", db_latency_ms: 0 },
        { id: "b", db_latency_ms: 0 },
      ],
      "pg:5432",
    ).length,
    0,
  );
});
