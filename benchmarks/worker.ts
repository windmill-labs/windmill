/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />
import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";

const promise = new Promise<
  api.Configuration & { workspace_id: string; per_worker_throughput: number }
>((resolve, _reject) => {
  self.onmessage = (evt) => {
    const sharedConfig = evt.data;
    const config = {
      ...api.createConfiguration({
        baseServer: new api.ServerConfiguration(sharedConfig.server, {}),
        authMethods: {
          bearerAuth: {
            tokenProvider: {
              getToken() {
                return sharedConfig.token;
              },
            },
          },
        },
      }),
      workspace_id: sharedConfig.workspace_id,
      per_worker_throughput: sharedConfig.per_worker_throughput,
    };
    self.name = "Worker " + sharedConfig.i;
    resolve(config);
    self.onmessage = null;
  };
});
const config = await promise;
const jobApi = new windmill.JobApi(config);
const outstanding: string[] = [];
let cont = true;
let total_spawned = 0;
const start_time = Date.now();

self.onmessage = (_evt) => {
  cont = false;
};

while (cont) {
  if ((await jobApi.listQueue(config.workspace_id)).length > 500) {
    console.log("queue very long. waiting...");
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
  const uuid = await jobApi.runScriptPreview(config.workspace_id, {
    language: "deno",
    content: 'export function main(){ return Deno.env.get("WM_JOB_ID"); }',
    args: {},
  });
  outstanding.push(uuid);
  total_spawned++;
}

while (outstanding.length > 0) {
  const uuid = outstanding.shift()!;
  const r = await jobApi.getJob(config.workspace_id, uuid);
  if (r.running) {
    outstanding.push(uuid);
    continue;
  } else {
    try {
      let result: string;
      if (r.result) {
        result = r.result;
      } else {
        const j = await jobApi.getCompletedJob(config.workspace_id, uuid);
        result = j.result;
      }
      if (result != uuid) {
        console.log("job did not return correct UUID: " + result);
      }
    } catch (e) {
      console.log("error during wait: ", e);
      outstanding.push(uuid);
      continue;
    }
  }
}

self.postMessage("done");
