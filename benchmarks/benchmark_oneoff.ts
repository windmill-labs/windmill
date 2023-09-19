/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";

import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";

import { VERSION, createBenchScript, getFlowPayload, login } from "./lib.ts";

export async function main({
  host,
  email,
  password,
  token,
  workspace,
  kind,
  jobs,
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  kind: string;
  jobs: number;
}) {
  windmill.setClient("", host);

  console.log(
    "Started benchmark with options",
    JSON.stringify(
      {
        host,
        email,
        workspace,
      },
      null,
      4
    )
  );

  const config = {
    token: "",
    server: host,
    workspace_id: workspace,
  };

  let final_token: string;
  if (!token) {
    if (email && password) {
      final_token = await login(email, password);
    } else {
      console.error("Token or email with password are required.");
      return;
    }
  } else {
    final_token = token;
  }

  config.token = final_token;
  windmill.setClient(final_token, host);
  const enc = (s: string) => new TextEncoder().encode(s);

  if (["deno", "python", "go", "bash", "dedicated", "bun"].includes(kind)) {
    await createBenchScript(kind, workspace);
  }

  let jobsSent = jobs;
  console.log(`Bulk creating ${jobsSent} jobs`);

  const start_create = Date.now();
  let response: Response;
  if (kind === "noop") {
    response = await fetch(
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
        body: JSON.stringify({
          kind: "noop",
        }),
      }
    );
  } else if (
    ["deno", "python", "go", "bash", "dedicated", "bun"].includes(kind)
  ) {
    response = await fetch(
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
        body: JSON.stringify({
          kind: "script",
          path: "f/benchmarks/" + kind,
          dedicated_worker: kind === "dedicated",
        }),
      }
    );
  } else if (["2steps", "onebranch", "branchallparrallel"].includes(kind)) {
    const payload = getFlowPayload(kind);
    response = await fetch(
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
        body: JSON.stringify({
          kind: "flow",
          path: "f/benchmarks/" + kind,
          flow_value: payload.value,
        }),
      }
    );
  } else {
    throw new Error("Unknown script pattern " + kind);
  }

  if (!response.ok) {
    throw new Error("Failed to create jobs: " + response.statusText);
  }
  const end_create = Date.now();
  const create_duration = end_create - start_create;
  console.log(
    `Jobs successfully added to the queue in ${
      create_duration / 1000
    }s. Windmill will start pulling them\n`
  );
  let start = Date.now();

  let queue_length = jobsSent;
  let lastElapsed = 0;
  let lastQueueLength = queue_length;
  const updateState = setInterval(async () => {
    const elapsed = start ? Date.now() - start : 0;
    queue_length = (
      await (
        await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length;
    const avgThr = (((jobsSent - queue_length) / elapsed) * 1000).toFixed(2);
    const instThr =
      lastElapsed > 0
        ? (
            ((lastQueueLength - queue_length) / (elapsed - lastElapsed)) *
            1000
          ).toFixed(2)
        : 0;

    lastElapsed = elapsed;
    lastQueueLength = queue_length;

    await Deno.stdout.write(
      enc(
        `elapsed: ${(elapsed / 1000).toFixed(2)} | jobs executed: ${
          jobsSent - queue_length
        }/${jobsSent} (thr: inst ${instThr} - avg ${avgThr}) | queue: ${queue_length}                          \r`
      )
    );
  }, 10);

  while (queue_length > 0) {
    if (queue_length < jobsSent && jobsSent === jobs) {
      // reset start time to when the first job was picked up
      start = Date.now();
      jobsSent = queue_length;
    }
    await sleep(0.01);
  }

  clearInterval(updateState);

  const total_duration_sec = (Date.now() - start) / 1000.0;

  await sleep(0.1);
  console.log(`\njobs: ${jobsSent}`);
  console.log(`duration: ${total_duration_sec}s`);
  console.log(`avg. throughput (jobs/time): ${jobsSent / total_duration_sec}`);

  console.log(
    "queue length:",
    (
      await (
        await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/queue/count",
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length
  );
  console.log("done");

  return {
    throughput: jobsSent / total_duration_sec,
  };
}

if (import.meta.main) {
  await new Command()
    .name("wmillbench")
    .description("Run Benchmark to measure throughput of windmill.")
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
      "Specifiy the benchmark kind among: deno, identity, python, go, bash, dedicated, bun, noop, 2steps, onebranch, branchallparrallel",
      {
        required: true,
      }
    )
    .option("-j --jobs <jobs:number>", "Number of jobs to create.", {
      default: 10000,
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
}
