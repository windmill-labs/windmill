import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import { JSDOM } from "https://jspm.dev/jsdom@22";

type DataPoint = {
  value: number;
  date: Date;
};
export function drawGraph(
  data: DataPoint[],
  title: string,
  yLabel: string = "[jobs/s]",
  yMax?: number,
) {
  const context = {
    jsdom: new JSDOM(""),
  };
  const { window } = context.jsdom;
  const { document } = window;

  const body = d3.select(document).select("body");

  // Time-series chart — 900px wide so the many-points lines (throughput,
  // CPU util, memory, PG memory etc.) have room to read individual peaks.
  // drawBars/drawDonut below stay at 600 since bar charts don't benefit
  // from extra width.
  const width = 900;
  const height = 200;

  const marginTop = 20;
  const marginRight = 30;
  const marginBottom = 30;
  const marginLeft = 60;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  svg = svg
    .append("g")
    .attr("transform", "translate(" + marginLeft + "," + marginTop + ")");

  const x = d3
    .scaleTime()
    .domain(
      d3.extent(data, function (d: DataPoint) {
        return d.date;
      })
    )
    // No .nice() on the time scale — it rounds the domain to "nice" tick
    // boundaries (e.g. extends domain to next 30s mark), pushing the first
    // few seconds of data off the left edge so phase 1 (idle baseline) and
    // m02's CPU curve appear to "start from the middle". Exact data extent
    // keeps the line starting at the actual first sample.
    .range([0, width]);

  // Relative-time x-axis: labels show seconds since the earliest data point
  // in this chart ("0s, 30s, 60s ..."). Wall-clock HH:MM:SS labels were
  // confusing — they alternated between "03:45" and ":30" formats and made
  // same-time comparisons across panels hard. Relative units anchor every
  // panel at 0s.
  const xDom = x.domain() as [Date, Date];
  const xOriginMs = xRelativeOriginMs ?? xDom[0].getTime();
  const xAxis = d3.axisBottom(x).ticks(5).tickFormat((d) => {
    const sec = Math.round(((d as Date).getTime() - xOriginMs) / 1000);
    return `${sec}s`;
  });

  svg
    .append("g")
    .attr("transform", "translate(0," + height + ")")
    .call(xAxis);

  // Add Y axis. When yMax is set, also `.clamp(true)` so out-of-range data
  // values (e.g. a CPU spike to 250% on a chart capped at 150%) render at the
  // axis boundary instead of escaping the chart area entirely.
  const y = d3
    .scaleLinear()
    .domain([
      0,
      yMax !== undefined ? yMax : d3.max(data, function (d: DataPoint) {
        return +d.value;
      }) * 1.5,
    ])
    .range([height, 0])
    .clamp(yMax !== undefined)
    .nice();
  svg.append("g").call(d3.axisLeft(y));

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 12px")
    .attr("transform", "rotate(-90)")
    .attr("y", -marginLeft + 20)
    .attr("x", -height / 2)
    .text(yLabel);

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 16px")
    .attr("y", 0)
    .attr("x", width / 2)
    .text(title);

  // Add the line
  svg
    .append("path")
    .datum(data)
    .attr("fill", "none")
    .attr("stroke", "steelblue")
    .attr("stroke-width", 1.5)
    .attr(
      "d",
      d3.line(
        function (d: DataPoint) {
          return x(d.date);
        },
        function (d: DataPoint) {
          return y(d.value);
        }
      )
    );

  return body.node().innerHTML;
}

export interface DataPointMulti extends DataPoint {
  kind: string;
}

