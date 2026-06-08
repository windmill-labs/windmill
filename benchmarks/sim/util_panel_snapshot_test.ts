// Snapshot-ish test for the util panel render path. We don't snapshot the
// full SVG (too brittle — d3 formatting varies between releases) but we
// assert the load-bearing markers are present:
//   - blue CPU-util area fill (#377eb8)
//   - orange oversaturation area fill (#ff8c00)
//   - 100% horizontal reference line
//   - relative-time x-axis ticks ("0s", "30s", ...) — NOT wall-clock HH:MM
//   - phase-boundary dashed verticals
//
// This catches the regressions we've already paid for: cap-at-100 leaking
// onto oversaturation, color override not wired, wall-clock leaking back in,
// area-fill ordering inverted.

import {
  assert,
  assertStringIncludes,
} from "https://deno.land/std@0.224.0/assert/mod.ts";
import { drawGraphMulti, type DataPointMulti } from "../graph.ts";

function buildUtilSeries(originMs: number): DataPointMulti[] {
  const out: DataPointMulti[] = [];
  // 120 samples at 1Hz — enough for the d3 axis to emit several ticks.
  for (let i = 0; i < 120; i++) {
    const date = new Date(originMs + i * 1000);
    // CPU util ramps 0 → 100%, capped.
    out.push({ value: Math.min(100, i), date, kind: "cpu util" });
    // Oversaturation kicks in after t=30s, climbs uncapped to 400%.
    const oversat = i < 30 ? 0 : (i - 30) * 4;
    out.push({ value: oversat, date, kind: "oversaturation" });
  }
  return out;
}

Deno.test("util panel: orange + blue area fills both present, oversat in front of cpu", () => {
  const t0 = Date.parse("2026-06-08T10:00:00Z");
  const svg = drawGraphMulti(
    buildUtilSeries(t0),
    "m04 — CPU + oversat",
    "[%]",
    undefined, // no yMax — oversat must be allowed >100
    [new Date(t0 + 60_000)], // phase boundary at 60s
    [{ y: 100, label: "100% (full VM)" }],
    undefined,
    undefined,
    [
      { kind: "oversaturation", color: "#ff8c00", opacity: 0.55 },
      { kind: "cpu util", color: "#377eb8", opacity: 1.0 },
    ],
    { "oversaturation": "#ff8c00", "cpu util": "#377eb8" },
    t0,
  );

  // Both fill colors must appear.
  assertStringIncludes(svg, "#ff8c00");
  assertStringIncludes(svg, "#377eb8");
  // Backmost-first: orange must appear in the SVG document before blue (DOM
  // order = paint order — later siblings paint on top).
  const orangeIdx = svg.indexOf('fill="#ff8c00"');
  const blueIdx = svg.indexOf('fill="#377eb8"');
  assert(orangeIdx >= 0 && blueIdx > orangeIdx,
    `expected oversaturation fill before cpu-util fill (orange=${orangeIdx}, blue=${blueIdx})`);
});

Deno.test("util panel: 100% horizontal reference line is rendered", () => {
  const t0 = Date.parse("2026-06-08T10:00:00Z");
  const svg = drawGraphMulti(
    buildUtilSeries(t0),
    "m04 — CPU + oversat",
    "[%]",
    undefined,
    undefined,
    [{ y: 100, label: "100% (full VM)" }],
    undefined,
    undefined,
    undefined,
    undefined,
    t0,
  );
  assertStringIncludes(svg, "100% (full VM)");
});

Deno.test("util panel: x-axis uses relative seconds, NOT wall-clock HH:MM", () => {
  const t0 = Date.parse("2026-06-08T10:00:00Z");
  const svg = drawGraphMulti(
    buildUtilSeries(t0),
    "m04 — CPU + oversat",
    "[%]",
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    t0, // shared origin = the bench start, same as series origin → ticks count from 0s
  );
  // d3 picks ~5 ticks across 120s → expect "0s" and a higher second mark.
  assertStringIncludes(svg, ">0s<");
  // Wall-clock format must NOT appear when relative formatter is active.
  assert(!/>1?[0-9]:[0-5][0-9]</.test(svg),
    `wall-clock-style HH:MM tick label leaked into relative-time axis svg`);
});

Deno.test("util panel: phase boundary dashed verticals are emitted", () => {
  const t0 = Date.parse("2026-06-08T10:00:00Z");
  const svg = drawGraphMulti(
    buildUtilSeries(t0),
    "m04 — CPU + oversat",
    "[%]",
    undefined,
    { dates: [new Date(t0 + 60_000)], hideLabels: true },
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    t0,
  );
  assertStringIncludes(svg, 'class="phase-boundary"');
  assertStringIncludes(svg, 'stroke-dasharray="3 3"');
  // hideLabels: true → no `P1>P2` text overlay.
  assert(!svg.includes("phase-boundary-label"),
    "phase-boundary-label leaked despite hideLabels: true");
});

Deno.test("util panel: shared origin overrides per-chart origin", () => {
  const seriesT0 = Date.parse("2026-06-08T10:00:30Z"); // 30s LATER than shared origin
  const sharedT0 = Date.parse("2026-06-08T10:00:00Z");
  const svg = drawGraphMulti(
    buildUtilSeries(seriesT0),
    "m04 — CPU + oversat",
    "[%]",
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    undefined,
    sharedT0,
  );
  // With shared origin, the first tick label should be "30s" (or larger),
  // not "0s" — series starts 30s after the shared origin.
  assert(!svg.includes(">0s<"),
    `expected first tick to NOT be "0s" when series starts after shared origin`);
});
