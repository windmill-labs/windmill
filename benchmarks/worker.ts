/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />
import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";

const promise = new Promise<api.Configuration & { workspace_id: string }>(
  (resolve, _reject) => {
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
      };
      self.name = "Worker " + sharedConfig.i;
      resolve(config);
      self.onmessage = null;
    };
  }
);
const config = await promise;
const jobApi = new windmill.JobApi(config);
while (true) {
  if ((await jobApi.listQueue(config.workspace_id)).length > 500) {
    console.log("queue very long. waiting...");
    await sleep(0.5);
    continue;
  }
  await jobApi.runScriptPreview(config.workspace_id, {
    language: "deno",
    content: "export function main(n: number){ return n; }",
    args: { n: 1 },
  });
}