export function drawGraphMulti(
  data: DataPointMulti[],
  title: string,
  yLabel: string = "[jobs/s]",
  // When set, fixes the chart Y axis at [0, yMax] instead of auto-scaling to
  // 1.5x the max sample. Use for percent-based panels (Node CPU at 150 caps
  // noise spikes that would otherwise compress the rest of the chart).
  yMax?: number,
  // Optional vertical dashed lines at given timestamps. Used by the phased
  // bench to mark phase boundaries (warmup → peak → cooldown, etc.) on every
  // time-series panel so you can attribute throughput/CPU shifts to phases.
  // Accepts a plain Date[] (lines + labels drawn) or `{ dates, hideLabels }`
  // — the renderer suppresses the P1>P2 labels on charts other than the
  // "primary" one so the dashboard doesn't end up with 27 boundary labels
  // (3 per chart × 9 charts) which reads as duplication side-by-side.
  verticalLines?: Date[] | { dates: Date[]; hideLabels?: boolean },
  // Optional horizontal reference lines at given y-values (in chart units).
  // Drawn as labeled dashed grey lines — used to show e.g. the 100% per-VM
  // ceiling on the Node CPU chart so saturation is obvious at a glance.
  horizontalLines?: { y: number; label: string }[],
  // Optional translucent shaded rectangles spanning [from, to] on the x-axis.
  // Used to mark the "push window" on Throughput / Queue depth / Node CPU so
  // it's visually obvious which range is "bench actively pushing" vs
  // "drain only" without staring at vertical phase boundaries.
  shadedZones?: { from: Date; to: Date; fill?: string; label?: string }[],
  // Optional substring match: if a series' `kind` contains this token, render
  // a translucent colored area fill under its curve so it stands out as the
  // "tinted" series. Used to flag the PG-hosting node in Node CPU.
  highlightKindToken?: string,
  // Optional ordered list of area fills drawn BEFORE the lines (back to front
  // in this order, so the first entry is the backmost). Each entry tints any
  // series whose `kind` exactly equals `kind`. Used for the util group where
  // we want oversaturation as the backmost orange band with CPU util on top.
  areaFills?: { kind: string; color: string; opacity?: number }[],
  // Optional per-kind line color override. Without it, lines use d3's
  // schemeCategory10 in the order kinds were inserted into sumstat — which
  // means "oversaturation" wouldn't naturally come out orange. Used by the
  // util group to pin each series' line color to its area fill color.
  lineColorOverrides?: Record<string, string>,
  // Optional shared origin (epoch ms) for the relative-time x-axis. When
  // unset, each chart uses its own earliest data point — fine in isolation
  // but inconsistent across panels because pollers / pushers / CPU samples
  // start at slightly different moments. Pass meta.json's `bench_start_ms`
  // here so 0s on every panel is the same wall-clock moment.
  xRelativeOriginMs?: number,
) {
  const context = {
    jsdom: new JSDOM(""),
  };
  const { window } = context.jsdom;
  const { document } = window;

  const body = d3.select(document).select("body");

  // Multi-series time chart — 900px wide for the same reason as drawGraph above.
  // Throughput, Queue depth, Node CPU/memory, PG memory, Failed jobs, Workers per
  // node all flow through this; bumping width here is the user-requested
  // "make the lots-of-points charts 50% wider".
  const width = 900;
  const height = 200;

  const marginTop = 20;
  const marginRight = 100;
  const marginBottom = 30;
  const marginLeft = 60;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  svg = svg
    .append("g")
    .attr("transform", "translate(" + marginLeft + "," + marginTop + ")");

  const x = d3
    .scaleTime()
    .domain(
      d3.extent(data, function (d: DataPoint) {
        return d.date;
      })
    )
    // No .nice() on the time scale — it rounds the domain to "nice" tick
    // boundaries (e.g. extends domain to next 30s mark), pushing the first
    // few seconds of data off the left edge so phase 1 (idle baseline) and
    // m02's CPU curve appear to "start from the middle". Exact data extent
    // keeps the line starting at the actual first sample.
    .range([0, width]);

  // Relative-time x-axis: labels show seconds since the earliest data point
  // in this chart ("0s, 30s, 60s ..."). Wall-clock HH:MM:SS labels were
  // confusing — they alternated between "03:45" and ":30" formats and made
  // same-time comparisons across panels hard. Relative units anchor every
  // panel at 0s.
  const xDom = x.domain() as [Date, Date];
  const xOriginMs = xRelativeOriginMs ?? xDom[0].getTime();
  const xAxis = d3.axisBottom(x).ticks(5).tickFormat((d) => {
    const sec = Math.round(((d as Date).getTime() - xOriginMs) / 1000);
    return `${sec}s`;
  });

  svg
    .append("g")
    .attr("transform", "translate(0," + height + ")")
    .call(xAxis);

  // Add Y axis. When yMax is set, also `.clamp(true)` so out-of-range data
  // values (e.g. a CPU spike to 250% on a chart capped at 150%) render at the
  // axis boundary instead of escaping the chart area entirely.
  const y = d3
    .scaleLinear()
    .domain([
      0,
      yMax !== undefined ? yMax : d3.max(data, function (d: DataPoint) {
        return +d.value;
      }) * 1.5,
    ])
    .range([height, 0])
    .clamp(yMax !== undefined)
    .nice();
  svg.append("g").call(d3.axisLeft(y));

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 12px")
    .attr("transform", "rotate(-90)")
    .attr("y", -marginLeft + 20)
    .attr("x", -height / 2)
    .text(yLabel);

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 16px")
    .attr("y", 0)
    .attr("x", width / 2)
    .text(title);

  const sumstat = d3.group(data, function (d: DataPointMulti) {
    return d.kind;
  });

  const keys = Array.from(sumstat.keys());

  const color = d3
    .scaleOrdinal()
    .domain(keys)
    .range([
      "#e41a1c",
      "#377eb8",
      "#4daf4a",
      "#984ea3",
      "#ff7f00",
      "#ffff33",
      "#a65628",
      "#f781bf",
      "#999999",
    ]);

  // Shaded zones (e.g. push window) — drawn BEFORE the lines so lines
  // paint on top. Translucent fill so the chart underneath stays
  // readable. Optional label hugs the top-left of the zone.
  if (shadedZones && shadedZones.length > 0) {
    svg
      .selectAll(".shaded-zone")
      .data(shadedZones)
      .enter()
      .append("rect")
      .attr("class", "shaded-zone")
      .attr("x", (d: { from: Date }) => x(d.from))
      .attr("width", (d: { from: Date; to: Date }) => Math.max(0, x(d.to) - x(d.from)))
      .attr("y", 0)
      .attr("height", height)
      .attr("fill", (d: { fill?: string }) => d.fill ?? "#1f77b4")
      .attr("fill-opacity", 0.08)
      .attr("stroke", "none");
    svg
      .selectAll(".shaded-zone-label")
      .data(shadedZones.filter((z: { label?: string }) => z.label))
      .enter()
      .append("text")
      .attr("class", "shaded-zone-label")
      .attr("x", (d: { from: Date }) => x(d.from) + 4)
      .attr("y", 22)
      .attr("text-anchor", "start")
      .style("font-size", "10px")
      .style("font-weight", "600")
      .style("fill", "#3a5d8a")
      .style("font-family", "monospace")
      .text((d: { label?: string }) => d.label!);
  }

  // Explicit area-fills list — drawn in order so caller controls back-to-front
  // z-order. Each is the area under the matching series' curve.
  if (areaFills && areaFills.length > 0) {
    const entries = Array.from(sumstat as any) as any[];
    for (const fill of areaFills) {
      const match = entries.find((e: any) => String(e[0]) === fill.kind);
      if (!match) continue;
      svg.append("path")
        .attr("class", "area-fill")
        .attr("fill", fill.color)
        .attr("fill-opacity", fill.opacity ?? 0.25)
        .attr("stroke", "none")
        .attr("d", d3
          .area()
          .x((p: any) => x(p.date))
          .y0(height)
          .y1((p: any) => y(p.value))(match[1]));
    }
  }

  // Tinted area fill under the highlighted series — drawn BEFORE the line so
  // line stays crisp on top. Translucent so the underlying x-axis and
  // overlapping series remain visible. sumstat is a d3.InternMap (Map-like);
  // convert to entries Array to filter.
  if (highlightKindToken) {
    const highlightSeries = Array.from(sumstat as any).filter(
      (d: any) => String(d[0]).includes(highlightKindToken),
    );
    svg
      .selectAll("path.tint")
      .data(highlightSeries)
      .join("path")
      .attr("class", "tint")
      .attr("fill", function (d: any) { return color(d[0]); })
      .attr("fill-opacity", 0.18)
      .attr("stroke", "none")
      .attr("d", (d: any) => {
        return d3
          .area()
          .x((p: any) => x(p.date))
          .y0(height)
          .y1((p: any) => y(p.value))(d[1]);
      });
  }

  // Add the line
  svg
    .selectAll("path.line")
    .data(sumstat)
    .join("path")
    .attr("class", "line")
    .attr("fill", "none")
    .attr("stroke", function (d) {
      return (lineColorOverrides && lineColorOverrides[String(d[0])]) ?? color(d[0]);
    })
    .attr("stroke-width", 1.5)
    .attr("d", (d) => {
      return d3
        .line()
        .x((d) => x(d.date))
        .y((d) => y(d.value))(d[1]);
    });

  const size = 15;
  svg
    .selectAll(".dot")
    .data(keys)
    .enter()
    .append("rect")
    .attr("class", "dot")
    .attr("x", 400)
    .attr("y", function (d, i) {
      return 5 + i * (size + 5);
    })
    .attr("width", size)
    .attr("height", size)
    .style("fill", function (d) {
      return color(d);
    });
  svg
    .selectAll(".label")
    .data(keys)
    .enter()
    .append("text")
    .attr("class", "label")
    .attr("x", 400 + size * 1.2)
    .attr("y", function (d, i) {
      return 5 + i * (size + 5) + size / 2;
    })
    .style("fill", function (d) {
      return color(d);
    })
    .text(function (d) {
      return d;
    })
    .attr("text-anchor", "left")
    .style("alignment-baseline", "middle");

  // Phase-boundary dashed verticals. Drawn last so they overlay the data lines.
  // Each line gets a tiny "P{n}>P{n+1}" label at the top so the migration is
  // visible at a glance — useful for phased benches where throughput / CPU
  // shifts between phases.
  // Normalize verticalLines into { dates, hideLabels }.
  const vlNormalized = Array.isArray(verticalLines)
    ? { dates: verticalLines, hideLabels: false }
    : verticalLines;
  if (vlNormalized && vlNormalized.dates.length > 0) {
    svg
      .selectAll(".phase-boundary")
      .data(vlNormalized.dates)
      .enter()
      .append("line")
      .attr("class", "phase-boundary")
      .attr("x1", (d: Date) => x(d))
      .attr("x2", (d: Date) => x(d))
      .attr("y1", 0)
      .attr("y2", height)
      .attr("stroke", "#888")
      .attr("stroke-width", 1)
      .attr("stroke-dasharray", "3 3");
    if (!vlNormalized.hideLabels) {
      svg
        .selectAll(".phase-boundary-label")
        .data(vlNormalized.dates)
        .enter()
        .append("text")
        .attr("class", "phase-boundary-label")
        .attr("x", (d: Date) => x(d) + 2)
        .attr("y", 10)
        .attr("text-anchor", "start")
        .style("font-size", "9px")
        .style("fill", "#666")
        .style("font-family", "monospace")
        .text((_: Date, i: number) => `P${i + 1}>P${i + 2}`);
    }
  }

  // Horizontal reference lines (e.g. CPU 100% ceiling) — labeled dashed
  // grey, with the label hugging the right edge so it doesn't collide
  // with the data lines.
  if (horizontalLines && horizontalLines.length > 0) {
    svg
      .selectAll(".h-ref")
      .data(horizontalLines)
      .enter()
      .append("line")
      .attr("class", "h-ref")
      .attr("x1", 0)
      .attr("x2", width)
      .attr("y1", (d: { y: number; label: string }) => y(d.y))
      .attr("y2", (d: { y: number; label: string }) => y(d.y))
      .attr("stroke", "#999")
      .attr("stroke-width", 1)
      .attr("stroke-dasharray", "4 4");
    svg
      .selectAll(".h-ref-label")
      .data(horizontalLines)
      .enter()
      .append("text")
      .attr("class", "h-ref-label")
      .attr("x", width - 4)
      .attr("y", (d: { y: number; label: string }) => y(d.y) - 4)
      .attr("text-anchor", "end")
      .style("font-size", "9px")
      .style("fill", "#666")
      .style("font-family", "monospace")
      .text((d: { y: number; label: string }) => d.label);
  }

  return body.node().innerHTML;
}

