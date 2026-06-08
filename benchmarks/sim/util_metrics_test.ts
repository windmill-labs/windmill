import { assertEquals } from "https://deno.land/std@0.224.0/assert/mod.ts";
import { computeOversatPct } from "./util_metrics.ts";

Deno.test("computeOversatPct: at capacity returns 0", () => {
  assertEquals(computeOversatPct(4, { procs_running: 4 }), 0);
  assertEquals(computeOversatPct(8, { procs_running: 8 }), 0);
});

Deno.test("computeOversatPct: idle returns 0 (clamped, not negative)", () => {
  assertEquals(computeOversatPct(4, { procs_running: 1 }), 0);
  assertEquals(computeOversatPct(4, { procs_running: 0 }), 0);
});

Deno.test("computeOversatPct: one extra core's worth → 100%", () => {
  assertEquals(computeOversatPct(4, { procs_running: 8 }), 100);
});

Deno.test("computeOversatPct: 10x runnable on 4 CPUs → 900%", () => {
  assertEquals(computeOversatPct(4, { procs_running: 40 }), 900);
});

Deno.test("computeOversatPct: prefers procs_running over load1", () => {
  // load1 says oversat, procs_running says fine — should report fine.
  assertEquals(computeOversatPct(4, { procs_running: 4, load1: 100 }), 0);
});

Deno.test("computeOversatPct: falls back to load1 when procs_running missing", () => {
  assertEquals(computeOversatPct(4, { procs_running: null, load1: 8 }), 100);
  assertEquals(computeOversatPct(4, { load1: 12 }), 200);
});

Deno.test("computeOversatPct: returns 0 when ncpu is invalid", () => {
  assertEquals(computeOversatPct(0, { procs_running: 100 }), 0);
  assertEquals(computeOversatPct(NaN, { procs_running: 100 }), 0);
  assertEquals(computeOversatPct(-1, { procs_running: 100 }), 0);
});

Deno.test("computeOversatPct: returns 0 when neither metric present", () => {
  assertEquals(computeOversatPct(4, {}), 0);
  assertEquals(computeOversatPct(4, { procs_running: null, load1: null }), 0);
});
