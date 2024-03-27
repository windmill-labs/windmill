import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

import { drawGraph, drawGraphMulti } from "./graph.ts";
import { VERSION } from "./lib.ts";

type GraphsConfig = [
  {
    graph_title: string;
    benchmarks: {
      kind: string;
      workers: number;
      label: string;
    }[];
    jobs: number;
  }
];

async function main({ configPath }: { configPath: string }) {
  async function getConfig(configPath: string): Promise<GraphsConfig> {
    if (configPath.startsWith("http")) {
      const response = await fetch(configPath);
      return await response.json();
    } else {
      return JSON.parse(await Deno.readTextFile(configPath));
    }
  }

  try {
    const config = await getConfig(configPath);

    for (const graphConfig of config || []) {
      const data: {
        value: number;
        ts: number;
        date: Date;
        kind: string;
      }[] = [];
      for (const benchmark of graphConfig.benchmarks) {
        const benchmarkName =
          benchmark.kind +
          (benchmark.workers > 1 ? `_${benchmark.workers}workers` : "");
        try {
          const existing = await Deno.readTextFile(
            `${benchmarkName}_benchmark.json`
          );
          const existingData = JSON.parse(existing)
            .map((d: { value: number; ts: number }) => ({
              ...d,
              date: new Date(d.ts),
              kind: benchmark.label,
            }))
            .slice(-10);
          data.push(...existingData);
        } catch (err) {
          console.log(
            "Error while loading",
            benchmark.kind,
            benchmark.workers > 1
              ? `(${benchmark.workers} workers)`
              : "(single worker)",
            "benchmark data",
            err
          );
        }
      }
      const svg =
        graphConfig.benchmarks.length > 1
          ? drawGraphMulti(data, graphConfig.graph_title)
          : drawGraph(data, graphConfig.graph_title);
      const fileName = graphConfig.graph_title.replace(/ /g, "_");
      await Deno.writeTextFile(`${fileName}.svg`, svg);
    }

    Deno.exit(0); // JSDOM from drawGraph doesn't exit cleanly
  } catch (err) {
    console.error(`Failed to read config file ${configPath}: ${err}`);
    Deno.exit(0); // JSDOM from drawGraph doesn't exit cleanly
  }
}

await new Command()
  .name("wmillbenchsuite")
  .description("Create and save graphs from benchmark data.")
  .version(VERSION)
  .option("-c --config-path <config:string>", "The path of the config file", {
    required: true,
  })
  .action(main)
  .command(
    "upgrade",
    new UpgradeCommand({
      main: "main.ts",
      args: [
        "--allow-net",
        "--allow-read",
        "--allow-write",
        "--allow-env",
        "--unstable",
      ],
      provider: new DenoLandProvider({ name: "wmillbench" }),
    })
  )
  .parse();