// Bar chart for one-dimensional distributions. Each bin is { label, count }.
// Works for both continuous (histogram bins, label = midpoint as string) and
// categorical (label = category name). Summary stats are rendered as text in
// the top-right.
export function drawBars(
  bins: { label: string; count: number; color?: string }[],
  title: string,
  xLabel: string,
  stats?: { min: number; max: number; avg: number },
  opts: { rotateDeg?: number; fontSize?: string } = {},
): string {
  const context = { jsdom: new JSDOM("") };
  const { document } = context.jsdom.window;
  const body = d3.select(document).select("body");

  const width = 600;
  const height = 200;
  const marginTop = 30;
  const marginRight = 30;
  // 100px bottom margin so long rotated tick labels (e.g. OOM panel pod
  // names like "windmill-postgresql-0 (12.5G) (cgroup)") aren't cropped.
  const marginBottom = 100;
  const marginLeft = 60;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  svg = svg
    .append("g")
    .attr("transform", "translate(" + marginLeft + "," + marginTop + ")");

  // Title
  svg
    .append("text")
    .attr("x", width / 2)
    .attr("y", -10)
    .attr("text-anchor", "middle")
    .style("font-size", "14px")
    .style("font-weight", "600")
    .text(title);

  const x = d3.scaleBand()
    .domain(bins.map((b) => b.label))
    .range([0, width])
    .padding(0.1);

  const y = d3.scaleLinear()
    .domain([0, d3.max(bins, (b) => b.count) || 1])
    .nice()
    .range([height, 0]);

  // X axis — label EVERY bar so the user can read what each one is.
  // rotateDeg / fontSize are caller-tunable: -45° + 10px is fine for
  // ~12 bars; -90° + smaller font lets a chart fit ~100 labels (per-pod
  // memory bars on a ~80-worker cluster).
  const rotateDeg = opts.rotateDeg ?? -45;
  const fontSize = opts.fontSize ?? (bins.length > 12 ? "9px" : "10px");
  svg.append("g")
    .attr("transform", "translate(0," + height + ")")
    .call(
      d3.axisBottom(x).tickValues(bins.map((b) => b.label)),
    )
    .selectAll("text")
    .attr("transform", `rotate(${rotateDeg})`)
    .style("text-anchor", "end")
    .style("font-size", fontSize);

  svg.append("text")
    .attr("x", width / 2)
    .attr("y", height + 90)   // ↓ below rotated tick labels (was 45)
    .attr("text-anchor", "middle")
    .style("font-size", "11px")
    .text(xLabel);

  svg.append("g").call(d3.axisLeft(y));

  svg.selectAll(".bar")
    .data(bins)
    .join("rect")
    .attr("class", "bar")
    .attr("x", (b) => x(b.label) || 0)
    .attr("y", (b) => y(b.count))
    .attr("width", x.bandwidth())
    .attr("height", (b) => height - y(b.count))
    .attr("fill", (b: { color?: string }) => b.color ?? "#377eb8");

  // Dashed vertical divider lines between contiguous groups of same-color
  // bars. With the Pod inventory chart (one node = one color), this draws a
  // separator between m02's run of bars and m03's, etc., making it
  // visually obvious which bars belong to which node.
  const groupBoundaries: number[] = [];
  for (let i = 1; i < bins.length; i++) {
    if (bins[i].color !== bins[i - 1].color) groupBoundaries.push(i);
  }
  if (groupBoundaries.length > 0) {
    const bandStep = x.step();
    svg.selectAll(".group-divider")
      .data(groupBoundaries)
      .enter()
      .append("line")
      .attr("class", "group-divider")
      .attr("x1", (i: number) => (x(bins[i].label) || 0) - bandStep * (1 - x.bandwidth() / bandStep) / 2)
      .attr("x2", (i: number) => (x(bins[i].label) || 0) - bandStep * (1 - x.bandwidth() / bandStep) / 2)
      .attr("y1", 0)
      .attr("y2", height + 5)
      .attr("stroke", "#aaa")
      .attr("stroke-width", 1)
      .attr("stroke-dasharray", "3 3");
  }

  // Stats text top-right.
  if (stats) {
    const fmt = (n: number) =>
      n >= 1000 ? n.toFixed(0) : n >= 10 ? n.toFixed(1) : n.toFixed(2);
    const lines = [
      `min: ${fmt(stats.min)}`,
      `avg: ${fmt(stats.avg)}`,
      `max: ${fmt(stats.max)}`,
    ];
    svg.selectAll(".stat")
      .data(lines)
      .enter()
      .append("text")
      .attr("class", "stat")
      .attr("x", width)
      .attr("y", (_, i) => 12 + i * 14)
      .attr("text-anchor", "end")
      .style("font-size", "11px")
      .style("font-family", "monospace")
      .style("fill", "#555")
      .text((d) => d);
  }

  return body.node().innerHTML;
}

