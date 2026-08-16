/**
 * Generate SVG bar charts from competitor benchmark results.
 *
 * Usage:
 *   deno run -A competitor_graphs.ts -c competitor_graphs_config.json --results-dir ./results
 */

import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import { JSDOM } from "https://jspm.dev/jsdom@22";
import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";

interface GraphConfig {
  graph_title: string;
  metric: string;
  unit: string;
  lower_is_better: boolean;
  filename: string;
}

interface SummaryEntry {
  competitor: string;
  metric: string;
  value: number;
  ts: number;
}

const COLORS: Record<string, string> = {
  windmill: "#3b82f6",
  temporal: "#8b5cf6",
  inngest: "#f59e0b",
  restate: "#10b981",
  kestra: "#ef4444",
};

function drawBarChart(
  data: { name: string; value: number }[],
  title: string,
  unit: string,
  lowerIsBetter: boolean,
): string {
  const context = { jsdom: new JSDOM("") };
  const { document } = context.jsdom.window;
  const body = d3.select(document).select("body");

  const width = 500;
  const height = 280;
  const marginTop = 40;
  const marginRight = 30;
  const marginBottom = 50;
  const marginLeft = 80;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg.append("rect").attr("width", "100%").attr("height", "100%").attr("fill", "white");

  svg = svg
    .append("g")
    .attr("transform", `translate(${marginLeft},${marginTop})`);

  // X scale (competitors)
  const x = d3
    .scaleBand()
    .domain(data.map((d) => d.name))
    .range([0, width])
    .padding(0.3);

  // Y scale (metric values)
  // deno-lint-ignore no-explicit-any
  const maxVal = d3.max(data, (d: any) => d.value) ?? 1;
  const y = d3
    .scaleLinear()
    .domain([0, maxVal * 1.2])
    .nice()
    .range([height, 0]);

  // Axes
  svg
    .append("g")
    .attr("transform", `translate(0,${height})`)
    .call(d3.axisBottom(x))
    .selectAll("text")
    .attr("style", "font-size: 12px; font-weight: bold");

  svg.append("g").call(d3.axisLeft(y).ticks(6));

  // Y axis label
  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 12px")
    .attr("transform", "rotate(-90)")
    .attr("y", -marginLeft + 20)
    .attr("x", -height / 2)
    .text(`[${unit}]`);

  // Title
  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 14px; font-weight: bold")
    .attr("y", -15)
    .attr("x", width / 2)
    .text(title);

  // Subtitle
  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 10px; fill: #666")
    .attr("y", -2)
    .attr("x", width / 2)
    .text(lowerIsBetter ? "(lower is better)" : "(higher is better)");

  // Bars
  svg
    .selectAll(".bar")
    .data(data)
    .join("rect")
    .attr("class", "bar")
    .attr("x", (d: any) => x(d.name)!)
    .attr("y", (d: any) => y(d.value))
    .attr("width", x.bandwidth())
    .attr("height", (d: any) => height - y(d.value))
    .attr("fill", (d: any) => COLORS[d.name] ?? "#999")
    .attr("rx", 3);

  // Value labels on top of bars
  svg
    .selectAll(".label")
    .data(data)
    .join("text")
    .attr("class", "label")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 11px; font-weight: bold")
    .attr("x", (d: any) => x(d.name)! + x.bandwidth() / 2)
    .attr("y", (d: any) => y(d.value) - 5)
    .text((d: any) => `${d.value.toFixed(1)}`);

  return body.node().innerHTML;
}

async function main({
  configPath,
  resultsDir,
  outputDir,
}: {
  configPath: string;
  resultsDir: string;
  outputDir: string;
}) {
  const configs: GraphConfig[] = JSON.parse(
    await Deno.readTextFile(configPath),
  );

  // Load the summary file
  let summary: SummaryEntry[];
  try {
    summary = JSON.parse(
      await Deno.readTextFile(`${resultsDir}/competitor_comparison_benchmark.json`),
    );
  } catch (e) {
    console.error(`Failed to load summary from ${resultsDir}:`, e);
    Deno.exit(1);
  }

  try {
    await Deno.mkdir(outputDir, { recursive: true });
  } catch (_) {
    // already exists
  }

  for (const config of configs) {
    const entries = summary.filter((e) => e.metric === config.metric);
    if (entries.length === 0) {
      console.warn(`No data for metric: ${config.metric}, skipping`);
      continue;
    }

    // Use the latest entry per competitor
    const latest = new Map<string, SummaryEntry>();
    for (const e of entries) {
      const existing = latest.get(e.competitor);
      if (!existing || e.ts > existing.ts) {
        latest.set(e.competitor, e);
      }
    }

    const data = Array.from(latest.values()).map((e) => ({
      name: e.competitor,
      value: e.value,
    }));

    // Sort: best first (lower-is-better → ascending, higher-is-better → descending)
    data.sort((a, b) =>
      config.lower_is_better ? a.value - b.value : b.value - a.value,
    );

    const svgContent = drawBarChart(
      data,
      config.graph_title,
      config.unit,
      config.lower_is_better,
    );

    const filepath = `${outputDir}/${config.filename}`;
    await Deno.writeTextFile(filepath, svgContent);
    console.log(`Generated: ${filepath}`);
  }
}

await new Command()
  .name("competitor-graphs")
  .description("Generate SVG bar charts from competitor benchmark results.")
  .version("1.0.0")
  .option("-c --config-path <path:string>", "Path to graph config JSON", {
    required: true,
  })
  .option("--results-dir <path:string>", "Directory containing result JSON files", {
    default: "./results",
  })
  .option("--output-dir <path:string>", "Directory to write SVG files", {
    default: "./results",
  })
  .action(main)
  .parse();
