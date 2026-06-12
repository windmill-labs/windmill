// Stitches per-metric chart SVGs (rendered by graph.ts:drawGraphMulti) into a
// single dashboard SVG with a Windmill-branded header. No new chart rendering
// — we just position the existing SVG outputs inside one outer document.
//
// Brand applied to the wrapper only (background, fonts, header colors). The
// embedded charts keep their existing line-color cycle for now.

import { drawGraphMulti } from "../graph.ts";

// Multi-series data point — matches the shape graph.ts:drawGraphMulti consumes.
// Each entry is one (series, sample) point — `kind` groups points into lines.
export type DataPointMulti = {
  value: number;
  date: Date;
  kind: string;
};

// Brand tokens, light mode. Extracted from frontend/brand-guidelines.md so we
// don't have to import the frontend Tailwind config.
const BRAND = {
  surface_primary: "#fbfbfd",
  surface_tertiary: "#ffffff",
  border_light: "#e5e7eb",
  text_emphasis: "#1d2430",
  text_primary: "#3d4758",
  text_secondary: "#718096",
  text_hint: "#8d93a1",
  font: "Inter, ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif",
  font_mono: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
};

export type DashboardMeta = {
  topology: string;
  suite: string;
  generated: string;       // ISO timestamp
  walltime_s: number;
  jobs_completed: number;
  throughput_per_s: number;
  // Free-form summary rows shown in the header. Use sentence case.
  summary?: Array<{ label: string; value: string }>;
};

export type DashboardPanel = {
  title: string;
  yLabel: string;
  // Time-series data — rendered via drawGraphMulti (one line per `kind`).
  // Pass `[]` if you're providing a pre-rendered svg instead.
  data: DataPointMulti[];
  // Optional pre-rendered SVG (e.g. from drawBars for distributions). When
  // present, used as-is instead of calling drawGraphMulti.
  svg?: string;
  // Optional dashed vertical lines drawn on the chart. Used by phased benches
  // to mark phase boundaries on every time-series panel. Either Date[] (lines
  // + labels) or { dates, hideLabels: true } to draw lines without P1>P2
  // text — only the primary chart (Throughput) labels them; other charts
  // get bare lines so the dashboard isn't visually noisy.
  verticalLines?: Date[] | { dates: Date[]; hideLabels?: boolean };
  // Optional dashed horizontal reference lines (e.g. CPU 100% ceiling).
  horizontalLines?: { y: number; label: string }[];
  // Optional translucent shaded rectangles spanning [from, to] on the x-axis
  // (e.g. "push window"). Drawn UNDER the data lines.
  shadedZones?: { from: Date; to: Date; fill?: string; label?: string }[];
  // Optional phase grouping. Panels with the same `phaseGroup.index` are
  // pulled out of the general 2-col grid and rendered together inside a
  // boxed section labeled with `phaseGroup.label`. `cols` overrides the
  // default 3-per-row layout for this section (e.g. Node memory uses 2).
  phaseGroup?: { index: number; label: string; cols?: number };
  // How many grid columns this panel spans. Default 1; the Node memory
  // time-series uses 2 so it gets the full first row of its section.
  colSpan?: number;
  // Optional list of area fills drawn behind the lines (back→front order).
  // Each entry tints the area under a series whose `kind` matches exactly.
  // Used by the per-node util panels to layer oversaturation behind CPU util.
  areaFills?: { kind: string; color: string; opacity?: number }[];
  // Optional y-axis cap. When set, fixes the chart y-axis to [0, yMax]
  // instead of letting drawGraphMulti auto-scale.
  yMax?: number;
  // Optional per-kind line color override (matches drawGraphMulti's param).
  lineColorOverrides?: Record<string, string>;
};

// drawGraphMulti returns a full <svg>...</svg> document. To embed it inside
// our outer SVG we strip the outer tag and keep the inner content. The inner
// content is already wrapped in a transformed <g>, so we just need the
// dimensions to lay panels out.
function extractSvgInner(svg: string): { inner: string; width: number; height: number } {
  const m = svg.match(/<svg[^>]*\swidth="(\d+)"[^>]*\sheight="(\d+)"[^>]*>([\s\S]*)<\/svg>\s*$/);
  if (!m) {
    return { inner: svg, width: 530, height: 250 };
  }
  return { inner: m[3], width: parseInt(m[1], 10), height: parseInt(m[2], 10) };
}

