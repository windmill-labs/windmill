import { assertEquals, assertStringIncludes } from "https://deno.land/std@0.224.0/assert/mod.ts";

import {
  formatBytes,
  parseMemory,
  validateTopology,
  workerCount,
  type NodeSpec,
  type Topology,
} from "./topology.ts";

const standaloneNode = (id: string, cpu = 2): NodeSpec => ({
  id, mode: "native", standalone: true, cpu, memory: "4G",
});
const workerNode = (id: string, cpu = 2): NodeSpec => ({
  id, mode: "native", cpu, memory: "4G",
});

const okTopo = (): Topology => ({
  name: "ok",
  host_resources: {
    cpus: 8,
    memory: "16G",
    reserved: { cpus: 2, memory: "2G" },
  },
  postgres: { cpu: 2, memory: "4G" },
  nodes: [standaloneNode("server")],
});

Deno.test("parseMemory: integer values across units", () => {
  assertEquals(parseMemory("1024"), 1024);
  assertEquals(parseMemory("1K"), 1024);
  assertEquals(parseMemory("1M"), 1024 ** 2);
  assertEquals(parseMemory("1G"), 1024 ** 3);
  assertEquals(parseMemory("2T"), 2 * 1024 ** 4);
});

Deno.test("parseMemory: decimal multipliers", () => {
  assertEquals(parseMemory("1.5G"), 1.5 * 1024 ** 3);
  assertEquals(parseMemory("0.5M"), 0.5 * 1024 ** 2);
});

Deno.test("parseMemory: case-insensitive units", () => {
  assertEquals(parseMemory("4g"), 4 * 1024 ** 3);
  assertEquals(parseMemory("512m"), 512 * 1024 ** 2);
});

Deno.test("parseMemory: rejects garbage", () => {
  let threw = false;
  try {
    parseMemory("twelve");
  } catch (_) {
    threw = true;
  }
  assertEquals(threw, true);
});

Deno.test("formatBytes: round-trips through parseMemory", () => {
  for (const bytes of [1024, 1024 ** 2, 4 * 1024 ** 3, 1.5 * 1024 ** 3]) {
    const formatted = formatBytes(bytes);
    // formatBytes uses 2dp so the parse may have a tiny rounding delta; just
    // confirm the unit landed in the right ballpark.
    const reparsed = parseMemory(formatted);
    const delta = Math.abs(reparsed - bytes) / bytes;
    assertEquals(delta < 0.02, true, `${bytes} -> ${formatted} -> ${reparsed}`);
  }
});

Deno.test("validateTopology: healthy topology produces no errors", async () => {
  const issues = await validateTopology(okTopo());
  assertEquals(issues.filter((i) => i.severity === "error").length, 0);
});

Deno.test("validateTopology: catches CPU oversubscription", async () => {
  const t = okTopo();
  t.host_resources!.cpus = 4; // 4 - 2 reserved = 2 budget, requesting 4
  const issues = await validateTopology(t);
  const errors = issues.filter((i) => i.severity === "error");
  assertEquals(errors.length >= 1, true);
  assertStringIncludes(errors[0].message, "CPU oversubscription");
});

Deno.test("validateTopology: catches memory oversubscription", async () => {
  const t = okTopo();
  t.host_resources!.memory = "8G"; // 8G - 2G reserved = 6G budget, requesting 8G
  const issues = await validateTopology(t);
  const errors = issues.filter((i) => i.severity === "error");
  assertEquals(
    errors.some((e) => e.message.includes("Memory oversubscription")),
    true,
  );
});

Deno.test("validateTopology: catches duplicate node ids", async () => {
  const t = okTopo();
  t.nodes.push(workerNode("server"));
  // Bump host so CPU/mem don't also error out and obscure the dup detection.
  t.host_resources!.cpus = 16;
  t.host_resources!.memory = "32G";
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("Duplicate node id")),
    true,
  );
});

Deno.test("validateTopology: catches non-positive cpu on a node", async () => {
  const t = okTopo();
  t.nodes[0].cpu = 0;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("non-positive cpu")),
    true,
  );
});

