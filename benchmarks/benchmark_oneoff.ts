/// <reference no-default-lib="true" />
/// <reference lib="deno.window" />

import { Command } from "https://deno.land/x/cliffy@v0.25.7/command/mod.ts";
import { UpgradeCommand } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/upgrade_command.ts";
import { DenoLandProvider } from "https://deno.land/x/cliffy@v0.25.7/command/upgrade/mod.ts";

import { sleep } from "https://deno.land/x/sleep@v1.2.1/mod.ts";

import * as windmill from "https://deno.land/x/windmill@v1.174.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.174.0/windmill-api/index.ts";

import { VERSION, createBenchScript, getFlowPayload, login } from "./lib.ts";

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
        console.log(`Job ${uuid} did not output the correct value: ${JSON.stringify(job)}`);
        incorrectResults++;
      }
    } catch (_) {
      console.log(`Job ${uuid} did not complete`);
      incorrectResults++;
    }
  }
  console.log(`Incorrect results: ${incorrectResults}`);
}

export const NON_TEST_TAGS = ["deno", "python", "go", "bash", "dedicated", "bun", "nativets", "dedicated_nativets", "flow"]
export async function main({
  host,
  email,
  password,
  token,
  workspace,
  kind,
  jobs,
  noVerify,
}: {
  host: string;
  email?: string;
  password?: string;
  token?: string;
  workspace: string;
  kind: string;
  jobs: number;
  noVerify?: boolean;
}) {
  windmill.setClient("", host);

  console.log(
    "Started benchmark with options",
    JSON.stringify(
      {
        host,
        email,
        workspace,
        kind,
        jobs,
        noVerify,
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

  async function getQueueCount(tags?: string[]) {
    return (
      await (
        await fetch(
          config.server + "/api/w/" + config.workspace_id + "/jobs/queue/count" + (tags && tags.length > 0 ? "?tags=" + tags.join(",") : ""),
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

  let pastJobs = 0;
  async function getCompletedJobsCount(tags?: string[]): Promise<number> {
    const completedJobs = (
      await (
        await fetch(
          host + "/api/w/" + config.workspace_id + "/jobs/completed/count" + (tags && tags.length > 0 ? "?tags=" + tags.join(",") : ""),
          { headers: { ["Authorization"]: "Bearer " + config.token } }
        )
      ).json()
    ).database_length;
    return completedJobs - pastJobs;
  }

  if (
    ["deno", "python", "go", "bash", "dedicated", "bun", "nativets", "dedicated_nativets"].includes(
      kind
    )
  ) {
    await createBenchScript(kind, workspace);
  }


  const jobsSent = jobs;
  console.log(`Bulk creating ${jobsSent} jobs`);

  const start_create = Date.now();
  let nStepsFlow = 0;
  let body: string;
  if (kind === "noop") {
    body = JSON.stringify({
      kind: "noop",
    });
  } else if (
    ["deno", "python", "go", "bash", "dedicated", "bun", "nativets", "dedicated_nativets"].includes(
      kind
    )
  ) {
    body = JSON.stringify({
      kind: "script",
      path: "f/benchmarks/" + kind,
    });
  } else if (["2steps", "bigscriptinflow"].includes(kind)) {
    nStepsFlow = kind == "2steps" ? 2 : 1;
    const payload = getFlowPayload(kind);
    body = JSON.stringify({
      kind: "flow",
      flow_value: payload.value,
    });
  } else if (kind.startsWith("flow:")) {
    console.log("Detected custom flow ");
    let flow_path = kind.substr(5);
    nStepsFlow = await getFlowStepCount(config.workspace_id, flow_path);
    console.log(`Total steps of flow including sub-flows: ${nStepsFlow}`);
    body = JSON.stringify({
      kind: "flow",
      path: flow_path,
    });
  } else if (kind.startsWith("script:")) {
    console.log("Detected custom script");
    body = JSON.stringify({
      kind: "script",
      path: kind.substr(7),
    });
  } else if (kind == "bigrawscript") {
    noVerify = true;
    body = JSON.stringify({
      kind: "rawscript",
      rawscript: {
        language: api.RawScript.language.BASH,
        content: "# let's bloat that bash script, 3.. 2.. 1.. BOOM\n".repeat(100) + "echo \"$WM_FLOW_JOB_ID\"\n",
      },
    });
  } else {
    throw new Error("Unknown script pattern " + kind);
  }

  let testOtherTag = false;
  if (testOtherTag) {
    const otherTagTodo = 2000000;

    let parsed = JSON.parse(body);
    parsed.tag = "test";
    let nbody = JSON.stringify(parsed);
    let response2 = await fetch(
      config.server +
      "/api/w/" +
      config.workspace_id +
      `/jobs/add_batch_jobs/${otherTagTodo}`,
      {
        method: "POST",
        headers: {
          ["Authorization"]: "Bearer " + config.token,
          "Content-Type": "application/json",
        },
        body: nbody,
      }
    );
    if (!response2.ok) {
      throw new Error(
        "Failed to create jobs: " +
        response2.statusText +
        " " +
        (await response2.text())
      );
    }
  }

  pastJobs = await getCompletedJobsCount(NON_TEST_TAGS);

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
    `Jobs successfully added to the queue in ${create_duration / 1000
    }s. Windmill will start pulling them\n`
  );
  let start = Date.now();

  let completedJobs = 0;
  let lastElapsed = 0;
  let lastCompletedJobs = 0;

  let didStart = false;
  while (completedJobs < jobsSent) {
    const loopStart = Date.now();
    if (!didStart) {
      const actual_queue = await getQueueCount(NON_TEST_TAGS);
      if (actual_queue < jobsSent) {
        start = Date.now();
        didStart = true;
      }
    } else {
      const elapsed = start ? Date.now() - start : 0;
      completedJobs = await getCompletedJobsCount(NON_TEST_TAGS);
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
          `elapsed: ${(elapsed / 1000).toFixed(
            2
          )} | jobs executed: ${completedJobs}/${jobsSent} (thr: inst ${instThr} - avg ${avgThr}) | remaining: ${jobsSent - completedJobs
          }                          \r`
        )
      );
    }
    const loopDuration = (Date.now() - loopStart) / 1000.0;
    if (loopDuration < 0.05) {
      await sleep(0.05 - loopDuration);
    }
  }

  const total_duration_sec = (Date.now() - start) / 1000.0;

  console.log(`\njobs: ${jobsSent}`);
  console.log(`duration: ${total_duration_sec}s`);
  console.log(`avg. throughput (jobs/time): ${jobsSent / total_duration_sec}`);

  console.log("completed jobs", completedJobs);
  console.log("queue length:", await getQueueCount(NON_TEST_TAGS));

  if (
    !noVerify &&
    kind !== "noop" &&
    kind !== "nativets" &&
    kind !== "dedicated_nativets" &&
    !kind.startsWith("flow:") &&
    !kind.startsWith("script:")
  ) {
    await verifyOutputs(uuids, config.workspace_id);
  }

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
      "Specifiy the benchmark kind among: deno, identity, python, go, bash, dedicated, bun, noop, 2steps, nativets, dedicated_nativets",
      {
        required: true,
      }
    )
    .option("-j --jobs <jobs:number>", "Number of jobs to create.", {
      default: 10000,
    })
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
        provider: new DenoLandProvider({ name: "wmillbench" }),
      })
    )
    .parse();
}
