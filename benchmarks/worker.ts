/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />
import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import * as windmill from "https://deno.land/x/windmill@v1.38.5/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.38.5/windmill-api/index.ts";
import { Job } from "https://deno.land/x/windmill@v1.38.5/windmill-api/index.ts";
import { Action, evaluate } from "./action.ts";

const promise = new Promise<{
  workspace_id: string;
  per_worker_throughput: number;
  useFlows: boolean;
  continous: boolean;
  max_per_worker: number;
  custom: Action | undefined;
  server: string;
  token: string;
}>((resolve, _reject) => {
  self.onmessage = (evt) => {
    const sharedConfig = evt.data;
    windmill.setClient(sharedConfig.token, sharedConfig.server);
    const config = {
      workspace_id: sharedConfig.workspace_id,
      per_worker_throughput: sharedConfig.per_worker_throughput,
      useFlows: sharedConfig.useFlows,
      continous: sharedConfig.continous,
      max_per_worker: sharedConfig.max_per_worker,
      custom: sharedConfig.custom,
      server: sharedConfig.server,
      token: sharedConfig.token,
    };
    self.name = "Worker " + sharedConfig.i;
    resolve(config);
    self.onmessage = null;
  };
});
const config = await promise;
const outstanding: string[] = [];
let cont = true;
let total_spawned = 0;
const start_time = Date.now();
let complete_timeout = Infinity;
self.onmessage = (evt) => {
  cont = false;
  complete_timeout = evt.data;
};

const updateStatusInterval = setInterval(() => {
  self.postMessage({ type: "jobs_sent", jobs_sent: total_spawned });
}, 100);

while (cont) {
  const queue_length = (await (await fetch(
    config.server + "/api/w/" + config.workspace_id + "/jobs/queue/count",
    { headers: { ["Authorization"]: "Bearer " + config.token } },
  )).json()).database_length;
  if (queue_length > 2500) {
    console.log(
      `queue length: ${queue_length} > 2500. waiting...                                                            `,
    );
    await sleep(0.5);
    continue;
  }
  if (
    (total_spawned * 1000) / (Date.now() - start_time) >
      config.per_worker_throughput
  ) {
    console.log("at maximum throughput. waiting...");
    await sleep(0.1);
    continue;
  }
  total_spawned++;
  if (total_spawned > config.max_per_worker) {
    break;
  }
  let uuid: string;
  if (config.custom) {
    await evaluate(config.custom);
    continue;
  } else if (config.useFlows) {
    uuid = await windmill.JobService.runFlowPreview({
      workspace: config.workspace_id,
      requestBody: {
        args: {},
        value: {
          modules: [
            {
              input_transforms: {},
              value: {
                language: api.RawScript.language.DENO,
                type: "rawscript",
                content:
                  'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
              },
            },
            {
              input_transforms: {},
              value: {
                language: api.RawScript.language.DENO,
                type: "rawscript",
                content:
                  'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
              },
            },
          ],
        },
      },
    });
  } else {
    uuid = await windmill.JobService.runScriptPreview({
      workspace: config.workspace_id,
      requestBody: {
        language: api.Preview.language.DENO,
        content: 'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
        args: {},
      },
    });
  }
  if (!config.continous) outstanding.push(uuid);
}

clearInterval(updateStatusInterval);

const end_time = Date.now() + complete_timeout;
let incorrect_results = 0;
const enc = (s: string) => new TextEncoder().encode(s);

while (outstanding.length > 0 && Date.now() < end_time) {
  const uuid = outstanding.shift()!;

  let r: Job;
  try {
    r = await windmill.JobService.getJob({
      workspace: config.workspace_id,
      id: uuid,
    });
  } catch {
    console.log("job not found: " + uuid);
    continue;
  }
  if (r.type == "QueuedJob") {
    outstanding.push(uuid);
    await Deno.stdout.write(
      enc(
        `uuid: ${uuid}, queue length: ${
          (await (await fetch(
            config.server + "/api/w/" + config.workspace_id +
              "/jobs/queue/count",
            { headers: { ["Authorization"]: "Bearer " + config.token } },
          )).json()).database_length
        }                                                                                   \r`,
      ),
    );
  } else if (!config.useFlows) {
    r = r as api.CompletedJob;
    try {
      if (r.result != uuid) {
        console.log(
          "job did not return correct UUID: " +
            r.result +
            " != " +
            uuid +
            "job: \n" +
            JSON.stringify(r, null, 2),
        );
        incorrect_results++;
      }
    } catch (e) {
      console.log("error during wait: ", e);
      outstanding.push(uuid);
    }
  }
}

self.postMessage({
  type: "zombie_jobs",
  zombie_jobs: outstanding.length,
  incorrect_results,
});