Deno.test("validateTopology: rejects num_workers field (no longer configurable)", async () => {
  const t = okTopo();
  (t.nodes[0] as unknown as Record<string, unknown>).num_workers = 2;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("no longer configurable")),
    true,
  );
});

Deno.test("validateTopology: rejects pods on k8s mode (helm owns scale)", async () => {
  const t = okTopo();
  t.nodes = [{ id: "c", mode: "k8s", cpu: 4, memory: "8G" } as NodeSpec];
  (t.nodes[0] as unknown as Record<string, unknown>).pods = 3;
  t.host_resources!.cpus = 8;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("pods is no longer configurable")),
    true,
  );
});

Deno.test("validateTopology: rejects workers_per_pod on k8s mode", async () => {
  const t = okTopo();
  t.nodes = [{ id: "c", mode: "k8s", cpu: 4, memory: "8G" } as NodeSpec];
  (t.nodes[0] as unknown as Record<string, unknown>).workers_per_pod = 2;
  t.host_resources!.cpus = 8;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("workers_per_pod is no longer configurable")),
    true,
  );
});

Deno.test("validateTopology: no standalone node fails", async () => {
  const t = okTopo();
  t.nodes = [workerNode("a"), workerNode("b")];
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("`standalone: true` node")),
    true,
  );
});

Deno.test("validateTopology: more than one standalone node fails", async () => {
  const t = okTopo();
  t.nodes = [standaloneNode("a"), standaloneNode("b")];
  t.host_resources!.cpus = 16;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("nodes with `standalone: true`")),
    true,
  );
});

Deno.test("validateTopology: standalone + worker mix is valid", async () => {
  const t = okTopo();
  t.nodes = [standaloneNode("server"), workerNode("worker1"), workerNode("worker2")];
  t.host_resources!.cpus = 16;
  t.host_resources!.memory = "32G";
  const issues = await validateTopology(t);
  assertEquals(issues.filter((i) => i.severity === "error").length, 0);
});

Deno.test("validateTopology: legacy mode=agent rejected with helpful message", async () => {
  const t = okTopo();
  t.nodes = [standaloneNode("server"), { id: "a", mode: "agent" as "native", cpu: 1, memory: "2G" }];
  t.host_resources!.cpus = 16;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("\"agent\" is not supported yet")),
    true,
  );
});

Deno.test("validateTopology: mixed k8s and non-k8s fails", async () => {
  const t = okTopo();
  t.nodes = [
    standaloneNode("server"),
    { id: "k8s", mode: "k8s", cpu: 4, memory: "8G" } as NodeSpec,
  ];
  t.host_resources!.cpus = 16;
  t.host_resources!.memory = "32G";
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.message.includes("Mixed k8s and non-k8s")),
    true,
  );
});

Deno.test("workerCount: native/agent always 1", () => {
  assertEquals(workerCount({ id: "x", mode: "native", cpu: 4, memory: "1G" }), 1);
  assertEquals(workerCount({ id: "x", mode: "agent" as "native",  cpu: 4, memory: "1G" }), 1);
});

Deno.test("workerCount: k8s returns null (helm owns scale)", () => {
  assertEquals(workerCount({ id: "x", mode: "k8s", cpu: 4, memory: "1G" }), null);
});

Deno.test("validateTopology: pin_cpus requires enough cores", async () => {
  const t = okTopo();
  t.pin_cpus = true;
  t.host_resources!.cpus = 4; // budget 2 for postgres+nodes, sum is 4
  const issues = await validateTopology(t);
  // First it errors on oversubscription, then again on the pin_cpus constraint.
  assertEquals(
    issues.some((i) => i.message.includes("pin_cpus")),
    true,
  );
});

Deno.test("validateTopology: drops host_resources entirely → falls back to live host", async () => {
  // When no host_resources block is declared, hostCores() / hostMemoryBytes()
  // are used as the budget. We don't assert exact behavior here (host-dependent);
  // we just confirm the validator doesn't crash and doesn't spuriously flag CPU
  // oversubscription for the healthy topo.
  const t = okTopo();
  delete (t as { host_resources?: unknown }).host_resources;
  const issues = await validateTopology(t);
  assertEquals(
    issues.some((i) => i.severity === "error" && i.message.includes("CPU oversubscription")),
    false,
  );
});
