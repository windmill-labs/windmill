/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />
import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import * as windmill from "https://deno.land/x/windmill@v1.121.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.121.0/windmill-api/index.ts";
import { Job } from "https://deno.land/x/windmill@v1.121.0/windmill-api/index.ts";
import { Action, evaluate } from "./action.ts";

const promise = new Promise<{
  workspace_id: string;
  per_worker_throughput: number;
  useFlows: boolean;
  flowPattern: string;
  scriptPattern: string;
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
      flowPattern: sharedConfig.flowPattern,
      scriptPattern: sharedConfig.scriptPattern,
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
  const queue_length = (
    await (
      await fetch(
        config.server + "/api/w/" + config.workspace_id + "/jobs/queue/count",
        { headers: { ["Authorization"]: "Bearer " + config.token } }
      )
    ).json()
  ).database_length;
  if (queue_length > 2500) {
    console.log(
      `queue length: ${queue_length} > 2500. waiting...                                                            `
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
    let payload: api.FlowPreview;
    if (config.flowPattern == "branchone") {
      payload = {
        path: "branchone",
        args: {},
        value: {
          modules: [
            {
              id: "a",
              value: {
                input_transforms: {},
                language: api.RawScript.language.DENO,
                type: "rawscript",
                content:
                  'export function main(){ return Deno.env.get("WM_FLOW_JOB_ID"); }',
              },
            },
            {
              id: "b",
              value: {
                type: "branchone",
                branches: [],
                default: [
                  {
                    id: "c",
                    value: {
                      input_transforms: {
                        x: {
                          type: "javascript",
                          expr: "results.a",
                        },
                      },
                      language: api.RawScript.language.DENO,
                      type: "rawscript",
                      content: "export function main(x: string){ return x; }",
                    },
                  },
                ],
              },
            },
          ],
        },
      };
    } else {
      payload = {
        path: "2steps",
        args: {},
        value: {
          modules: [
            {
              id: "a",
              value: {
                input_transforms: {},
                language: api.RawScript.language.DENO,
                type: "rawscript",
                content:
                  'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
              },
            },
            {
              id: "b",
              value: {
                input_transforms: {},
                language: api.RawScript.language.DENO,
                type: "rawscript",
                content:
                  'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
              },
            },
          ],
        },
      };
    }
    uuid = await windmill.JobService.runFlowPreview({
      workspace: config.workspace_id,
      requestBody: payload,
    });
  } else {
    let payload: api.Preview;
    if (config.scriptPattern == "httpversion") {
      payload = {
        path: "httpversion",
        kind: "http",
        args: {
          url: "http://localhost:8000/api/version",
        },
      };
    } else if (config.scriptPattern == "identity") {
      payload = {
        path: "identity",
        kind: "identity",
        args: {
          identity: "itsme",
        },
      };
    } else {
      payload = {
        path: "denosimple",
        language: api.Preview.language.DENO,
        content: 'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
        args: {},
      };
    }
    uuid = await windmill.JobService.runScriptPreview({
      workspace: config.workspace_id,
      requestBody: payload,
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
  } catch (e) {
    console.log("job not found: " + uuid + " " + e.message);
    continue;
  }
  if (r.type == "QueuedJob") {
    outstanding.push(uuid);
    await Deno.stdout.write(
      enc(
        `uuid: ${uuid}, queue length: ${
          (
            await (
              await fetch(
                config.server +
                  "/api/w/" +
                  config.workspace_id +
                  "/jobs/queue/count",
                { headers: { ["Authorization"]: "Bearer " + config.token } }
              )
            ).json()
          ).database_length
        }                                                                                   \r`
      )
    );
  } else {
    r = r as api.CompletedJob;
    try {
      if (
        config.scriptPattern != "httpversion" &&
        config.scriptPattern != "identity" &&
        r.result != uuid
      ) {
        console.log(
          "job did not return correct UUID: " +
            r.result +
            " != " +
            uuid +
            "job: \n" +
            JSON.stringify(r, null, 2)
        );
        incorrect_results++;
      } else {
        // console.log(r.result);
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
