/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />
import { resolve } from "https://deno.land/std@0.141.0/path/win32.ts";
import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import * as windmill from "https://deno.land/x/windmill@v1.37.0/mod.ts";
import * as api from "https://deno.land/x/windmill@v1.37.0/windmill-api/index.ts";

const promise = new Promise<api.Configuration & { workspace_id: string }>(
  (resolve, _reject) => {
    self.onmessage = (evt) => {
      const shared_config = evt.data;
      const config = {
        ...api.createConfiguration({
          baseServer: new api.ServerConfiguration(shared_config.server, {}),
          authMethods: {
            bearerAuth: {
              tokenProvider: {
                getToken() {
                  return shared_config.token;
                },
              },
            },
          },
        }),
        workspace_id: shared_config.workspace_id,
      };
      self.name = "Worker " + shared_config.i;
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
    await sleep(500);
    continue;
  }
  await jobApi.runScriptPreview(config.workspace_id, {
    language: "deno",
    content: "export function main(n: number){ return n; }",
    args: { n: 1 },
  });
}
