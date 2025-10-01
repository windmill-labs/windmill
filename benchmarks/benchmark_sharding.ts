/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";

import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.174.0/windmill-api/index.ts";

import { VERSION, createBenchScript, getFlowPayload, login } from "./lib.ts";

export const NON_TEST_TAGS = [
  "deno",
  "python",
  "go",
  "bash",
  "dedicated",
  "bun",
  "nativets",
  "flow",
];

interface ShardingBenchmarkResult {
  numShards: number;
  throughput: number;
  totalDuration: number;
  jobsCompleted: number;
  avgLatency: number;
}

interface ShardingBenchmarkOptions {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  kind: string;
  jobs: number;
  maxShards: number;
  noVerify?: boolean;
}

interface BenchmarkContext {
  config: {
    token: string;
    server: string;
    workspace_id: string;
  };
  nStepsFlow: number;
  bodyTemplate: any;
  getQueueCount: (tags?: string[], num_shards?: number) => Promise<number>;
  getCompletedJobsCount: (
    tags?: string[],
    baseline?: number,
    num_shards?: number
  ) => Promise<number>;
}

async function verifyOutputs(uuids: string[], workspace: string) {
  console.log("Verifying outputs");
  let incorrectResults = 0;
  for (const uuid of uuids) {
    try {
      const job = await windmill.JobService.getCompletedJob({
        workspace,
        id: uuid,
      });
      if (!job.success) {
        console.log(`Job ${uuid} did not complete`);
        incorrectResults++;
      }
      if (job.result !== uuid) {
        console.log(
          `Job ${uuid} did not output the correct value: ${JSON.stringify(job)}`
        );
        incorrectResults++;
      }
    } catch (_) {
      console.log(`Job ${uuid} did not complete`);
      incorrectResults++;
    }
  }
  console.log(`Incorrect results: ${incorrectResults}`);
}

