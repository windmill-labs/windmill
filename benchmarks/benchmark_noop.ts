/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";
import * as windmill from "https://deno.land/x/windmill@v1.38.5/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";
export {
  DenoLandProvider,
  UpgradeCommand,
} from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

async function login(email: string, password: string): Promise<string> {
  return await windmill.UserService.login({
    requestBody: {
      email: email,
      password: password,
    },
  });
}

export const VERSION = "v1.167.0";

export async function main({
  host,
  email,
  password,
  token,
  workspace,
  jobs,
  batches,
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  jobs: number;
  batches: number;
}) {
  windmill.setClient("", host);

  console.log(
    "Started benchmark with NOOP jobs with options",
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

  console.log("Disabling workers before loading jobs");
  const disable_workers = await fetch(
    config.server + "/api/workers/toggle?disable=true",
    {
      method: "GET",
      headers: { ["Authorization"]: "Bearer " + config.token },
    }
  );
  if (!disable_workers.ok) {
    console.error(
      "Unable to disable workers. Is the Windmill server running in benchmark mode?"
    );
  }

  const jobsSent = jobs;
  const batch_num = batches;
  console.log(`Bulk creating ${jobsSent} jobs in ${batch_num} batches`);

  const start_create = Date.now();
  const all_create_operations = [];
  for (let i = 0; i < batch_num; i++) {
    all_create_operations.push(
      fetch(
        config.server +
          "/api/w/" +
          config.workspace_id +
          `/jobs/add_noop_jobs/${jobsSent / batch_num}`,
        {
          method: "POST",
          headers: { ["Authorization"]: "Bearer " + config.token },
        }
      )
    );
  }
  await Promise.all(all_create_operations);

  const end_create = Date.now();
  const create_duration = end_create - start_create;
  console.log(
    `Jobs successfully added to the queue in ${create_duration}s. Windmill will start pulling them\n`
  );
  const start = Date.now();

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
  }, 100);

  console.log("Enabling workers to start processing jobs");
  const enable_workers = await fetch(
    config.server + "/api/workers/toggle?disable=false",
    {
      method: "GET",
      headers: { ["Authorization"]: "Bearer " + config.token },
    }
  );
  if (!enable_workers.ok) {
    console.error(
      "Unable to disable workers. Is the Windmill server running in benchmark mode?"
    );
  }

  while (queue_length > 0) {
    await sleep(0.1);
  }

  clearInterval(updateState);

  const total_duration_sec = (Date.now() - start) / 1000.0;
  console.log(`jobs: ${jobsSent}`);
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
    .option("-j --jobs <jobs:number>", "Number of NOOP jobs to create.", {
      default: 10000,
    })
    .option(
      "-b --batches <batches:number>",
      "Number of batches to create all the jobs.",
      { default: 1 }
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
}