// Shared relative-time origin for the whole dashboard (epoch ms). Read from
// meta.json's bench_start_ms — used as the "0s" reference on every panel's
// x-axis so the same x position means the same moment across charts.
export type DashboardOptions = { xRelativeOriginMs?: number };

export function renderDashboard(
  meta: DashboardMeta,
  panels: DashboardPanel[],
  opts: DashboardOptions = {},
): string {
  const panelsWithData = panels.filter((p) => p.data.length > 0 || p.svg);

  const PADDING = 40;       // outer margin
  const COLS = 2;
  const COL_GAP = 48;
  const ROW_GAP = 40;
  const PHASE_BOX_PAD = 40;   // inner padding inside a phase box — bumped from 20
                              // so rotated x-axis labels on bar charts (drawBars
                              // uses text-anchor:end transform:rotate(-45) which
                              // can extend past the panel SVG's declared height/width)
                              // don't touch the box border.
  const PHASE_LABEL_H = 56;   // height reserved for phase header text — two
                              // lines (heading + subtitle) when the label
                              // contains " — "; single line otherwise.
  const PHASE_BOX_GAP = 32;   // vertical gap between phase boxes
  // Header height grows with the summary list. Base block (title + suite +
  // generated + headline metrics) takes ~80px; each summary row adds 18px.
  const summaryCount = (meta.summary ?? []).length;
  const HEADER_H = Math.max(130, 80 + summaryCount * 18 + 24);

  // Split panels: general 2-col grid above, phase-grouped boxes below. Both
  // share the same panelW/panelH so all charts align visually.
  const generalPanels = panelsWithData.filter((p) => !p.phaseGroup);
  const phasedPanels  = panelsWithData.filter((p) => !!p.phaseGroup);

  // Pre-render all panels (general + phased) up front — we need their widths
  // to compute the dashboard total width before we can lay them out.
  const allRendered = panelsWithData.map((p) => {
    const svg = p.svg ?? drawGraphMulti(p.data, p.title, p.yLabel, p.yMax, p.verticalLines, p.horizontalLines, p.shadedZones, undefined, p.areaFills, p.lineColorOverrides, opts.xRelativeOriginMs);
    return { ...extractSvgInner(svg), title: p.title, phaseGroup: p.phaseGroup };
  });
  const renderedGeneral = allRendered.filter((r) => !r.phaseGroup);
  const renderedPhased  = allRendered.filter((r) =>  r.phaseGroup);

  // Two separate "slot widths" — the general grid (with throughput/CPU/etc.
  // time-series at 1060px) inflates panelW for the top section, but per-phase
  // distribution panels (bars/donut, 640-690px) shouldn't be forced to reserve
  // 1060 each just because some OTHER panel is that wide. Computing each
  // separately keeps the dashboard from being absurdly wider than its content.
  const generalPanelW = Math.max(530, ...renderedGeneral.map((r) => r.width));
  const panelH = Math.max(250, ...allRendered.map((r) => r.height));

  // Group phased panels by index → ordered phase sections (each section will
  // render as a single boxed row containing N side-by-side panels). The slot
  // width for a section is the MAX width of its own panels — bar charts get
  // bar-chart slot widths, not the global time-series slot width.
  const phaseByIndex = new Map<number, { label: string; panels: typeof renderedPhased; slotW: number }>();
  for (const r of renderedPhased) {
    const pg = r.phaseGroup!;
    let entry = phaseByIndex.get(pg.index);
    if (!entry) {
      entry = { label: pg.label, panels: [], slotW: 0 };
      phaseByIndex.set(pg.index, entry);
    }
    entry.panels.push(r);
    if (r.width > entry.slotW) entry.slotW = r.width;
  }
  const phaseSections = [...phaseByIndex.entries()]
    .sort(([a], [b]) => a - b)
    .map(([_, v]) => v);

  const generalRows = Math.ceil(renderedGeneral.length / COLS);

  // Per-section column count (default 3, overridable via phaseGroup.cols)
  // + per-panel colSpan support so a section can be e.g. 1 wide hero +
  // 2-per-row beneath (Node memory section uses this).
  const DEFAULT_COLS = 3;
  const sectionColsOf = (s: typeof phaseSections[number]): number => {
    for (const r of s.panels) {
      const pg = r.phaseGroup;
      if (pg?.cols !== undefined) return pg.cols;
    }
    return Math.min(s.panels.length, DEFAULT_COLS);
  };
  const colSpansOf = (s: typeof phaseSections[number]): number[] =>
    s.panels.map((p) => Math.max(1, p.colSpan ?? 1));
  // Compute grid row count taking colSpan into account.
  const sectionRowCountOf = (s: typeof phaseSections[number]): number => {
    const cols = sectionColsOf(s);
    let row = 0;
    let usedInRow = 0;
    for (const span of colSpansOf(s)) {
      if (usedInRow + span > cols) {
        row++;
        usedInRow = span;
      } else {
        usedInRow += span;
      }
    }
    return row + 1;
  };
  const sectionRowWs = phaseSections.map((s) => {
    const cols = sectionColsOf(s);
    return cols * s.slotW + (cols - 1) * COL_GAP + PHASE_BOX_PAD * 2;
  });
  const generalGridW = COLS * generalPanelW + (COLS - 1) * COL_GAP;
  const contentW = Math.max(generalGridW, ...sectionRowWs, 0);
  const totalW = PADDING * 2 + contentW;
  const sectionHeights = phaseSections.map((s) => {
    const rows = sectionRowCountOf(s);
    return PHASE_LABEL_H + rows * panelH + (rows - 1) * ROW_GAP + PHASE_BOX_PAD * 2;
  });
  const phaseSectionH = phaseSections.length > 0
    ? sectionHeights.reduce((a, h) => a + h, 0)
      + (phaseSections.length - 1) * PHASE_BOX_GAP
      + (renderedGeneral.length > 0 ? ROW_GAP : 0)
    : 0;
  const totalH = PADDING * 2 + HEADER_H + 16
    + (generalRows > 0 ? generalRows * panelH + (generalRows - 1) * ROW_GAP : 0)
    + phaseSectionH;

  // --- Header ---
  const summaryRows = meta.summary ?? [];
  const summaryX0 = PADDING;
  const summaryY0 = PADDING + 64; // below the title block
  const summaryLineH = 18;

  // Title + suite path + generated timestamp on the left,
  // headline metrics on the right.
  const headerSvg = `
    <rect x="0" y="0" width="${totalW}" height="${totalH}" fill="${BRAND.surface_primary}"/>
    <rect x="${PADDING}" y="${PADDING}" width="${totalW - PADDING * 2}" height="${HEADER_H}"
          fill="${BRAND.surface_tertiary}" stroke="${BRAND.border_light}" stroke-width="1"/>
    <text x="${PADDING + 20}" y="${PADDING + 32}"
          font-family="${BRAND.font}" font-size="18" font-weight="600"
          fill="${BRAND.text_emphasis}">Sim run: ${escapeText(meta.topology)}</text>
    <text x="${PADDING + 20}" y="${PADDING + 52}"
          font-family="${BRAND.font_mono}" font-size="11"
          fill="${BRAND.text_secondary}">${escapeText(meta.suite)}</text>
    <text x="${PADDING + 20}" y="${PADDING + 68}"
          font-family="${BRAND.font}" font-size="11"
          fill="${BRAND.text_hint}">${escapeText(meta.generated)}</text>

    ${headlineMetric(totalW - PADDING - 320, PADDING + 32, "Wall time", `${meta.walltime_s.toFixed(2)}s`)}
    ${headlineMetric(totalW - PADDING - 220, PADDING + 32, "Jobs done", String(meta.jobs_completed))}
    ${headlineMetric(totalW - PADDING - 100, PADDING + 32, "Throughput", `${meta.throughput_per_s.toFixed(1)}/s`)}

    ${summaryRows.map((row, i) => `
      <text x="${summaryX0 + 20}" y="${summaryY0 + 18 + i * summaryLineH}"
            font-family="${BRAND.font}" font-size="12" font-weight="600"
            fill="${BRAND.text_emphasis}">${escapeText(row.label)}:</text>
      <text x="${summaryX0 + 140}" y="${summaryY0 + 18 + i * summaryLineH}"
            font-family="${BRAND.font_mono}" font-size="11"
            fill="${BRAND.text_primary}">${escapeText(row.value)}</text>
    `).join("\n")}
  `;

  // --- General panels (2-col grid, using time-series slot width) ---
  const generalY0 = PADDING + HEADER_H + 16;
  const generalSvg = renderedGeneral.map((p, i) => {
    const col = i % COLS;
    const row = Math.floor(i / COLS);
    const x = PADDING + col * (generalPanelW + COL_GAP);
    const y = generalY0 + row * (panelH + ROW_GAP);
    return `<g transform="translate(${x}, ${y})">${p.inner}</g>`;
  }).join("\n");

  // --- Phase sections (one boxed row per phase, no wrap) ---
  // Cycle through subtle pastel backgrounds so adjacent phases are
  // visually distinguishable; user explicitly asked for "different
  // background / outlines" so the eye can compare phases at a glance.
  const PHASE_PALETTE = [
    { fill: "#f0f6ff", stroke: "#bcd5ff" }, // soft blue
    { fill: "#fff7ed", stroke: "#fdba74" }, // soft orange
    { fill: "#f0fdf4", stroke: "#86efac" }, // soft green
    { fill: "#fdf4ff", stroke: "#e9d5ff" }, // soft violet
    { fill: "#fff1f2", stroke: "#fda4af" }, // soft rose
  ];
  const phasesY0 = generalY0
    + (generalRows > 0 ? generalRows * panelH + (generalRows - 1) * ROW_GAP + ROW_GAP : 0);
  const phasesSvg = phaseSections.map((section, si) => {
    const palette = PHASE_PALETTE[si % PHASE_PALETTE.length];
    const boxY = phasesY0
      + sectionHeights.slice(0, si).reduce((a, h) => a + h + PHASE_BOX_GAP, 0);
    const phaseBoxH = sectionHeights[si];
    const slotW = section.slotW;
    const cols = sectionColsOf(section);
    const rowW = cols * slotW + (cols - 1) * COL_GAP;
    const boxX = PADDING;
    const boxW = contentW;
    const innerX0 = boxX + Math.max(PHASE_BOX_PAD, (boxW - rowW) / 2);
    // Lay out with colSpan awareness: panel with colSpan=2 takes two grid
    // slots (slotW + COL_GAP + slotW) and a single-span panel takes one.
    // Wrap to next row when current row's used cols would overflow.
    const spans = colSpansOf(section);
    let row = 0;
    let usedInRow = 0;
    const panelsRow = section.panels.map((p, i) => {
      const span = spans[i];
      if (usedInRow + span > cols) {
        row++;
        usedInRow = 0;
      }
      const col = usedInRow;
      const x = innerX0 + col * (slotW + COL_GAP);
      const y = boxY + PHASE_LABEL_H + PHASE_BOX_PAD + row * (panelH + ROW_GAP);
      usedInRow += span;
      return `<g transform="translate(${x}, ${y})">${p.inner}</g>`;
    }).join("\n");
    return `
      <rect x="${boxX}" y="${boxY}" width="${boxW}" height="${phaseBoxH}"
            rx="10" ry="10"
            fill="${palette.fill}" stroke="${palette.stroke}" stroke-width="1.5"/>
      ${renderSectionLabel(section.label, boxX + PHASE_BOX_PAD, boxY + 22)}
      ${panelsRow}
    `;
  }).join("\n");

  const panelsSvg = `${generalSvg}\n${phasesSvg}`;

  return `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${totalW}" height="${totalH}" viewBox="0 0 ${totalW} ${totalH}">
${headerSvg}
${panelsSvg}
</svg>`;
}