async function initializeBenchmarkContext(
  options: ShardingBenchmarkOptions
): Promise<BenchmarkContext> {
  const { host, workspace, kind } = options;

  windmill.setClient("", host);

  const config = {
    token: "",
    server: host,
    workspace_id: workspace,
  };

  let final_token: string;
  if (!options.token) {
    if (options.email && options.password) {
      final_token = await login(options.email, options.password);
    } else {
      throw new Error("Token or email with password are required.");
    }
  } else {
    final_token = options.token;
  }

  config.token = final_token;
  windmill.setClient(final_token, host);

  async function getQueueCount(tags?: string[], numShards?: number) {
    const params = new URLSearchParams();
    if (tags && tags.length > 0) {
      params.set("tags", tags.join(","));
    }
    if (numShards !== undefined) {
      params.set("num_shards", numShards.toString());
    }
    const queryString = params.toString();
    return (
      await (
        await fetch(
          config.server +
            "/api/w/" +
            config.workspace_id +
            "/jobs/queue/count" +
            (queryString ? "?" + queryString : ""),
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length;
  }

  async function getFlowStepCount(
    workspace: string,
    path: string
  ): Promise<number> {
    const response = await fetch(
      `${config.server}/api/w/${workspace}/flows/get/${path}`,
      { headers: { ["Authorization"]: "Bearer " + config.token } }
    );

    const data = await response.json();
    let stepCount = 0;

    for (const mod of data.value.modules) {
      if (mod.value.type === "flow" && mod.value.path) {
        const subFlowCount = await getFlowStepCount(workspace, mod.value.path);
        stepCount += subFlowCount;
      } else {
        stepCount += 1;
      }
    }

    return stepCount;
  }

  async function getCompletedJobsCount(
    tags?: string[],
    baseline: number = 0,
    numShards?: number
  ): Promise<number> {
    const params = new URLSearchParams();
    if (tags && tags.length > 0) {
      params.set("tags", tags.join(","));
    }
    if (numShards !== undefined) {
      params.set("num_shards", numShards.toString());
    }
    const queryString = params.toString();
    const completedJobs = (
      await (
        await fetch(
          host +
            "/api/w/" +
            config.workspace_id +
            "/jobs/completed/count" +
            (queryString ? "?" + queryString : ""),
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length;
    return completedJobs - baseline;
  }

  if (
    ["deno", "python", "go", "bash", "dedicated", "bun", "nativets"].includes(
      kind
    )
  ) {
    await createBenchScript(kind, workspace);
  }

  // Determine job body template and flow steps
  let nStepsFlow = 0;
  let bodyTemplate: any;

  if (kind === "noop") {
    bodyTemplate = {
      kind: "noop",
    };
  } else if (
    ["deno", "python", "go", "bash", "dedicated", "bun", "nativets"].includes(
      kind
    )
  ) {
    bodyTemplate = {
      kind: "script",
      path: "f/benchmarks/" + kind,
    };
  } else if (["2steps", "bigscriptinflow"].includes(kind)) {
    nStepsFlow = kind == "2steps" ? 2 : 1;
    const payload = getFlowPayload(kind);
    bodyTemplate = {
      kind: "flow",
      flow_value: payload.value,
    };
  } else if (kind.startsWith("flow:")) {
    console.log("Detected custom flow ");
    let flow_path = kind.substr(5);
    nStepsFlow = await getFlowStepCount(config.workspace_id, flow_path);
    console.log(`Total steps of flow including sub-flows: ${nStepsFlow}`);
    bodyTemplate = {
      kind: "flow",
      path: flow_path,
    };
  } else if (kind.startsWith("script:")) {
    console.log("Detected custom script");
    bodyTemplate = {
      kind: "script",
      path: kind.substr(7),
    };
  } else if (kind == "bigrawscript") {
    bodyTemplate = {
      kind: "rawscript",
      rawscript: {
        language: api.RawScript.language.BASH,
        content:
          "# let's bloat that bash script, 3.. 2.. 1.. BOOM\n".repeat(100) +
          'echo "$WM_FLOW_JOB_ID"\n',
      },
    };
  } else {
    throw new Error("Unknown script pattern " + kind);
  }

  return {
    config,
    nStepsFlow,
    bodyTemplate,
    getQueueCount,
    getCompletedJobsCount,
  };
}

async function runShardingBenchmark(
  context: BenchmarkContext,
  options: ShardingBenchmarkOptions,
  numShards: number
): Promise<ShardingBenchmarkResult> {
  const { jobs, kind } = options;
  const {
    config,
    nStepsFlow,
    bodyTemplate,
    getQueueCount,
    getCompletedJobsCount,
  } = context;
  console.log(`\n=== Running benchmark with ${numShards} shard(s) ===`);

  const pastJobs = await getCompletedJobsCount(NON_TEST_TAGS, 0, numShards);

  const enc = (s: string) => new TextEncoder().encode(s);
  const jobsSent = jobs;
  console.log(`Bulk creating ${jobsSent} jobs`);

  const start_create = Date.now();

  // Create request body with number_of_shards
  const body = JSON.stringify({
    ...bodyTemplate,
    number_of_shards: numShards,
  });

  const response = await fetch(
    config.server +
      "/api/w/" +
      config.workspace_id +
      `/jobs/add_batch_jobs/${jobsSent}`,
    {
      method: "POST",
      headers: {
        ["Authorization"]: "Bearer " + config.token,
        "Content-Type": "application/json",
      },
      body,
    }
  );

  if (!response.ok) {
    throw new Error(
      "Failed to create jobs: " +
        response.statusText +
        " " +
        (await response.text())
    );
  }

  const uuids = await response.json();
  const end_create = Date.now();
  const create_duration = end_create - start_create;
  console.log(
    `Jobs successfully added to the queue in ${
      create_duration / 1000
    }s. Windmill will start pulling them\n`
  );

  let start = Date.now();
  let completedJobs = 0;
  let lastElapsed = 0;
  let lastCompletedJobs = 0;
  let totalMonitoringOverhead = 0;

  let didStart = false;
  while (completedJobs < jobsSent) {
    if (!didStart) {
      const queueCheckStart = Date.now();
      const actual_queue = await getQueueCount(NON_TEST_TAGS, numShards);
      totalMonitoringOverhead += Date.now() - queueCheckStart;

      if (actual_queue < jobsSent) {
        start = Date.now();
        totalMonitoringOverhead = 0;
        didStart = true;
      }
    } else {
      await sleep(1);
      const monitoringStart = Date.now();
      completedJobs = await getCompletedJobsCount(
        NON_TEST_TAGS,
        pastJobs,
        numShards
      );
      totalMonitoringOverhead += Date.now() - monitoringStart;

      const elapsed = start
        ? Math.max(0, Date.now() - start - totalMonitoringOverhead)
        : 0;
      if (nStepsFlow > 0) {
        completedJobs = Math.floor(completedJobs / (nStepsFlow + 1));
      }
      const avgThr = ((completedJobs / elapsed) * 1000).toFixed(2);
      const instThr =
        lastElapsed > 0
          ? (
              ((completedJobs - lastCompletedJobs) / (elapsed - lastElapsed)) *
              1000
            ).toFixed(2)
          : 0;

      lastElapsed = elapsed;
      lastCompletedJobs = completedJobs;

      await Deno.stdout.write(
        enc(
          `[${numShards} shards] elapsed: ${(elapsed / 1000).toFixed(
            2
          )} | jobs executed: ${completedJobs}/${jobsSent} (thr: inst ${instThr} - avg ${avgThr}) | remaining: ${
            jobsSent - completedJobs
          }                          \r`
        )
      );
    }
  }

  const total_duration_sec =
    (Date.now() - start - totalMonitoringOverhead) / 1000.0;
  const throughput = jobsSent / total_duration_sec;
  const avgLatency = total_duration_sec / jobsSent;

  console.log(`\n--- Results for ${numShards} shard(s) ---`);
  console.log(`jobs: ${jobsSent}`);
  console.log(`duration: ${total_duration_sec}s`);
  console.log(
    `monitoring overhead: ${(totalMonitoringOverhead / 1000).toFixed(2)}s`
  );
  console.log(`throughput: ${throughput.toFixed(2)} jobs/s`);
  console.log(`avg latency: ${(avgLatency * 1000).toFixed(2)}ms per job`);
  console.log(`completed jobs: ${completedJobs}`);

  if (
    !options.noVerify &&
    kind !== "noop" &&
    kind !== "nativets" &&
    !kind.startsWith("flow:") &&
    !kind.startsWith("script:")
  ) {
    await verifyOutputs(uuids, config.workspace_id);
  }

  return {
    numShards,
    throughput,
    totalDuration: total_duration_sec,
    jobsCompleted: completedJobs,
    avgLatency,
  };
}

export async function main({
  host,
  email,
  password,
  token,
  workspace,
  kind,
  jobs,
  maxShards,
  noVerify,
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  kind: string;
  jobs: number;
  maxShards: number;
  noVerify?: boolean;
}) {
  if (maxShards <= 0) {
    throw new Error("Max shards must be greater than 0");
  }

  console.log(
    "Started sharding benchmark with options",
    JSON.stringify(
      {
        host,
        email,
        workspace,
        kind,
        jobs,
        maxShards,
        noVerify,
      },
      null,
      4
    )
  );

  const options: ShardingBenchmarkOptions = {
    host,
    email,
    password,
    token,
    workspace,
    kind,
    jobs,
    maxShards,
    noVerify,
  };

  console.log("Initializing benchmark context...");
  const context = await initializeBenchmarkContext(options);
  console.log("Context initialized successfully!");

  const results: ShardingBenchmarkResult[] = [];

  for (let numShards = 1; numShards <= maxShards; numShards++) {
    try {
      const result = await runShardingBenchmark(context, options, numShards);
      results.push(result);

      await sleep(1);
    } catch (error) {
      console.error(
        `Failed to run benchmark with ${numShards} shard(s):`,
        error
      );
      break;
    }
  }

  console.log("\n" + "=".repeat(80));
  console.log("SHARDING BENCHMARK RESULTS SUMMARY");
  console.log("=".repeat(80));
  console.log(
    "Shards | Throughput (jobs/s) | Duration (s) | Avg Latency (ms) | Scaling Factor"
  );
  console.log("-".repeat(80));

  const baselineThroughput = results[0]?.throughput || 1;

  for (const result of results) {
    const scalingFactor = (result.throughput / baselineThroughput).toFixed(2);
    console.log(
      `${result.numShards.toString().padStart(6)} | ${result.throughput
        .toFixed(2)
        .padStart(18)} | ${result.totalDuration.toFixed(2).padStart(11)} | ${(
        result.avgLatency * 1000
      )
        .toFixed(2)
        .padStart(15)} | ${scalingFactor.padStart(13)}`
    );
  }

  console.log("-".repeat(80));

  if (results.length > 1) {
    const linearScalingEfficiency = results.map((result, index) => {
      const expectedThroughput = baselineThroughput * result.numShards;
      const efficiency = (result.throughput / expectedThroughput) * 100;
      return { shards: result.numShards, efficiency: efficiency.toFixed(1) };
    });

    console.log("\nLinear Scaling Efficiency:");
    for (const eff of linearScalingEfficiency) {
      console.log(`${eff.shards} shard(s): ${eff.efficiency}% efficiency`);
    }
  }

  console.log("\nBenchmark completed!");

  return {
    results,
    summary: {
      maxShards,
      bestThroughput: Math.max(...results.map((r) => r.throughput)),
      scalingEfficiency:
        results.length > 1
          ? (results[results.length - 1].throughput /
              (baselineThroughput * results[results.length - 1].numShards)) *
            100
          : 100,
    },
  };
}

if (import.meta.main) {
  await new Command()
    .name("wmillbench-sharding")
    .description(
      "Run Sharding Benchmark to measure linear scaling of windmill with multiple database shards."
    )
    .version(VERSION)
    .option("--host <url:string>", "The windmill host to benchmark.", {
      default: "http://127.0.0.1:8000",
    })
    .option("-e --email <email:string>", "The email to use to login.", {
      default: "admin@windmill.dev",
    })
    .option(
      "-p --password <password:string>",
      "The password to use to login.",
      {
        default: "changeme",
      }
    )
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
    .option(
      "--kind <kind:string>",
      "Specify the benchmark kind among: deno, identity, python, go, bash, dedicated, bun, noop, 2steps, nativets",
      {
        required: true,
      }
    )
    .option("-j --jobs <jobs:number>", "Number of jobs to create per test.", {
      default: 10000,
    })
    .env(
      "MAX_SHARDS=<shards:number>",
      "Maximum number of shards to test (will test 1, 2, 3, ... up to this number)"
    )
    .option(
      "--max-shards <shards:number>",
      "Maximum number of shards to test (will test 1, 2, 3, ... up to this number)",
      {
        required: true,
      }
    )
    .option("--no-verify", "Do not verify the output of the jobs.", {
      default: false,
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
        provider: new DenoLandProvider({ name: "wmillbench-sharding" }),
      })
    )
    .parse();
}