// Donut chart for categorical distributions. Slices labeled with category +
// percentage; legend on the right.
export function drawDonut(
  slices: { label: string; count: number }[],
  title: string,
): string {
  const context = { jsdom: new JSDOM("") };
  const { document } = context.jsdom.window;
  const body = d3.select(document).select("body");

  const width = 600;
  const height = 220;
  const marginTop = 30;
  const marginBottom = 10;
  const marginLeft = 20;
  const marginRight = 20;

  const total = slices.reduce((a, b) => a + b.count, 0) || 1;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  // Title
  svg
    .append("text")
    .attr("x", (width + marginLeft + marginRight) / 2)
    .attr("y", marginTop - 8)
    .attr("text-anchor", "middle")
    .style("font-size", "14px")
    .style("font-weight", "600")
    .text(title);

  const r = Math.min(width, height) / 2 - 10;
  const cx = marginLeft + r + 10;
  const cy = marginTop + height / 2;

  const palette = [
    "#377eb8", "#e41a1c", "#4daf4a", "#984ea3",
    "#ff7f00", "#a65628", "#f781bf", "#999999",
  ];
  const color = (i: number) => palette[i % palette.length];

  const pie = d3.pie<{ label: string; count: number }>().value((d) => d.count).sort(null);
  const arc = d3.arc<d3.PieArcDatum<{ label: string; count: number }>>()
    .innerRadius(r * 0.55)
    .outerRadius(r);

  const arcs = pie(slices);
  const g = svg.append("g").attr("transform", `translate(${cx},${cy})`);

  g.selectAll("path")
    .data(arcs)
    .join("path")
    .attr("d", arc as never)
    .attr("fill", (_, i) => color(i))
    .attr("stroke", "white")
    .attr("stroke-width", 2);

  // Legend on the right
  const legendX = cx + r + 30;
  const legendY = marginTop + 20;
  const sw = 12;
  slices.forEach((s, i) => {
    const pct = ((s.count / total) * 100).toFixed(1);
    svg.append("rect")
      .attr("x", legendX)
      .attr("y", legendY + i * 22)
      .attr("width", sw)
      .attr("height", sw)
      .attr("fill", color(i));
    svg.append("text")
      .attr("x", legendX + sw + 6)
      .attr("y", legendY + i * 22 + sw - 1)
      .style("font-size", "12px")
      .style("font-family", "monospace")
      .style("fill", "#333")
      .text(`${s.label}  ${pct}%`);
  });

  return body.node().innerHTML;
}

if (import.meta.main) {
  const svg = drawGraph(
    [
      {
        value: 10,
        date: new Date(86400000),
      },
      {
        value: 12,
        date: new Date(86400000 * 2),
      },
    ],
    "test"
  );

  const svg2 = drawGraphMulti(
    [
      {
        value: 10,
        date: new Date(86400000),
        kind: "test",
      },
      {
        value: 12,
        date: new Date(86400000 * 2),
        kind: "test",
      },
      {
        value: 8,
        date: new Date(86400000),
        kind: "test2",
      },
      {
        value: 9,
        date: new Date(86400000 * 2),
        kind: "test2",
      },
    ],
    "test"
  );

  console.log(svg);
  console.log(svg2);
  Deno.exit(0);
}
