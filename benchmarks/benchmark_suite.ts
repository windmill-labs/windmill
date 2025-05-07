import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

import { main as runBenchmark } from "./benchmark_oneoff.ts";

import { VERSION } from "./lib.ts";

type Config = {
  kind: string;
  jobs: number;
  noSave?: boolean;
}[];

async function warmUp(
  host: string,
  email: string | undefined,
  password: string | undefined,
  token: string | undefined,
  workspace: string
) {
  console.log("%cWarming up...", "font-weight: bold;");
  await runBenchmark({
    host,
    email,
    password,
    token,
    workspace,
    kind: "noop",
    jobs: 50000,
  });
}

async function main({
  host,
  email,
  password,
  token,
  workspace,
  configPath,
  workers,
  factor
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  configPath: string;
  workers: number;
  factor?: number;
}) {
  async function getConfig(configPath: string): Promise<Config> {
    if (configPath.startsWith("http")) {
      const response = await fetch(configPath);
      return await response.json();
    } else {
      return JSON.parse(await Deno.readTextFile(configPath));
    }
  }

  if (!Deno.args.includes("--no-warm-up")) {
    await warmUp(host, email, password, token, workspace);
  }

  try {
    const config = await getConfig(configPath);
    for (const benchmark of config) {
      try {
        console.log(
          "%cRunning benchmark " + benchmark.kind,
          "font-weight: bold;"
        );

        const result = await runBenchmark({
          host,
          email,
          password,
          token,
          workspace,
          kind: benchmark.kind,
          jobs: benchmark.jobs * (factor ?? 1),
        });

        if (benchmark.noSave) {
          continue;
        }

        if (!result) {
          throw new Error("No result returned");
        }
        const stat = {
          value: result.throughput,
          ts: Date.now(),
        };
        let data: (typeof stat)[] = [];
        const benchmarkName =
          benchmark.kind + (workers > 1 ? `_${workers}workers` : "");
        const jsonFilePath = `${benchmarkName}_benchmark.json`;
        try {
          const remotePath =
            "https://raw.githubusercontent.com/windmill-labs/windmill/benchmarks/" +
            jsonFilePath;
          data = await fetch(remotePath).then((r) => r.json());
        } catch (_) {
          console.log("No existing data file found, creating new one.");
        }
        data.push(stat);
        await Deno.writeTextFile(jsonFilePath, JSON.stringify(data, null, 4));
      } catch (err) {
        console.error("Failed to run benchmark", benchmark.kind, err);
      }
    }
  } catch (err) {
    console.error(`Failed to read config file ${configPath}: ${err}`);
  }
}

await new Command()
  .name("wmillbenchsuite")
  .description("Run benchmark suite to measure throughput of windmill.")
  .version(VERSION)
  .option("--host <url:string>", "The windmill host to benchmark.", {
    default: "http://127.0.0.1:8000",
  })
  .option("-e --email <email:string>", "The email to use to login.", {
    default: "admin@windmill.dev",
  })
  .option("-p --password <password:string>", "The password to use to login.", {
    default: "changeme",
  })
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
  .option("--no-warm-up", "Skip the warm up phase.")
  .option(
    "--workers <workers:number>",
    "Number of workers that are used to run the benchmarks (only affect graph title)",
    { default: 1 }
  )
  .option("--factor <factor:number>", "Factor to multiply the number of jobs by.", {
    default: 1,
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
