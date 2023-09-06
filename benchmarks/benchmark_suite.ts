import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

const VERSION = "1.167.0";

type Config = {
  benchmarks: [
    {
      graph_title: string;
      name: string;
      jobs: number | undefined;
      type: "noop" | "flow" | "deno" | "python" | "go" | "bash";
    }
  ];
};

async function main({
  host,
  email,
  password,
  token,
  workspace,
  configPath,
  branch,
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  configPath: string;
  branch?: string;
}) {
  const { main: runNoopBenchmark } = await import(
    branch !== undefined
      ? `https://raw.githubusercontent.com/windmill-labs/windmill/${branch}/benchmarks/benchmark_noop.ts`
      : "./benchmark_noop.ts"
  );

  const { main: runBenchmark } = await import(
    branch !== undefined
      ? `https://raw.githubusercontent.com/windmill-labs/windmill/${branch}/benchmarks/main.ts`
      : "./main.ts"
  );

  const { drawGraph } = await import(
    branch !== undefined
      ? `https://raw.githubusercontent.com/windmill-labs/windmill/${branch}/benchmarks/graph.ts`
      : "./graph.ts"
  );

  async function getConfig(configPath: string): Promise<Config> {
    if (configPath.startsWith("http")) {
      const response = await fetch(configPath);
      return await response.json();
    } else {
      return JSON.parse(await Deno.readTextFile(configPath));
    }
  }

  try {
    const config = await getConfig(configPath);
    for (const benchmark of config.benchmarks) {
      try {
        console.log(
          "%cRunning benchmark " + benchmark.name,
          "font-weight: bold;"
        );

        let result:
          | {
              throughput: number;
            }
          | undefined;
        if (benchmark.type === "noop") {
          result = await runNoopBenchmark({
            host,
            email,
            password,
            token,
            workspace,
            jobs: 1000,
            batches: 1,
          });
        } else {
          result = await runBenchmark({
            host,
            email,
            password,
            token,
            workspace,
            workers: 1,
            seconds: benchmark.type === "flow" ? 2 : 5,
            metrics: "http://localhost:8001/metrics",
            maximumThroughput: Infinity,
            zombieTimeout: 90000,
            histogramBuckets: [],
            scriptPattern: [
              "deno",
              "python",
              "go",
              "bash",
              "dedicated",
            ].includes(benchmark.type)
              ? benchmark.type
              : "deno",
            useFlows: benchmark.type === "flow",
          });
        }

        if (!result) {
          throw new Error("No result returned");
        }
        const stat = {
          value: result.throughput,
          ts: Date.now(),
        };
        let data: (typeof stat)[] = [];
        const jsonFilePath = `${benchmark.name}.json`;
        try {
          const existing = await Deno.readTextFile(jsonFilePath);
          data = JSON.parse(existing);
        } catch (_) {
          console.log("No existing data file found, creating new one.");
        }
        data.push(stat);
        await Deno.writeTextFile(jsonFilePath, JSON.stringify(data, null, 4));
        const svg = drawGraph(
          data.slice(-10).map((d) => ({ ...d, date: new Date(d.ts) })),
          benchmark.graph_title
        );
        await Deno.writeTextFile(`${benchmark.name}.svg`, svg);
      } catch (err) {
        console.error("Failed to run benchmark", benchmark.name, err);
      }
    }

    Deno.exit(0); // JSDOM from drawGraph doesn't exit cleanly
  } catch (err) {
    return console.error(`Failed to read config file ${configPath}: ${err}`);
  }
}

await new Command()
  .name("wmillbenchsuite")
  .description("Run benchmark suite to measure throughput of windmill.")
  .version(VERSION)
  .option("--host <url:string>", "The windmill host to benchmark.", {
    default: "http://127.0.0.1:8000",
  })
  .option("-e --email <email:string>", "The email to use to login.")
  .option("-p --password <password:string>", "The password to use to login.")
  .env(
    "WM_TOKEN=<token:string>",
    "The token to use when talking to the API server. Preferred over manual login."
  )
  .option(
    "-t --token <token:string>",
    "The token to use when talking to the API server. Preferred over manual login."
  )
  .env(
    "WM_WORKSPACE=<workspace:string>",
    "The workspace to spawn scripts from."
  )
  .option(
    "-w --workspace <workspace:string>",
    "The workspace to spawn scripts from.",
    { default: "admins" }
  )
  .option("-c --config-path <config:string>", "The path of the config file", {
    required: true,
  })
  .option(
    "--branch <branch:string>",
    "The branch to use when running remotely."
  )
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