// Section header that breaks a "Foo — bar baz qux" label into two lines
// (heading + a slightly smaller subtitle). Single-line labels render
// unchanged. Wrapping keeps long section labels from overflowing the box
// width without needing dynamic measurement.
function renderSectionLabel(label: string, x: number, y: number): string {
  const sep = " — ";
  const idx = label.indexOf(sep);
  if (idx < 0) {
    return `<text x="${x}" y="${y}" font-family="${BRAND.font}" font-size="14"
      font-weight="600" fill="${BRAND.text_emphasis}">${escapeText(label)}</text>`;
  }
  const heading = label.slice(0, idx);
  const subtitle = label.slice(idx + sep.length);
  return `
    <text x="${x}" y="${y}" font-family="${BRAND.font}" font-size="14"
          font-weight="600" fill="${BRAND.text_emphasis}">${escapeText(heading)}</text>
    <text x="${x}" y="${y + 18}" font-family="${BRAND.font}" font-size="12"
          font-weight="500" fill="${BRAND.text_primary}">${escapeText(subtitle)}</text>
  `;
}

function headlineMetric(x: number, y: number, label: string, value: string): string {
  return `
    <text x="${x}" y="${y}" font-family="${BRAND.font}" font-size="11"
          fill="${BRAND.text_secondary}">${escapeText(label)}</text>
    <text x="${x}" y="${y + 22}" font-family="${BRAND.font_mono}" font-size="18" font-weight="600"
          fill="${BRAND.text_emphasis}">${escapeText(value)}</text>
  `;
}

function escapeText(s: string): string {
  return String(s)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}
